use std::collections::HashSet;

use crate::{
  closure_decorator::ClosureDecorator,
  virtual_machine::{Names, Scope, VirtualMachine},
};
use swc_plugin::ast::*;

pub enum ArrowOrFunction<'a> {
  ArrowFunction(&'a ArrowExpr),
  Function(&'a Function),
}

pub struct FreeVariableVisitor {
  vm: VirtualMachine,
  /**
   * A reference to the names visible outside of a closure.
   */
  outer_names: Names,
  /**
   * A HashSet of discovered free variables.
   */
  free_variables: HashSet<Id>,
}

impl ClosureDecorator {
  pub fn discover_free_variables(&mut self, func: ArrowOrFunction) -> HashSet<Id> {
    // store the state of the outer block scope (names visible leading up to the declaration of this function)
    let outer_names = self.vm.get_names(Scope::Block).clone();

    // push an empty scope onto the stack - any variable we encounter that is not in the isolated scope must be a free variable
    self.vm.enter_isolation();

    let mut visitor = FreeVariableVisitor {
      vm: self.vm.clone(), // O(1) clone
      outer_names,         // O(1) clone
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

    // restore the stack state to where it was before exploring this function
    self.vm.exit();

    visitor.free_variables
  }
}

// read-only visitor that will discover free variables
impl Visit for FreeVariableVisitor {
  fn visit_ident(&mut self, ident: &Ident) {
    // all identifiers that are discovered by the visitor are assumed to be references
    match self.vm.lookup_ident(ident, Scope::Block) {
      None => {
        // didn't find the identifier in the isolated scope, this must be a free variable
        self.free_variables.insert(ident.to_id());
        // if self.outer_names.contains_key(&ident.sym) {
        //   // a captured free variable
        // } else {
        //   // assume as globally defined
        // }
      }
      _ => {}
    }
  }

  fn visit_arrow_expr(&mut self, arrow: &ArrowExpr) {
    self.vm.enter(Scope::Function);

    // hoist all of the parameters into the function scope (any functions within the default arguments can see all arguments)
    // a default arrow function can access any of the parameters
    // (a, b = () => a) => {}
    self.vm.bind_all_pats(&arrow.params, Scope::Function);

    match &arrow.body {
      BlockStmtOrExpr::Expr(expr) => expr.visit_with(self),
      BlockStmtOrExpr::BlockStmt(stmt) => stmt.visit_with(self),
    }

    self.vm.exit();
  }

  fn visit_function(&mut self, function: &Function) {
    self.vm.enter(Scope::Function);

    // hoist all of the parameters into the function scope (any functions within the default arguments ca)
    // a default arrow function can access any of the parameters
    // function foo(a, b = () => a) {}
    self.vm.bind_all_params(&function.params, Scope::Function);

    match &function.body {
      Some(body) => {
        // hoist all function and var declarations from the body and make them available to the Function scope
        // function foo (b = () => bar) {
        //   function bar() {}
        //            ^ visible to b = () => bar
        // }
        self.vm.bind_hoisted_stmts(&body.stmts, Scope::Function);

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

  fn visit_block_stmt(&mut self, block: &BlockStmt) {
    self.vm.enter(Scope::Block);

    block.visit_children_with(self);

    self.vm.exit();
  }
}
