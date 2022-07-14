use std::collections::HashSet;

use crate::{
  closure_decorator::ClosureDecorator,
  virtual_machine::{Scope, VirtualMachine},
};
use swc_plugin::ast::*;

pub enum ArrowOrFunction<'a> {
  ArrowFunction(&'a ArrowExpr),
  Function(&'a Function),
}

pub struct FreeVariableVisitor {
  /**
   * A VM instance for maintaining lexical scope while walking through the closure's contents.
   */
  vm: VirtualMachine,
  /**
   * A HashSet of discovered free variables.
   */
  free_variables: HashSet<Id>,
}

impl ClosureDecorator {
  pub fn discover_free_variables(&mut self, func: ArrowOrFunction) -> Vec<Id> {
    let mut visitor = FreeVariableVisitor {
      vm: VirtualMachine::new(),
      // outer_names, // O(1) clone
      free_variables: HashSet::new(),
    };

    match func {
      ArrowOrFunction::ArrowFunction(arrow) => {
        arrow.visit_with(&mut visitor);
      }
      ArrowOrFunction::Function(function) => {
        function.visit_with(&mut visitor);
      }
    };

    let mut free_variables = visitor.free_variables.into_iter().collect::<Vec<Id>>();
    free_variables.sort_by(|a, b| a.0.cmp(&b.0));
    free_variables
  }
}

// read-only visitor that will discover free variables
impl Visit for FreeVariableVisitor {
  fn visit_block_stmt(&mut self, block: &BlockStmt) {
    self.vm.enter(Scope::Block);

    self.vm.bind_block(block);

    block.visit_children_with(self);

    self.vm.exit();
  }

  fn visit_arrow_expr(&mut self, arrow: &ArrowExpr) {
    self.vm.enter(Scope::Function);

    self.vm.bind_all_pats(&arrow.params, Scope::Block);

    match &arrow.body {
      BlockStmtOrExpr::Expr(expr) => {
        expr.visit_with(self);
      }
      BlockStmtOrExpr::BlockStmt(block) => {
        self.vm.bind_block(block);
        block.visit_children_with(self);
      }
    }

    self.vm.exit();
  }

  fn visit_function(&mut self, function: &Function) {
    self.vm.enter(Scope::Function);

    // greedily bind all of the parameters into the block scope (any functions within the default arguments ca)
    // a default arrow function can access any of the parameters
    // function foo(a, b = () => a) {}
    self.vm.bind_all_params(&function.params, Scope::Block);

    match &function.body {
      Some(body) => {
        // hoist all function and var declarations from the body and make them available to the Function scope
        // function foo (b = () => bar) {
        //   function bar() {}
        //            ^ visible to b = () => bar
        // }
        self.vm.bind_block(&body);

        function.params.iter().for_each(|param| {
          // walk through each param left to right and bind them to the Block scope
          self.vm.bind_param(param, Scope::Block);

          // evaluate the pattern which may contain a default expression
          param.visit_with(self);
        });

        // step into the body and evaluate all statements
        // skip the BlockStmt (i.e. `body.visit_with(self)` because we have already created a function scope)
        body.visit_children_with(self);
      }
      _ => {}
    }

    self.vm.exit();
  }

  fn visit_var_declarator(&mut self, var: &VarDeclarator) {
    self.vm.bind_pat(&var.name, Scope::Block);

    match &var.init {
      Some(init) => {
        init.as_ref().visit_with(self);
      }
      _ => {}
    }
  }

  fn visit_ident(&mut self, ident: &Ident) {
    // all identifiers that are discovered by the visitor are assumed to be references
    // because of the order in which we traverse
    if self.vm.lookup_ident(ident, Scope::Block).is_none() {
      if &ident.sym == "i" {
        println!("hello {:#?}", ident.to_id());
      }
      self.free_variables.insert(ident.to_id());
    }
  }
}
