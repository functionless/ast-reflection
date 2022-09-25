use std::iter;

use swc_atoms::JsWord;
use swc_core::ast::*;
use swc_core::common::util::take::Take;
use swc_core::common::DUMMY_SP;
use swc_core::plugin::proxies::PluginSourceMapProxy;
use swc_core::utils::{prepend_stmts, private_ident, quote_ident, quote_str};
use swc_core::visit::*;

use crate::class_like::ClassLike;
use crate::js_util::{prop_access_expr, ref_expr, require_expr, string_expr, this_expr};
use crate::prepend::prepend;
use crate::span::{concat_span, get_prop_name_span};
use crate::virtual_machine::VirtualMachine;

/**
 * Flags used to identify the Functionless AST association sequences.
 *
 * These can be used by downstream programs to discover and manipulate the injects
 * functionless assignment sequences.
 */
// (stash=REGISTER,stash=func,stash[]=ast,stash)
const REGISTER_FLAG: &str = "REGISTER";
// (stash=REGISTER_REF,stash[]=ast)
const REGISTER_REF_FLAG: &str = "REGISTER_REF";
// // (stash=BIND, ...)
const BIND_FLAG: &str = "BIND";
// // (stash=PROXY, ...)
const PROXY_FLAG: &str = "PROXY";

const GLOBAL_THIS_NAME: &str = "global_8269d1a8";

/**
 * A unique variable used to story temporary values like registering functions with their AST.
 *
 * Example:
 * ```ts
 * const func = () => {};
 * ```
 *
 * becomes:
 *
 * ```ts
 * let stash;
 *
 * const func = (stash = () => {}, stash[Symbol.for(...)] = ast, stash);
 * ```
 */
const STASH_NAME: &str = "stash_8269d1a8";

const UTIL_FUNCTION_NAME: &str = "util_8269d1a8";

pub struct ClosureDecorator<'a> {
  /**
   * A reference to the source file's source map. We need this to map byte positions back to line and column.
   */
  pub source_map: &'a PluginSourceMapProxy,
  /**
   * A Virtual Machine managing lexical scope as we walk the tree.
   */
  pub vm: VirtualMachine,
  /**
   * A private identifier to globalThis.
   */
  pub global: Ident,
  /**
   * A unique identifier functionless uses to story temporary values for registration.
   */
  pub stash: Ident,
  /**
   * An Identifier referencing NodeJS's `util` module.
   */
  pub util: Ident,
  /**
   * A counter for generating unique IDs
   */
  ids: u32,
}

impl<'a> ClosureDecorator<'a> {
  pub fn next_id(&mut self) -> u32 {
    self.ids += 1;
    self.ids
  }

  pub fn new(source_map: &PluginSourceMapProxy) -> ClosureDecorator {
    ClosureDecorator {
      source_map,
      vm: VirtualMachine::new(),
      global: private_ident!(GLOBAL_THIS_NAME),
      stash: private_ident!(STASH_NAME),
      util: private_ident!(UTIL_FUNCTION_NAME),
      ids: 0,
    }
  }
}

impl<'a> VisitMut for ClosureDecorator<'a> {
  fn visit_mut_module_items(&mut self, items: &mut Vec<ModuleItem>) {
    let new_stmts: Vec<ModuleItem> = [
      ModuleItem::Stmt(Stmt::Decl(Decl::Var(VarDecl {
        span: DUMMY_SP,
        declare: false,
        decls: vec![VarDeclarator {
          definite: false,
          name: Pat::Ident(BindingIdent {
            id: self.stash.clone(),
            type_ann: None,
          }),
          init: None,
          span: DUMMY_SP,
        }],
        kind: VarDeclKind::Let,
      }))),
      ModuleItem::Stmt(Stmt::Decl(self.create_global_this())),
      ModuleItem::Stmt(Stmt::Decl(self.create_import_util())),
    ]
    .into_iter()
    .chain(
      // discover all function declarations in the module and
      // add the expressions to `register` function with its AST.
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
    // add the expressions to `register` function with its AST.
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
          value: self.register_ast_seq(func, ast),
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

        // replace the arrow function with the expressions which associate the function to it's AST
        // () => {..}
        // =>
        // ("REGISTER", stash = () => {...}, stash[Symbol.for("AST")] = ast, stash)
        *expr = *self.register_mut_ast(&mut Expr::Arrow(arrow.take()), ast);
      }
      Expr::Fn(func) if func.function.body.is_some() => {
        let ast = self.parse_function_expr(&func, true);

        func.visit_mut_children_with(self);

        // replace the function expression with a call to the `register` interceptor function
        // that decorates the function with its AST
        // function foo() {..}
        // =>
        // ("REGISTER", stash = function foo() {..}, stash[Symbol.for("AST")] = ast, stash)
        *expr = *self.register_mut_ast(&mut Expr::Fn(func.take()), ast);
      }
      Expr::New(NewExpr { args, callee, .. }) if args.is_some() => {
        // detect if this looks like a new Proxy
        // e.g. `new Proxy({}, {})`
        let maybe_proxy_class = callee.as_mut_ident().filter(|ident| &ident.sym == "Proxy");

        if maybe_proxy_class.is_some() {
          let proxy_class = maybe_proxy_class.unwrap();

          args
            .iter_mut()
            .for_each(|arg| arg.visit_mut_children_with(self));

          proxy_class.visit_mut_children_with(self);

          // replace the NewExpr with a call to the `bind` interceptor function
          // new Proxy({}, {})
          // =>
          // ("PROXY", ...)
          *expr = self.wrap_proxy(
            Box::new(proxy_class.take()),
            args.take().unwrap_or_default(),
          );
        } else {
          expr.visit_mut_children_with(self)
        }
      }
      Expr::Call(call) => {
        let maybe_bind_expr = match &mut call.callee {
          Callee::Expr(expr) => match expr.as_mut() {
            Expr::Member(member) => match member {
              MemberExpr { obj, prop, .. }
                if prop
                  .as_ident()
                  .filter(|ident| &ident.sym == "bind")
                  .is_some() =>
              {
                Some(obj)
              }
              _ => None,
            },
            _ => None,
          },
          _ => None,
        };

        if maybe_bind_expr.is_some() {
          let func = maybe_bind_expr.unwrap();
          call
            .args
            .iter_mut()
            .for_each(|arg| arg.visit_mut_children_with(self));

          func.visit_mut_children_with(self);

          // replace the CallExpr with a call to the `bind` interceptor function
          // foo.bind(bar)
          // =>
          // ("BIND", ...);
          *expr = self.wrap_bind(
            func.take(),
            call.args[0].expr.clone(),
            call.args[1..].to_vec(),
          )
        } else {
          expr.visit_mut_children_with(self);
        }
      }
      _ => {
        expr.visit_mut_children_with(self);
      }
    };
  }
}

impl<'a> ClosureDecorator<'a> {
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
    let method_ast = self.parse_method_like(
      method,
      Some(ref_expr(self.source_map, this_expr())),
      &method.span,
    );

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
      expr: self.register_ref_ast_seq(expr, ast),
      span: DUMMY_SP,
    })
  }

  fn register_mut_ast(&self, func: &mut Expr, ast: Box<Expr>) -> Box<Expr> {
    self.register_ast_seq(Box::new(func.take()), ast)
  }

  /**
   * Declare a private variable pointing to the globalThis without possibility of lexical scope collision.
   * const Global_8269d1a8 = register_8269d1a8.constructor("return this")();
   */
  fn create_global_this(&self) -> Decl {
    Decl::Var(VarDecl {
      declare: false,
      kind: VarDeclKind::Const,
      span: DUMMY_SP,
      decls: vec![VarDeclarator {
        span: DUMMY_SP,
        definite: false,
        name: Pat::Ident(BindingIdent {
          id: self.global.clone(),
          type_ann: None,
        }),
        init: Some(Box::new(Expr::Call(CallExpr {
          span: DUMMY_SP,
          type_args: None,
          callee: Callee::Expr(Box::new(Expr::Call(CallExpr {
            span: DUMMY_SP,
            type_args: None,
            callee: Callee::Expr(prop_access_expr(
              Box::new(Expr::Arrow(ArrowExpr {
                is_async: false,
                is_generator: false,
                return_type: None,
                params: vec![],
                span: DUMMY_SP,
                type_params: None,
                body: BlockStmtOrExpr::BlockStmt(BlockStmt {
                  span: DUMMY_SP,
                  stmts: vec![],
                }),
              })),
              quote_ident!("constructor"),
            )),
            args: vec![ExprOrSpread {
              spread: None,
              expr: string_expr("return this;"),
            }],
          }))),
          args: vec![],
        }))),
      }],
    })
  }

  fn symbol_ref(&self) -> Box<Expr> {
    Box::new(Expr::Member(MemberExpr {
      obj: Box::new(Expr::Ident(self.global.clone())),
      span: DUMMY_SP,
      prop: MemberProp::Ident(quote_ident!("Symbol")),
    }))
  }

  fn create_import_util(&self) -> Decl {
    Decl::Var(VarDecl {
      declare: false,
      span: DUMMY_SP,
      kind: VarDeclKind::Const,
      decls: vec![VarDeclarator {
        definite: false,
        span: DUMMY_SP,
        name: Pat::Ident(BindingIdent {
          id: self.util.clone(),
          type_ann: None,
        }),
        init: Some(require_expr("util")),
      }],
    })
  }

  /**
   * Registers a function or object with it's AST and returns a reference to it's value using the stash variable.
   *
   * ```ts
   * const x = () => {}
   * ```
   *
   * becomes
   *
   * ```ts
   * ("REGISTER", stash = () => {}, stash[Symbol.from(functionless::AST)] = ast, stash)
   * ```
   */
  fn register_ast_seq(&self, func: Box<Expr>, ast: Box<Expr>) -> Box<Expr> {
    return Box::new(Expr::Seq(SeqExpr {
      span: DUMMY_SP,
      exprs: vec![
        // "REGISTER"
        Box::new(Expr::Assign(AssignExpr {
          left: PatOrExpr::Expr(Box::new(Expr::Ident(self.stash.clone()))),
          op: AssignOp::Assign,
          right: string_expr(REGISTER_FLAG),
          span: DUMMY_SP,
        })),
        // stash=func
        Box::new(Expr::Assign(AssignExpr {
          left: PatOrExpr::Expr(Box::new(Expr::Ident(self.stash.clone()))),
          op: AssignOp::Assign,
          right: func,
          span: DUMMY_SP,
        })),
        // stash[Symbol.from(functionless::AST)] = ast
        Box::new(self.set_symbol_expr(
          Box::new(Expr::Ident(self.stash.clone())),
          "functionless:AST",
          // is this right?
          Box::new(Expr::Arrow(ArrowExpr {
            is_async: false,
            is_generator: false,
            return_type: None,
            params: vec![],
            span: DUMMY_SP,
            type_params: None,
            body: BlockStmtOrExpr::Expr(ast),
          })),
        )),
        // stash
        Box::new(Expr::Ident(self.stash.clone())),
      ],
    }));
  }

  /**
   * Registers an identifier with it's AST.
   *
   * (stash=REGISTER_REF,x[Symbol.for(AST)]=ast)
   * function x() {
   *    ....
   * }
   */
  fn register_ref_ast_seq(&self, expr: Box<Expr>, ast: Box<Expr>) -> Box<Expr> {
    return Box::new(Expr::Seq(SeqExpr {
      span: DUMMY_SP,
      exprs: vec![
        // "REGISTER"
        Box::new(Expr::Assign(AssignExpr {
          left: PatOrExpr::Expr(Box::new(Expr::Ident(self.stash.clone()))),
          op: AssignOp::Assign,
          right: string_expr(REGISTER_REF_FLAG),
          span: DUMMY_SP,
        })),
        // stash[Symbol.from(functionless::AST)] = ast
        Box::new(self.set_symbol_expr(
          expr,
          "functionless:AST",
          // is this right?
          Box::new(Expr::Arrow(ArrowExpr {
            is_async: false,
            is_generator: false,
            return_type: None,
            params: vec![],
            span: DUMMY_SP,
            type_params: None,
            body: BlockStmtOrExpr::Expr(ast),
          })),
        )),
      ],
    }));
  }

  /**
   * ("BIND",
   *    stash={ args: args, self: this, func: func },
   *    stash={ f: stash.func.bind(stash.self, ...stash.args), ...stash },
   *    typeof stash.f === "function" && (
   *        stash.f[Symbol.for("functionless:BoundThis")] = stash.self,
   *        stash.f[Symbol.for("functionless:BoundArgs")] = stash.args,
   *        stash.f[Symbol.for("functionless:TargetFunction")] = stash.func
   *    ),
   *    stash
   * )
   */
  fn wrap_bind(&self, func: Box<Expr>, this: Box<Expr>, args: Vec<ExprOrSpread>) -> Expr {
    let member_f = Box::new(Expr::Member(MemberExpr {
      span: DUMMY_SP,
      obj: Box::new(Expr::Ident(self.stash.clone())),
      prop: MemberProp::Ident(quote_ident!("f")),
    }));
    // the original function stored in stash
    let member_func = Box::new(Expr::Member(MemberExpr {
      span: DUMMY_SP,
      obj: Box::new(Expr::Ident(self.stash.clone())),
      prop: MemberProp::Ident(quote_ident!("func")),
    }));
    let member_args = Box::new(Expr::Member(MemberExpr {
      span: DUMMY_SP,
      obj: Box::new(Expr::Ident(self.stash.clone())),
      prop: MemberProp::Ident(quote_ident!("args")),
    }));
    //
    let member_this = Box::new(Expr::Member(MemberExpr {
      span: DUMMY_SP,
      obj: Box::new(Expr::Ident(self.stash.clone())),
      prop: MemberProp::Ident(quote_ident!("this")),
    }));

    return Expr::Seq(SeqExpr {
      span: DUMMY_SP,
      exprs: vec![
        Box::new(Expr::Assign(AssignExpr {
          left: PatOrExpr::Expr(Box::new(Expr::Ident(self.stash.clone()))),
          op: AssignOp::Assign,
          right: string_expr(BIND_FLAG),
          span: DUMMY_SP,
        })),
        // stash={ args, self, func}
        Box::new(Expr::Assign(AssignExpr {
          span: DUMMY_SP,
          left: PatOrExpr::Expr(Box::new(Expr::Ident(self.stash.clone()))),
          op: AssignOp::Assign,
          right: Box::new(Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: vec![
              // args = [args]
              PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                key: PropName::Ident(quote_ident!("args")),
                value: Box::new(Expr::Array(ArrayLit {
                  span: DUMMY_SP,
                  elems: args.into_iter().map(Some).collect(),
                })),
              }))),
              // this = this
              PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                key: PropName::Ident(quote_ident!("this")),
                value: this,
              }))),
              // func = func
              PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                key: PropName::Ident(quote_ident!("func")),
                value: func,
              }))),
            ],
          })),
        })),
        // stash = { f: stash.func.bind(stash.this, stash.args), ...stash }
        Box::new(Expr::Assign(AssignExpr {
          span: DUMMY_SP,
          left: PatOrExpr::Expr(Box::new(Expr::Ident(self.stash.clone()))),
          op: AssignOp::Assign,
          right: Box::new(Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: vec![
              PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                key: PropName::Ident(quote_ident!("f")),
                value: Box::new(Expr::Call(CallExpr {
                  span: DUMMY_SP,
                  type_args: None,
                  callee: Callee::Expr(Box::new(Expr::Member(MemberExpr {
                    obj: member_func.clone(),
                    prop: MemberProp::Ident(quote_ident!("bind")),
                    span: DUMMY_SP,
                  }))),
                  args: vec![
                    ExprOrSpread {
                      expr: member_this.clone(),
                      spread: None,
                    },
                    // ...stash.args
                    ExprOrSpread {
                      expr: member_args.clone(),
                      spread: Some(DUMMY_SP),
                    },
                  ],
                })),
              }))),
              // ...stash
              PropOrSpread::Spread(SpreadElement {
                dot3_token: DUMMY_SP,
                expr: Box::new(Expr::Ident(self.stash.clone())),
              }),
            ],
          })),
        })),
        // typeof stash.f === "function" && (stash.f[]=..., stash.f[]=..., stash.f[]=...)
        Box::new(Expr::Bin(BinExpr {
          span: DUMMY_SP,
          op: BinaryOp::LogicalAnd,
          left: Box::new(Expr::Bin(BinExpr {
            span: DUMMY_SP,
            left: Box::new(Expr::Unary(UnaryExpr {
              arg: member_f.clone(),
              span: DUMMY_SP,
              op: UnaryOp::TypeOf,
            })),
            op: BinaryOp::EqEqEq,
            right: Box::new(Expr::Lit(Lit::Str(quote_str!("function")))),
          })),
          right: Box::new(Expr::Seq(SeqExpr {
            span: DUMMY_SP,
            exprs: vec![
              // stash.f["functionless:BoundThis"] = stash.this;
              Box::new(self.set_symbol_expr(
                member_f.clone(),
                "functionless:BoundThis",
                member_this.clone(),
              )),
              // stash.f["functionless:BoundArgs"] = stash.args;
              Box::new(self.set_symbol_expr(
                member_f.clone(),
                "functionless:BoundArgs",
                member_args.clone(),
              )),
              // stash.f["functionless:TargetFunction"] = stash.func;
              Box::new(self.set_symbol_expr(
                member_f.clone(),
                "functionless:TargetFunction",
                member_func.clone(),
              )),
            ],
          })),
        })),
        // return stash.f
        member_f.clone(),
      ],
    });
  }

  /**
   * ("PROXY",
   *    stash={ args }, // ensure the args are only evaluated once
   *    stash={ proxy: new clss(...stash.args), ...stash }, // create the proxy
   *    (globalThis.util.types.isProxy(stash.proxy) && (
   *       globalThis.proxies = globalThis.proxies ?? new globalThis.WeakMap()
   *    ).set(stash.proxy, stash.args)
   *    proxy
   * )
   */
  fn wrap_proxy(&self, clss: Box<Ident>, args: Vec<ExprOrSpread>) -> Expr {
    let ident_args = quote_ident!("args");
    let ident_proxy = quote_ident!("proxy");
    let global_proxies = Expr::Member(MemberExpr {
      span: DUMMY_SP,
      obj: Box::new(Expr::Ident(self.global.clone())),
      prop: MemberProp::Computed(ComputedPropName {
        expr: self.symbol_for("functionless:Proxies"),
        span: DUMMY_SP,
      }),
    });

    let member_proxy = Box::new(Expr::Member(MemberExpr {
      span: DUMMY_SP,
      obj: Box::new(Expr::Ident(self.stash.clone())),
      prop: MemberProp::Ident(ident_proxy.clone()),
    }));
    let member_args = Box::new(Expr::Member(MemberExpr {
      span: DUMMY_SP,
      obj: Box::new(Expr::Ident(self.stash.clone())),
      prop: MemberProp::Ident(ident_args.clone()),
    }));

    return Expr::Seq(SeqExpr {
      span: DUMMY_SP,
      exprs: vec![
        Box::new(Expr::Assign(AssignExpr {
          left: PatOrExpr::Expr(Box::new(Expr::Ident(self.stash.clone()))),
          op: AssignOp::Assign,
          right: string_expr(PROXY_FLAG),
          span: DUMMY_SP,
        })),
        // stash={ args }
        Box::new(Expr::Assign(AssignExpr {
          span: DUMMY_SP,
          left: PatOrExpr::Expr(Box::new(Expr::Ident(self.stash.clone()))),
          op: AssignOp::Assign,
          right: Box::new(Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: vec![
              // args = [args]
              PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                key: PropName::Ident(ident_args.clone()),
                value: Box::new(Expr::Array(ArrayLit {
                  span: DUMMY_SP,
                  elems: args.into_iter().map(Some).collect(),
                })),
              }))),
            ],
          })),
        })),
        // stash = { proxy: new clss(...args), ...stash }
        Box::new(Expr::Assign(AssignExpr {
          span: DUMMY_SP,
          left: PatOrExpr::Expr(Box::new(Expr::Ident(self.stash.clone()))),
          op: AssignOp::Assign,
          right: Box::new(Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: vec![
              PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                key: PropName::Ident(ident_proxy.clone()),
                value: Box::new(Expr::New(NewExpr {
                  span: DUMMY_SP,
                  type_args: None,
                  callee: Box::new(Expr::Ident(*clss)),
                  args: Some(vec![ExprOrSpread {
                    expr: member_args.clone(),
                    spread: Some(DUMMY_SP),
                  }]),
                })),
              }))),
              // ...stash
              PropOrSpread::Spread(SpreadElement {
                dot3_token: DUMMY_SP,
                expr: Box::new(Expr::Ident(self.stash.clone())),
              }),
            ],
          })),
        })),
        // (typeof stash.proxy === "function" &&
        //       (globalThis.proxies = globalThis.proxies ?? new globalThis.WeakMap())
        //            .set(stash.proxy, stash.args), stash.proxy)
        Box::new(Expr::Bin(BinExpr {
          span: DUMMY_SP,
          op: BinaryOp::LogicalAnd,
          // typeof stash.proxy === "function"
          left: Box::new(Expr::Bin(BinExpr {
            span: DUMMY_SP,
            left: Box::new(Expr::Unary(UnaryExpr {
              arg: member_proxy.clone(),
              span: DUMMY_SP,
              op: UnaryOp::TypeOf,
            })),
            op: BinaryOp::EqEqEq,
            right: Box::new(Expr::Lit(Lit::Str(quote_str!("function")))),
          })),
          right: Box::new(Expr::Call(CallExpr {
            span: DUMMY_SP,
            callee: Callee::Expr(Box::new(Expr::Member(MemberExpr {
              // globalThis.proxies = globalThis.proxies ?? new globalThis.WeakMap())
              obj: Box::new(Expr::Assign(AssignExpr {
                // globalThis.proxies
                left: PatOrExpr::Expr(Box::new(global_proxies.clone())),
                op: AssignOp::Assign,
                span: DUMMY_SP,
                right: Box::new(Expr::Bin(BinExpr {
                  op: BinaryOp::NullishCoalescing,
                  // globalThis.proxies
                  left: Box::new(global_proxies.clone()),
                  span: DUMMY_SP,
                  // new globalThis.WeakMap()
                  right: Box::new(Expr::New(NewExpr {
                    type_args: None,
                    span: DUMMY_SP,
                    // globalThis.WeakMap
                    callee: Box::new(Expr::Member(MemberExpr {
                      span: DUMMY_SP,
                      obj: Box::new(Expr::Ident(self.global.clone())),
                      prop: MemberProp::Ident(quote_ident!("WeakMap")),
                    })),
                    args: None,
                  })),
                })),
              })),
              span: DUMMY_SP,
              prop: MemberProp::Ident(quote_ident!("set")),
            }))),
            args: vec![
              ExprOrSpread {
                spread: None,
                expr: member_proxy.clone(),
              },
              ExprOrSpread {
                spread: None,
                expr: member_args.clone(),
              },
            ],
            type_args: None,
          })),
        })),
        // return stash.proxy
        member_proxy.clone(),
      ],
    });
  }

  fn set_symbol_expr(&self, on: Box<Expr>, sym: &str, to: Box<Expr>) -> Expr {
    Expr::Assign(AssignExpr {
      // func[Symbol.for("functionless:AST")] = ast
      left: PatOrExpr::Expr(Box::new(Expr::Member(MemberExpr {
        obj: on,
        span: DUMMY_SP,
        prop: MemberProp::Computed(ComputedPropName {
          span: DUMMY_SP,
          expr: self.symbol_for(sym),
        }),
      }))),
      op: AssignOp::Assign,
      right: to.clone(),
      span: DUMMY_SP,
    })
  }

  fn symbol_for(&self, sym: &str) -> Box<Expr> {
    Box::new(Expr::Call(CallExpr {
      callee: Callee::Expr(prop_access_expr(self.symbol_ref(), quote_ident!("for"))),
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
    }))
  }
}
