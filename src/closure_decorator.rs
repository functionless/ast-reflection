use std::collections::HashSet;

use swc_common::{util::take::Take, DUMMY_SP};
use swc_ecma_visit::VisitMut;
use swc_plugin::ast::*;
use swc_plugin::utils::{prepend_stmts, quote_ident};

use crate::free_variables::ArrowOrFunction;
use crate::virtual_machine::{Scope, VirtualMachine};

pub struct ClosureDecorator {
  pub vm: VirtualMachine,
}

impl ClosureDecorator {
  pub fn new() -> ClosureDecorator {
    ClosureDecorator {
      vm: VirtualMachine::new(),
    }
  }
}

impl VisitMut for ClosureDecorator {
  fn visit_mut_expr(&mut self, expr: &mut Expr) {
    match expr {
      Expr::Arrow(arrow) => {
        *expr = self.register_arrow_expr(arrow);
      }
      Expr::Fn(func) => {
        *expr = self.register_function(&mut func.function, func.ident.take());
      }
      _ => {}
    }
  }

  fn visit_mut_module_items(&mut self, items: &mut Vec<ModuleItem>) {
    self.vm.bind_hoisted_stmts(items, Scope::Block);

    items.iter_mut().for_each(|stmt| stmt.visit_mut_with(self));

    self.register_module_items_func_decls(items);
  }

  fn visit_mut_block_stmt(&mut self, block: &mut BlockStmt) {
    // we are entering a block, so push a frame onto the stack
    self.vm.enter(Scope::Block);

    self.vm.bind_hoisted_stmts(&block.stmts, Scope::Block);

    // now that all hoisted variables are in scope, walk each of the children
    block.visit_mut_children_with(self);

    // hoist some ExprStmt(CallExpr)s to register function declarations defined later in the block
    self.register_block_stmt_func_decls(&mut block.stmts);

    // finally, pop the stack frame
    self.vm.exit();
  }

  fn visit_mut_var_decl(&mut self, var: &mut VarDecl) {
    for decl in var.decls.iter_mut() {
      // bind the variable into scope
      self.vm.bind_var_declarator(var.kind, decl, Scope::Block);

      if decl.init.is_some() {
        // then visit the initializer with the updated lexical scope
        decl.init.as_deref_mut().unwrap().visit_mut_with(self);
      }
    }
  }

  fn visit_mut_pat(&mut self, param: &mut Pat) {
    // bind this argument into lexical scope

    self.vm.bind_pat(param, Scope::Block);
    match param {
      Pat::Assign(assign) => {
        // this is a parameter with a default value
        // e.g (a, b = a)
        // or  (a, b = () => [a, b])

        // we must transform the initializer with the arguments to its left in scope
        assign.right.as_mut().visit_mut_children_with(self);
      }
      _ => {}
    }
  }
}

impl ClosureDecorator {
  fn register_module_items_func_decls(&mut self, items: &mut Vec<ModuleItem>) {
    let register_stmts: Vec<ModuleItem> = items
      .iter()
      .filter_map(|item| match item {
        ModuleItem::Stmt(stmt) => Some(stmt),
        _ => None,
      })
      .filter_map(|stmt| self.filter_map_func_decl_to_register_call(stmt))
      .map(|stmt| ModuleItem::Stmt(stmt))
      .collect();

    prepend_stmts(items, register_stmts.into_iter());
  }

  fn register_block_stmt_func_decls(&mut self, stmts: &mut Vec<Stmt>) {
    let register_stmts: Vec<Stmt> = stmts
      .iter()
      .filter_map(|stmt| self.filter_map_func_decl_to_register_call(stmt))
      .collect();

    prepend_stmts(stmts, register_stmts.into_iter());
  }

  fn filter_map_func_decl_to_register_call(&mut self, stmt: &Stmt) -> Option<Stmt> {
    match stmt {
      Stmt::Decl(Decl::Fn(func)) => {
        let free_variables =
          self.discover_free_variables(ArrowOrFunction::Function(&func.function));

        Some(Stmt::Expr(ExprStmt {
          expr: Box::new(Expr::Call(register_closure_call(
            Box::new(Expr::Ident(func.ident.clone())),
            free_variables,
          ))),
          span: DUMMY_SP,
        }))
      }
      _ => None,
    }
  }

  fn register_function(&mut self, func: &mut Function, ident: Option<Ident>) -> Expr {
    // discover which identifiers within the closure point to free variables
    let free_variables = self.discover_free_variables(ArrowOrFunction::Function(func));

    self.vm.enter(Scope::Function);

    let block = func.body.as_mut().unwrap();

    // transform each of the children nodes now that we have extracted the free variables
    block.visit_mut_children_with(self);

    // wrap the Function with a call to global.__fnl_func to
    let call = Expr::Call(register_closure_call(
      Box::new(Expr::Fn(FnExpr {
        function: func.take(),
        ident,
      })),
      // decorate the closure with its free variables
      free_variables,
    ));

    self.vm.exit();

    call
  }

  fn register_arrow_expr(&mut self, arrow: &mut ArrowExpr) -> Expr {
    // analyze the free variable prior to transformation
    let free_variables = self.discover_free_variables(ArrowOrFunction::ArrowFunction(arrow));

    // push a new frame onto the stack for the contents of this function
    self.vm.enter(Scope::Function);

    // transform the closure's body
    match &mut arrow.body {
      BlockStmtOrExpr::Expr(expr) => {
        expr.visit_mut_with(self);
      }
      BlockStmtOrExpr::BlockStmt(block) => {
        self.vm.bind_hoisted_stmts(&block.stmts, Scope::Block);
        block.visit_mut_children_with(self);
      }
    }

    // wrap the closure in a decorator call
    let call = Expr::Call(register_closure_call(
      Box::new(Expr::Arrow(arrow.take())),
      free_variables,
    ));

    self.vm.exit();

    call
  }
}

/**
 * global.__fnl_func((...args) => { ..stmts }, () => ({ ...metadata }))
 */
fn register_closure_call(expr: Box<Expr>, free_variables: HashSet<Id>) -> CallExpr {
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
                          Expr::Ident(Ident {
                            optional: false,
                            span: DUMMY_SP,
                            sym: v.0.clone(),
                          }),
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
