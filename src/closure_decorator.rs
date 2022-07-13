use std::collections::HashSet;

use swc_common::{util::take::Take, DUMMY_SP};
use swc_ecma_visit::VisitMut;
use swc_plugin::ast::*;
use swc_plugin::utils::quote_ident;

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
        *expr = self.wrap_arrow_expr(arrow);
      }
      Expr::Fn(func) => {
        *expr = self.wrap_function(&mut func.function, func.ident.take());
      }
      _ => {}
    }
  }

  fn visit_mut_module_items(&mut self, items: &mut Vec<ModuleItem>) {
    self.vm.bind_hoisted_stmts(items, Scope::Block);
    items.iter_mut().for_each(|stmt| stmt.visit_mut_with(self));
  }

  fn visit_mut_block_stmt(&mut self, block: &mut BlockStmt) {
    // we are entering a block, so push a frame onto the stack
    self.vm.enter(Scope::Block);

    self.vm.bind_hoisted_stmts(&block.stmts, Scope::Block);

    // now that all hoisted variables are in scope, walk each of the children
    block.visit_mut_children_with(self);

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
  fn wrap_function(&mut self, func: &mut Function, ident: Option<Ident>) -> Expr {
    // discover which identifiers within the closure point to free variables
    let free_variables = self.discover_free_variables(ArrowOrFunction::Function(func));

    self.vm.enter(Scope::Function);

    let block = func.body.as_mut().unwrap();

    // transform each of the children nodes now that we have extracted the free variables
    block.visit_mut_children_with(self);

    // replace the ArrowExpr with a call to global.wrapClosure to
    let call = Expr::Call(wrap_closure_call(
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

  fn wrap_arrow_expr(&mut self, arrow: &mut ArrowExpr) -> Expr {
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
    let call = Expr::Call(wrap_closure_call(
      Box::new(Expr::Arrow(arrow.take())),
      free_variables,
    ));

    self.vm.exit();

    call
  }
}

/**
 * global.wrapClosure((...args) => { ..stmts }, () => ({ ...metadata }))
 */
fn wrap_closure_call(expr: Box<Expr>, free_variables: HashSet<Id>) -> CallExpr {
  // global.wrapClosure((...args) => { ..stmts }, () => ({ ...metadata }))
  CallExpr {
    span: DUMMY_SP,
    callee: Callee::Expr(Box::new(Expr::Member(MemberExpr {
      obj: Box::new(Expr::Ident(quote_ident!("global"))),
      prop: MemberProp::Ident(quote_ident!("wrapClosure")),
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
                create_array(free_variables.iter().map(|v| {
                  create_object_lit(vec![create_prop("name", Expr::Ident(quote_ident!(*v.0)))])
                })),
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

fn create_object_lit(props: Vec<PropOrSpread>) -> Expr {
  Expr::Object(ObjectLit {
    span: DUMMY_SP,
    props: props,
  })
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

fn create_array<I>(items: I) -> Expr
where
  I: Iterator<Item = Expr>,
{
  Expr::Array(ArrayLit {
    span: DUMMY_SP,
    elems: items
      .map(|v| {
        Some(ExprOrSpread {
          expr: Box::new(v),
          spread: None,
        })
      })
      .collect(),
  })
}

// fn create_object_lit(key: &str, value: Expr) -> ObjectLit {
//   ObjectLit {
//     span: DUMMY_SP,
//     props: vec![PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
//       key: PropName::Ident(quote_ident!(key)),
//       value: Box::new(value),
//     })))],
//   }
// }
