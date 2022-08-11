use std::iter;

use swc_common::{util::take::Take, DUMMY_SP};
use swc_common::{BytePos, Span, SyntaxContext};
use swc_ecma_visit::VisitMut;
use swc_plugin::ast::*;
use swc_plugin::utils::{prepend_stmts, quote_ident};

use crate::class_like::ClassLike;
use crate::js_util::{ref_expr, this_expr};
use crate::prepend::prepend;
use crate::span::{concat_span, get_prop_name_span};
use crate::virtual_machine::VirtualMachine;

/**
 * Name of the `register` function that is injected into all compiled source files.
 *
 * ```ts
 * function register_8269d1a8(func, ast) {
 *   func[Symbol.for("functionless:AST")] = ast;
 *   return func;
 * }
 * ```
 *
 * All Function Declarations, Expressions and Arrow Expressions are decorated with
 * the `register` function which attaches its AST as a property.
 */
const REGISTER_FUNCTION_NAME: &str = "register_8269d1a8";

/**
 * Name of the `bind` function that is injected into all compiled source files.
 *
 * ```ts
 * function bind_8269d1a8(func, self, ...args) {
 *   const tmp = func.bind(self, ...args);
 *   if (typeof func === "function") {
 *     func[Symbol.for("functionless:BoundThis")] = self;
 *     func[Symbol.for("functionless:BoundArgs")] = args;
 *     func[Symbol.for("functionless:TargetFunction")] = func;
 *   }
 *   return tmp;
 * }
 * ```
 *
 * All CallExpressions with the shape <expr>.bind(...<args>) are re-written as calls
 * to this special function which intercepts the call.
 * ```ts
 * <expr>.bind(...<args>)
 * // =>
 * bind_8269d1a8(<expr>, ...<args>)
 * ```
 *
 * If `<expr>` is a Function, then the values of BoundThis, BoundArgs and TargetFunction
 * are added to the bound Function.
 *
 * If `<expr>` is not a Function, then the call is proxied without modification.
 */
const BIND_FUNCTION_NAME: &str = "bind_8269d1a8";

pub struct ClosureDecorator {
  /**
   * A Virtual Machine managing lexical scope as we walk the tree.
   */
  pub vm: VirtualMachine,
  /**
   * An Identifier referencing the `register` interceptor function injected into all processed modules.
   */
  pub register: Ident,
  /**
   * An Identifier referencing the `bind` interceptor function injected into all processed modules.
   */
  pub bind: Ident,
}

impl ClosureDecorator {
  pub fn new() -> ClosureDecorator {
    ClosureDecorator {
      vm: VirtualMachine::new(),
      register: module_ident(REGISTER_FUNCTION_NAME),
      bind: module_ident(BIND_FUNCTION_NAME),
    }
  }
}

/**
 * Creates an [Identifier](Ident) that points to a value declared in the top-level of a module.
 *
 * This means it has [SyntaxContext](SyntaxContext) of `0`.
 */
fn module_ident(name: &str) -> Ident {
  Ident {
    span: Span {
      ctxt: SyntaxContext::from_u32(0),
      hi: BytePos(0),
      lo: BytePos(0),
    },
    sym: JsWord::from(name),
    optional: false,
  }
}

impl VisitMut for ClosureDecorator {
  fn visit_mut_module_items(&mut self, items: &mut Vec<ModuleItem>) {
    let new_stmts: Vec<ModuleItem> = [
      ModuleItem::Stmt(Stmt::Decl(create_register_function())),
      ModuleItem::Stmt(Stmt::Decl(create_bind_function())),
    ]
    .into_iter()
    .chain(
      // discover all function declarations in the module and create a
      // CallExpr to `register` that decorates each function with its AST.
      items
        .iter()
        .filter_map(|item| match item {
          ModuleItem::ModuleDecl(decl) => match decl {
            ModuleDecl::ExportDecl(ex) => match &ex.decl {
              // TODO: handle classes
              Decl::Class(_) => None,
              Decl::Fn(func) => Some(self.register_func_decl(func)),
              _ => None,
            },
            _ => None,
          },
          ModuleItem::Stmt(stmt) => self.register_stmt_if_func_decl(stmt),
        })
        .map(|stmt| ModuleItem::Stmt(stmt)),
    )
    .collect();

    // recursively visit and transform each of the statements in the module
    items.iter_mut().for_each(|stmt| stmt.visit_mut_with(self));

    // prepend the `register` and `bind` function declarations
    // and the `register` calls into the top of every module.
    prepend_stmts(items, new_stmts.into_iter());
  }

  fn visit_mut_stmts(&mut self, stmts: &mut Vec<Stmt>) {
    // discover all function declarations in the block and create a
    // CallExpr to `register` that decorates each function with its AST.
    let register_stmts: Vec<Stmt> = stmts
      .iter()
      .filter_map(|stmt| self.register_stmt_if_func_decl(stmt))
      .collect();

    // recursively visit and transform each of the statements in the block
    stmts.visit_mut_children_with(self);

    if register_stmts.len() > 0 {
      // only bother doing the work of prepend if there are statements to prepend
      prepend_stmts(stmts, register_stmts.into_iter());
    }
  }

  fn visit_mut_class_decl(&mut self, class_decl: &mut ClassDecl) {
    self.visit_mut_class_like(class_decl);
  }

  fn visit_mut_class_expr(&mut self, class_expr: &mut ClassExpr) {
    self.visit_mut_class_like(class_expr);
  }

  fn visit_mut_call_expr(&mut self, call: &mut CallExpr) {
    // detect if this looks like a call to Function.bind
    // e.g. `foo.bind(self)`
    // in general: `<expr>.bind(...<args>)`
    let maybe_bind_expr = match &mut call.callee {
      Callee::Expr(expr) => match expr.as_mut() {
        Expr::Member(member) => match &member.prop {
          MemberProp::Ident(ident) if &ident.sym == "bind" => Some(member.obj.as_mut()),
          _ => None,
        },
        _ => None,
      },
      _ => None,
    };

    if maybe_bind_expr.is_some() {
      let expr = maybe_bind_expr.unwrap();

      let args = iter::once(ExprOrSpread {
        expr: Box::new(expr.take()),
        spread: None,
      })
      .chain(call.args.iter_mut().map(|arg| ExprOrSpread {
        spread: None,
        expr: arg.expr.take(),
      }))
      .collect();

      // replace the CallExpr with a call to the `bind` interceptor function
      // foo.bind(bar)
      // =>
      // bind(foo, bar);
      *call = CallExpr {
        callee: Callee::Expr(Box::new(Expr::Ident(self.bind.clone()))),
        span: call.span.clone(),
        type_args: None,
        args,
      }
    }

    call.visit_mut_children_with(self);
  }

  fn visit_mut_prop(&mut self, prop: &mut Prop) {
    match prop {
      // { method() { } }
      Prop::Method(method) => {
        let ast = self.parse_method_like(
          method,
          None,
          &concat_span(get_prop_name_span(&method.key), &method.function.span),
        );

        method.visit_mut_children_with(self);

        // re-write to
        // { method: function method() { }}
        let func = Box::new(Expr::Fn(FnExpr {
          ident: match &method.key {
            PropName::Ident(id) => Some(id.clone()),
            _ => None,
          },
          function: method.function.take(),
        }));

        *prop = Prop::KeyValue(KeyValueProp {
          key: method.key.take(),
          value: self.register_ast(func, ast),
        });
      }
      _ => {
        prop.visit_mut_children_with(self);
      }
    };
  }

  fn visit_mut_expr(&mut self, expr: &mut Expr) {
    match expr {
      Expr::Arrow(arrow) => {
        let ast = self.parse_arrow(arrow, true);

        arrow.visit_mut_children_with(self);

        // replace the arrow function with a call to the `register` interceptor function
        // that decorates the function with its AST
        // () => {..}
        // =>
        // register(
        //   () => {..}, // original arrow
        //   () => [..], // function that produces the arrow's AST
        // )
        *expr = *self.register_mut_ast(&mut Expr::Arrow(arrow.take()), ast);
      }
      Expr::Fn(func) if func.function.body.is_some() => {
        let ast = self.parse_function_expr(&func, true);

        func.visit_mut_children_with(self);

        // replace the function expression with a call to the `register` interceptor function
        // that decorates the function with its AST
        // function foo() {..}
        // =>
        // register(
        //   function foo() {..}, // original function expression
        //   () => [..], // function that produces the function expression's AST
        // )
        *expr = *self.register_mut_ast(&mut Expr::Fn(func.take()), ast);
      }
      _ => {
        expr.visit_mut_children_with(self);
      }
    };
  }
}

impl ClosureDecorator {
  /**
   * Generic visitor for ClassDecl and ClassExpr.
   *
   * 1. The static Class object is decorated with an AST describing the entire contents of the class.
   * 2. Each Method, Getter and Setter within the class are decorated with their own AST.
   * 3. For Getters and Setters, the PropertyDescriptor.{get|set} function is decorated.
   * 4. All register calls are injected as a ClassStaticBlock at the very top of the class.
   */
  fn visit_mut_class_like<T>(&mut self, class_like: &mut T)
  where
    T: ClassLike,
  {
    let class_ast = self.parse_class_like(class_like, true);

    // class Foo {
    //  static {
    //    register(this);
    //  }
    // }
    let class_ref = Box::new(Expr::This(ThisExpr { span: DUMMY_SP }));

    let register_class_stmt = self.register_ast_stmt(class_ref, class_ast);

    let register_stmts: Vec<Stmt> = class_like
      .class()
      .body
      .iter()
      .filter_map(|member| match member {
        ClassMember::Method(method) => Some(self.register_class_method(method)),
        // ClassMember::PrivateMethod(method) => Some(self.register_class_method(method, class_name)),
        _ => None,
      })
      .chain(iter::once(register_class_stmt))
      .collect();

    class_like.class_mut().visit_mut_children_with(self);

    if register_stmts.len() > 0 {
      prepend(
        &mut class_like.class_mut().body,
        ClassMember::StaticBlock(StaticBlock {
          span: DUMMY_SP,
          body: BlockStmt {
            span: DUMMY_SP,
            stmts: register_stmts,
          },
        }),
      );
    }
  }

  fn register_class_method(&mut self, method: &ClassMethod) -> Stmt {
    let method_ast = self.parse_method_like(method, Some(ref_expr(this_expr())), &method.span);

    let this = Box::new(if method.is_static {
      // `this` if it is static
      Expr::This(ThisExpr { span: DUMMY_SP })
    } else {
      // `this.prototype` if the method is on the prototype
      Expr::Member(MemberExpr {
        obj: Box::new(Expr::This(ThisExpr { span: DUMMY_SP })),
        prop: MemberProp::Ident(quote_ident!("prototype")),
        span: DUMMY_SP,
      })
    });

    let method_name = match &method.key {
      PropName::Ident(id) => MemberProp::Ident(id.clone()),
      PropName::Computed(expr) => MemberProp::Computed(expr.clone()),
      prop => MemberProp::Computed(ComputedPropName {
        span: DUMMY_SP,
        expr: Box::new(Expr::Lit(match prop {
          PropName::BigInt(x) => Lit::BigInt(x.clone()),
          PropName::Num(x) => Lit::Num(x.clone()),
          PropName::Str(x) => Lit::Str(x.clone()),
          _ => panic!(""),
        })),
      }),
    };

    fn get_property_descriptor(this: Box<Expr>, prop: &PropName) -> Box<Expr> {
      Box::new(Expr::Call(CallExpr {
        span: DUMMY_SP,
        type_args: None,
        // Object.getOwnPropertyDescriptor
        callee: Callee::Expr(Box::new(Expr::Member(MemberExpr {
          obj: Box::new(Expr::Ident(Ident {
            optional: false,
            span: DUMMY_SP,
            sym: JsWord::from("Object"),
          })),
          span: DUMMY_SP,
          prop: MemberProp::Ident(Ident {
            optional: false,
            span: DUMMY_SP,
            sym: JsWord::from("getOwnPropertyDescriptor"),
          }),
        }))),
        args: vec![
          ExprOrSpread {
            expr: this,
            spread: None,
          },
          ExprOrSpread {
            expr: match prop {
              PropName::Ident(id) => Box::new(Expr::Lit(Lit::Str(Str {
                raw: None,
                span: DUMMY_SP,
                value: id.sym.clone(),
              }))),
              PropName::Str(_) => todo!(),
              PropName::Num(_) => todo!(),
              PropName::Computed(comp) => comp.expr.clone(),
              PropName::BigInt(_) => todo!(),
            },
            spread: None,
          },
        ],
      }))
    }

    // class Foo {
    //   static {
    //     register(this.prototype.method, ..)
    //              ^-------------------^
    //   }
    //   method() { }
    // }
    let method_ref = match method.kind {
      // this.prototype.method
      MethodKind::Method => Box::new(Expr::Member(MemberExpr {
        obj: this,
        span: DUMMY_SP,
        prop: method_name,
      })),
      // Object.getOwnPropertyDescriptor(this.prototype, "m").get
      MethodKind::Getter => Box::new(Expr::Member(MemberExpr {
        obj: get_property_descriptor(this, &method.key),
        prop: MemberProp::Ident(quote_ident!("get")),
        span: DUMMY_SP,
      })),
      // Object.getOwnPropertyDescriptor(this.prototype, "m").set
      MethodKind::Setter => Box::new(Expr::Member(MemberExpr {
        obj: get_property_descriptor(this, &method.key),
        prop: MemberProp::Ident(quote_ident!("set")),
        span: DUMMY_SP,
      })),
    };

    self.register_ast_stmt(method_ref, method_ast)
  }

  fn register_stmt_if_func_decl(&mut self, stmt: &Stmt) -> Option<Stmt> {
    match stmt {
      Stmt::Decl(Decl::Fn(func)) => Some(self.register_func_decl(func)),
      _ => None,
    }
  }

  fn register_func_decl(&mut self, func: &FnDecl) -> Stmt {
    let parse_func = self.parse_function_decl(&func, true);
    self.register_ast_stmt(Box::new(Expr::Ident(func.ident.clone())), parse_func)
  }

  fn register_ast_stmt(&self, expr: Box<Expr>, ast: Box<Expr>) -> Stmt {
    Stmt::Expr(ExprStmt {
      expr: self.register_ast(expr, ast),
      span: DUMMY_SP,
    })
  }

  fn register_mut_ast(&self, func: &mut Expr, ast: Box<Expr>) -> Box<Expr> {
    self.register_ast(Box::new(func.take()), ast)
  }

  fn register_ast(&self, func: Box<Expr>, ast: Box<Expr>) -> Box<Expr> {
    Box::new(Expr::Call(CallExpr {
      span: DUMMY_SP,
      type_args: None,
      // register(() =>  { .. }, () => new FunctionDecl([..]))
      callee: Callee::Expr(Box::new(Expr::Ident(self.register.clone()))),
      args: vec![
        ExprOrSpread {
          expr: func,
          spread: None,
        },
        ExprOrSpread {
          expr: Box::new(Expr::Arrow(ArrowExpr {
            is_async: false,
            is_generator: false,
            return_type: None,
            params: vec![],
            span: DUMMY_SP,
            type_params: None,
            body: BlockStmtOrExpr::Expr(ast),
          })),
          spread: None,
        },
      ],
    }))
  }
}

// function register(func, ast) {
//   func[Symbol.for("functionless:ast")] = ast;
//   return func;
// }
fn create_register_function() -> Decl {
  let func = quote_ident!("func");
  let ast = quote_ident!("ast");
  Decl::Fn(FnDecl {
    declare: false,
    ident: Ident {
      span: DUMMY_SP,
      sym: JsWord::from(REGISTER_FUNCTION_NAME),
      optional: false,
    },
    function: Function {
      params: vec![param(func.clone(), false), param(ast.clone(), false)],
      decorators: vec![],
      span: DUMMY_SP,
      body: Some(BlockStmt {
        span: DUMMY_SP,
        stmts: vec![
          set_symbol(func.clone(), "functionless:AST", ast.clone()),
          Stmt::Return(ReturnStmt {
            span: DUMMY_SP,
            arg: Some(Box::new(Expr::Ident(func.clone()))),
          }),
        ],
      }),
      is_generator: false,
      is_async: false,
      type_params: None,
      return_type: None,
    },
  })
}

// function bind(func, self, ...args) {
//   const f = func.bind(self, ...args);
//   f[Symbol.for("functionless:BoundThis")] = self;
//   f[Symbol.for("functionless:BoundArgs")] = args;
//   f[Symbol.for("functionless:TargetFunction")] = func;
//   return func.bind(self, ...args);
//}
fn create_bind_function() -> Decl {
  let func = quote_ident!("func");
  let this = quote_ident!("self");
  let args = quote_ident!("args");
  let f = quote_ident!("f");
  Decl::Fn(FnDecl {
    declare: false,
    ident: Ident {
      span: DUMMY_SP,
      sym: JsWord::from(BIND_FUNCTION_NAME),
      optional: false,
    },
    function: Function {
      params: vec![
        param(func.clone(), false),
        param(this.clone(), false),
        param(args.clone(), true),
      ],
      decorators: vec![],
      span: DUMMY_SP,
      body: Some(BlockStmt {
        span: DUMMY_SP,
        stmts: vec![
          // const f = func.bind(self, ...args);
          Stmt::Decl(Decl::Var(VarDecl {
            declare: false,
            kind: VarDeclKind::Const,
            decls: vec![VarDeclarator {
              definite: false,
              span: DUMMY_SP,
              name: Pat::Ident(BindingIdent {
                id: f.clone(),
                type_ann: None,
              }),
              init: Some(Box::new(Expr::Call(CallExpr {
                span: DUMMY_SP,
                type_args: None,
                callee: Callee::Expr(Box::new(Expr::Member(MemberExpr {
                  obj: Box::new(Expr::Ident(func.clone())),
                  prop: MemberProp::Ident(quote_ident!("bind")),
                  span: DUMMY_SP,
                }))),
                args: vec![
                  ExprOrSpread {
                    expr: Box::new(Expr::Ident(this.clone())),
                    spread: None,
                  },
                  ExprOrSpread {
                    expr: Box::new(Expr::Ident(args.clone())),
                    spread: Some(DUMMY_SP),
                  },
                ],
              }))),
            }],
            span: DUMMY_SP,
          })),
          // if(typeof func === "function")
          Stmt::If(IfStmt {
            span: DUMMY_SP,
            test: Box::new(Expr::Bin(BinExpr {
              span: DUMMY_SP,
              left: Box::new(Expr::Unary(UnaryExpr {
                arg: Box::new(Expr::Ident(func.clone())),
                span: DUMMY_SP,
                op: UnaryOp::TypeOf,
              })),
              op: BinaryOp::EqEqEq,
              right: Box::new(Expr::Lit(Lit::Str(Str {
                raw: None,
                span: DUMMY_SP,
                value: JsWord::from("function"),
              }))),
            })),
            alt: None,
            cons: Box::new(Stmt::Block(BlockStmt {
              span: DUMMY_SP,
              stmts: vec![
                // f[Symbol.for("functionless:BoundThis")] = self;
                // f[Symbol.for("functionless:BoundArgs")] = args;
                // f[Symbol.for("functionless:TargetFunction")] = func;
                set_symbol(f.clone(), "functionless:BoundThis", this.clone()),
                set_symbol(f.clone(), "functionless:BoundArgs", args.clone()),
                set_symbol(f.clone(), "functionless:TargetFunction", func.clone()),
              ],
            })),
          }),
          Stmt::Return(ReturnStmt {
            span: DUMMY_SP,
            arg: Some(Box::new(Expr::Ident(f.clone()))),
          }),
        ],
      }),
      is_generator: false,
      is_async: false,
      type_params: None,
      return_type: None,
    },
  })
}

fn param(id: Ident, rest: bool) -> Param {
  Param {
    decorators: vec![],
    span: DUMMY_SP,
    pat: if rest {
      Pat::Rest(RestPat {
        arg: Box::new(Pat::Ident(BindingIdent { id, type_ann: None })),
        dot3_token: DUMMY_SP,
        span: DUMMY_SP,
        type_ann: None,
      })
    } else {
      Pat::Ident(BindingIdent { id, type_ann: None })
    },
  }
}

fn set_symbol(on: Ident, sym: &str, to: Ident) -> Stmt {
  Stmt::Expr(ExprStmt {
    span: DUMMY_SP,
    expr: Box::new(Expr::Assign(AssignExpr {
      // func[Symbol.for("functionless:AST")] = ast
      left: PatOrExpr::Expr(Box::new(Expr::Member(MemberExpr {
        obj: Box::new(Expr::Ident(on.clone())),
        span: DUMMY_SP,
        prop: MemberProp::Computed(ComputedPropName {
          span: DUMMY_SP,
          // Symbol.for("functionless:AST")
          expr: Box::new(Expr::Call(CallExpr {
            callee: Callee::Expr(Box::new(Expr::Member(MemberExpr {
              obj: Box::new(Expr::Ident(quote_ident!("Symbol"))),
              prop: MemberProp::Ident(quote_ident!("for")),
              span: DUMMY_SP,
            }))),
            args: vec![ExprOrSpread {
              expr: Box::new(Expr::Lit(Lit::Str(Str {
                raw: None,
                span: DUMMY_SP,
                value: JsWord::from(sym),
              }))),
              spread: None,
            }],
            span: DUMMY_SP,
            type_args: None,
          })),
        }),
      }))),
      op: AssignOp::Assign,
      right: Box::new(Expr::Ident(to.clone())),
      span: DUMMY_SP,
    })),
  })
}
