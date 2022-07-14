use swc_common::{util::take::Take, DUMMY_SP};
use swc_common::{BytePos, Span};
use swc_ecma_visit::VisitMut;
use swc_plugin::ast::*;
use swc_plugin::utils::{prepend_stmts, quote_ident};

use crate::free_variables::ArrowOrFunction;

pub struct ClosureDecorator {}

impl ClosureDecorator {
  pub fn new() -> ClosureDecorator {
    ClosureDecorator {}
  }
}

impl VisitMut for ClosureDecorator {
  fn visit_mut_module_items(&mut self, items: &mut Vec<ModuleItem>) {
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
    // extract statements to register hoisted function declarations
    let register_stmts: Vec<Stmt> = block
      .stmts
      .iter()
      .filter_map(|stmt| self.register_stmt_if_func_decl(stmt))
      .collect();

    //
    block.visit_mut_children_with(self);

    prepend_stmts(&mut block.stmts, register_stmts.into_iter());
  }

  fn visit_mut_expr(&mut self, expr: &mut Expr) {
    match expr {
      Expr::Arrow(arrow) => {
        // analyze the free variable prior to transformation
        let free_variables = self.discover_free_variables(ArrowOrFunction::ArrowFunction(arrow));

        // transform all of the parameters
        arrow.params.visit_mut_with(self);

        // transform the closure's body
        match &mut arrow.body {
          BlockStmtOrExpr::Expr(expr) => {
            expr.visit_mut_with(self);
          }
          BlockStmtOrExpr::BlockStmt(block) => {
            block.visit_mut_children_with(self);
          }
        }

        if !free_variables.is_empty() {
          // wrap the closure in a decorator call
          let call = Expr::Call(register_closure_call(
            Box::new(Expr::Arrow(arrow.take())),
            free_variables,
          ));

          *expr = call;
        }
      }
      Expr::Fn(func) if func.function.body.is_some() => {
        // discover which identifiers within the closure point to free variables
        let free_variables =
          self.discover_free_variables(ArrowOrFunction::Function(&func.function));

        if !free_variables.is_empty() {
          let mut function = func.function.take();

          // transform each of the children nodes now that we have extracted the free variables
          function
            .body
            .as_mut()
            .unwrap()
            .visit_mut_children_with(self);

          // wrap the Function with a call to global.__fnl_func to
          let call = Expr::Call(register_closure_call(
            Box::new(Expr::Fn(FnExpr {
              function,
              ident: func.ident.take(),
            })),
            // decorate the closure with its free variables
            free_variables,
          ));

          *expr = call;
        }
      }
      _ => {}
    };
  }
}

impl ClosureDecorator {
  fn register_stmt_if_func_decl(&mut self, stmt: &Stmt) -> Option<Stmt> {
    match stmt {
      Stmt::Decl(Decl::Fn(func)) => {
        let free_variables =
          self.discover_free_variables(ArrowOrFunction::Function(&func.function));

        if free_variables.is_empty() {
          None
        } else {
          Some(Stmt::Expr(ExprStmt {
            expr: Box::new(Expr::Call(register_closure_call(
              Box::new(Expr::Ident(func.ident.clone())),
              free_variables,
            ))),
            span: DUMMY_SP,
          }))
        }
      }
      _ => None,
    }
  }
}

/**
 * global.__fnl_func((...args) => { ..stmts }, () => ({ ...metadata }))
 */
fn register_closure_call(expr: Box<Expr>, free_variables: Vec<Id>) -> CallExpr {
  // global.__fnl_func((...args) => { ..stmts }, () => ({ ...metadata }))
  CallExpr {
    span: DUMMY_SP,
    callee: Callee::Expr(Box::new(Expr::Member(MemberExpr {
      obj: Box::new(Expr::Ident(quote_ident!("global"))),
      prop: MemberProp::Ident(quote_ident!("__fnl_func")),
      span: DUMMY_SP,
    }))),
    args: vec![
      ExprOrSpread { expr, spread: None },
      ExprOrSpread {
        expr: Box::new(Expr::Arrow(ArrowExpr {
          is_async: false,
          is_generator: false,
          type_params: None,
          span: DUMMY_SP,
          body: BlockStmtOrExpr::Expr(Box::new(Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: vec![
              create_short_hand_prop("__filename"),
              create_prop(
                "free",
                Expr::Array(ArrayLit {
                  span: DUMMY_SP,
                  elems: free_variables
                    .iter()
                    .map(|v| {
                      Expr::Array(ArrayLit {
                        elems: vec![
                          Expr::Lit(Lit::Str(Str {
                            raw: None,
                            span: DUMMY_SP,
                            value: v.0.clone(),
                          })),
                          Expr::Lit(Lit::Num(Number {
                            span: DUMMY_SP,
                            value: v.1.as_u32() as f64,
                            raw: None,
                          })),
                          Expr::Ident(quote_ident!(
                            Span {
                              hi: BytePos(0),
                              lo: BytePos(0),
                              // this is very important - we must attach the SyntaxContext of the free variable's origin
                              // or else: when rust renames identifiers, it will skip this one, leaving us with a broken references
                              ctxt: v.1,
                            },
                            *v.0
                          )),
                        ]
                        .iter()
                        .map(|expr| {
                          Some(ExprOrSpread {
                            spread: None,
                            expr: Box::new(expr.clone()),
                          })
                        })
                        .collect(),
                        span: DUMMY_SP,
                      })
                    })
                    .map(|expr| {
                      Some(ExprOrSpread {
                        spread: None,
                        expr: Box::new(expr),
                      })
                    })
                    .collect(),
                }),
              ),
            ],
          }))),
          params: vec![],
          return_type: None,
        })),
        spread: None,
      },
    ], // TODO: inject metadata about free variables
    type_args: None,
  }
}

fn create_short_hand_prop(expr: &str) -> PropOrSpread {
  PropOrSpread::Prop(Box::new(Prop::Shorthand(quote_ident!(expr))))
}

fn create_prop(key: &str, value: Expr) -> PropOrSpread {
  PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
    key: PropName::Ident(quote_ident!(key)),
    value: Box::new(value),
  })))
}
