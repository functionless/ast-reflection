use swc_common::{util::take::Take, DUMMY_SP};
use swc_common::{BytePos, Span, SyntaxContext};
use swc_ecma_visit::VisitMut;
use swc_plugin::ast::*;
use swc_plugin::utils::{prepend_stmts, quote_ident};

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
   * An Identifier referencing the global Functionless value.
   */
  pub functionless: Ident,
}

impl ClosureDecorator {
  pub fn new() -> ClosureDecorator {
    ClosureDecorator {
      vm: VirtualMachine::new(),
      // global reference to Functionless
      // our require-hook will ensure this value is globally available
      functionless: Ident {
        span: Span {
          // use syntax context of 0 for global lexical scope
          // TODO: validate this works
          ctxt: SyntaxContext::from_u32(0),
          hi: BytePos(0),
          lo: BytePos(0),
        },
        sym: JsWord::from("Functionless"),
        optional: false,
      },
    }
  }
}

impl VisitMut for ClosureDecorator {
  fn visit_mut_module_items(&mut self, items: &mut Vec<ModuleItem>) {
    self.vm.bind_module_items(items);

    prepend_stmts(
      items,
      vec![
        ModuleItem::Stmt(Stmt::Decl(create_register_function())),
        ModuleItem::Stmt(Stmt::Decl(create_bind_function())),
      ]
      .into_iter(),
    );

    // extract statements to register hoisted function declarations
    let register_stmts: Vec<ModuleItem> = items
      .iter()
      .filter_map(|item| match item {
        ModuleItem::Stmt(stmt) => Some(stmt),
        _ => None,
      })
      .filter_map(|stmt| self.register_stmt_if_func_decl(stmt))
      .map(|stmt| ModuleItem::Stmt(stmt))
      .collect();

    // transform each of the statements in the module
    items.iter_mut().for_each(|stmt| stmt.visit_mut_with(self));

    // finally, prepend the __fnl_func calls to the top of the module
    prepend_stmts(items, register_stmts.into_iter());
  }

  fn visit_mut_block_stmt(&mut self, block: &mut BlockStmt) {
    self.vm.enter();

    block.visit_mut_children_with(self);

    self.vm.exit();
  }

  fn visit_mut_stmts(&mut self, stmts: &mut Vec<Stmt>) {
    self.vm.bind_stmts(stmts);

    // extract statements to register hoisted function declarations
    let register_stmts: Vec<Stmt> = stmts
      .iter()
      .filter_map(|stmt| self.register_stmt_if_func_decl(stmt))
      .collect();

    stmts.visit_mut_children_with(self);

    prepend_stmts(stmts, register_stmts.into_iter());
  }

  fn visit_mut_class(&mut self, class: &mut Class) {
    let register_stmts: Vec<Stmt> = class
      .body
      .iter()
      .filter_map(|member| match member {
        ClassMember::Method(method) => Some(self.register_ast_stmt(
          // global.__fnl__func(this.prototype.method_name, () => .. )
          Box::new(Expr::Member(MemberExpr {
            // this.prototype
            obj: Box::new(Expr::Member(MemberExpr {
              obj: Box::new(Expr::This(ThisExpr { span: DUMMY_SP })),
              prop: MemberProp::Ident(quote_ident!("prototype")),
              span: DUMMY_SP,
            })),
            span: DUMMY_SP,
            prop: match &method.key {
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
            },
          })),
          self.parse_class_method(method),
        )),

        ClassMember::PrivateMethod(_method) => None,

        _ => None,
      })
      .collect();

    // class.body.iter().for_each(|member| match member {
    //   ClassMember::Constructor(ctor) => {
    //     ctor.body.iter().for_each(|body| {});
    //   }
    //   _ => {}
    // });

    class.visit_mut_children_with(self);

    class.body.push(ClassMember::StaticBlock(StaticBlock {
      span: DUMMY_SP,
      body: BlockStmt {
        span: DUMMY_SP,
        stmts: register_stmts,
      },
    }));
  }

  fn visit_mut_params(&mut self, params: &mut Vec<Param>) {
    self.vm.bind_all_params(params);
  }

  fn visit_mut_pats(&mut self, pats: &mut Vec<Pat>) {
    self.vm.bind_all_pats(pats);
  }

  fn visit_mut_pat(&mut self, pat: &mut Pat) {
    self.vm.bind_pat(pat);
  }

  fn visit_mut_expr(&mut self, expr: &mut Expr) {
    match expr {
      Expr::Arrow(arrow) => {
        let ast = self.parse_arrow(arrow);

        self.vm.enter();

        arrow.params.visit_mut_with(self);

        arrow.body.visit_mut_with(self);

        self.vm.exit();

        *expr = *self.register_mut_ast(&mut Expr::Arrow(arrow.take()), ast);
      }
      Expr::Fn(func) if func.function.body.is_some() => {
        let ast = self.parse_function_expr(&func.function);

        // create a new scope for the function parameters
        self.vm.enter();

        self.vm.bind_all_params(&func.function.params);

        func.function.params.visit_mut_with(self);

        func.function.body.visit_mut_with(self);

        *expr = *self.register_mut_ast(&mut Expr::Fn(func.take()), ast);

        // exit the function parameters scope
        self.vm.exit();
      }
      _ => {
        expr.visit_mut_children_with(self);
      }
    };
  }
}

impl ClosureDecorator {
  fn register_stmt_if_func_decl(&mut self, stmt: &Stmt) -> Option<Stmt> {
    match stmt {
      Stmt::Decl(Decl::Fn(func)) => Some(self.register_ast_stmt(
        Box::new(Expr::Ident(func.ident.clone())),
        self.parse_function_decl(&func.function),
      )),
      _ => None,
    }
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
      // Functionless.register(() =>  { .. }, () => new FunctionDecl([..]))
      callee: Callee::Expr(Box::new(Expr::Member(MemberExpr {
        span: DUMMY_SP,
        obj: Box::new(Expr::Ident(self.functionless.clone())),
        prop: MemberProp::Ident(quote_ident!("register")),
      }))),
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
