use swc_common::{util::take::Take, DUMMY_SP};
use swc_common::{BytePos, Span};
use swc_ecma_visit::VisitMut;
use swc_plugin::ast::*;
use swc_plugin::utils::{prepend_stmts, quote_ident};

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

  fn visit_mut_class(&mut self, class: &mut Class) {
    let mut register_stmts: Vec<Stmt> = class
      .body
      .iter()
      .filter_map(|member| match member {
        ClassMember::Method(method) => {
          let free_variables = self.discover_free_variables(method);

          if !free_variables.is_empty() {
            Some(register_closure_stmt(
              // global.__fnl__func(this.prototype.method_name, () => .. )
              Expr::Member(MemberExpr {
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
              }),
              free_variables,
            ))
          } else {
            None
          }
        }

        ClassMember::PrivateMethod(_method) => None,

        _ => None,
      })
      .collect();

    let constructor_free_variables: Vec<Id> = class
      .body
      .iter()
      .flat_map(|member| match member {
        ClassMember::Constructor(constructor) => self.discover_free_variables(constructor),
        ClassMember::ClassProp(prop) => self.discover_free_variables(prop),
        _ => vec![],
      })
      .collect();

    if !constructor_free_variables.is_empty() {
      register_stmts.push(register_closure_stmt(
        Expr::This(ThisExpr { span: DUMMY_SP }),
        constructor_free_variables,
      ));
    }
    class.visit_mut_children_with(self);

    class.body.push(ClassMember::StaticBlock(StaticBlock {
      span: DUMMY_SP,
      body: BlockStmt {
        span: DUMMY_SP,
        stmts: register_stmts,
      },
    }));
  }

  fn visit_mut_expr(&mut self, expr: &mut Expr) {
    match expr {
      Expr::Arrow(arrow) => {
        // analyze the free variable prior to transformation
        let free_variables = self.discover_free_variables(arrow);

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
          let call = Expr::Call(register_free_variables_call_expr(
            Box::new(Expr::Arrow(arrow.take())),
            free_variables,
          ));

          *expr = call;
        }
      }
      Expr::Fn(func) if func.function.body.is_some() => {
        // discover which identifiers within the closure point to free variables
        let free_variables = self.discover_free_variables(&func.function);

        // transform each of the children nodes now that we have extracted the free variables
        func.function.visit_mut_children_with(self);

        if !free_variables.is_empty() {
          // wrap the Function with a call to global.__fnl_func to
          let call = Expr::Call(register_free_variables_call_expr(
            Box::new(Expr::Fn(FnExpr {
              ident: func.ident.take(),
              function: func.function.take(),
            })),
            // decorate the closure with its free variables
            free_variables,
          ));

          *expr = call;
        }
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
      Stmt::Decl(Decl::Fn(func)) => {
        let free_variables = self.discover_free_variables(&func.function);

        if free_variables.is_empty() {
          None
        } else {
          Some(register_closure_stmt(
            Expr::Ident(func.ident.clone()),
            free_variables,
          ))
        }
      }
      _ => None,
    }
  }
}
fn register_closure_stmt(expr: Expr, free_variables: Vec<Id>) -> Stmt {
  Stmt::Expr(ExprStmt {
    expr: Box::new(Expr::Call(register_free_variables_call_expr(
      Box::new(expr),
      free_variables,
    ))),
    span: DUMMY_SP,
  })
}

fn register_free_variables_call_expr(expr: Box<Expr>, free_variables: Vec<Id>) -> CallExpr {
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
        expr: Box::new(Expr::Ident(quote_ident!("__filename"))),
        spread: None,
      },
      ExprOrSpread {
        expr: Box::new(Expr::Arrow(ArrowExpr {
          is_async: false,
          is_generator: false,
          type_params: None,
          span: DUMMY_SP,
          body: BlockStmtOrExpr::Expr(Box::new(Expr::Array(ArrayLit {
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
