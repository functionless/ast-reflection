use std::collections::HashSet;

use crate::{closure_decorator::ClosureDecorator, virtual_machine::VirtualMachine};
use swc_common::SyntaxContext;
use swc_plugin::ast::*;

pub struct FreeVariableVisitor {
  /**
   * A VM instance for maintaining lexical scope while walking through the closure's contents.
   */
  vm: VirtualMachine,
  /**
   * A HashSet of discovered free variables.
   */
  free_variables: HashSet<Id>,
  /**
   * Whether the visitor is currently in a function (or arrow function).
   *
   * Used to identify whether the `this` keyword is a free variable.
   */
  in_function: bool,
}

impl ClosureDecorator {
  pub fn discover_free_variables<T>(&mut self, node: &T) -> Vec<Id>
  where
    T: VisitWith<FreeVariableVisitor>,
  {
    let mut visitor = FreeVariableVisitor {
      vm: VirtualMachine::new(),
      // outer_names, // O(1) clone
      free_variables: HashSet::new(),
      in_function: false,
    };

    node.visit_with(&mut visitor);

    let mut seen: HashSet<Id> = HashSet::new();

    // TODO: more efficient way of .distinct()
    let mut free_variables = visitor
      .free_variables
      .into_iter()
      .filter(|v| {
        if seen.contains(v) {
          false
        } else {
          seen.insert(v.clone());
          true
        }
      })
      .collect::<Vec<Id>>();

    free_variables.sort_by(|a, b| a.0.cmp(&b.0));
    free_variables
  }
}

// read-only visitor that will discover free variables
impl Visit for FreeVariableVisitor {
  fn visit_constructor(&mut self, constructor: &Constructor) {
    self.vm.bind_all_constructor_params(&constructor.params);
    constructor.params.visit_with(self);
    constructor.body.visit_with(self);
  }

  fn visit_pat(&mut self, pat: &Pat) {
    match pat {
      // only look for free variables in the assigned value
      // identifiers found in destructuring syntax must be ignored
      Pat::Assign(assign) => assign.right.as_ref().visit_with(self),
      _ => {}
    }
  }

  fn visit_class_prop(&mut self, class_member: &ClassProp) {
    match &class_member.key {
      PropName::Computed(expr) => {
        expr.visit_with(self);
      }
      _ => {}
    }
    class_member.value.visit_with(self);
  }

  fn visit_class_method(&mut self, method: &ClassMethod) {
    method.function.visit_with(self);
  }

  fn visit_arrow_expr(&mut self, arrow: &ArrowExpr) {
    self.vm.enter();

    let prev_in_function = self.in_function;

    self.in_function = false;

    self.vm.bind_all_pats(&arrow.params);

    arrow.params.visit_with(self);

    match &arrow.body {
      BlockStmtOrExpr::Expr(expr) => {
        expr.visit_with(self);
      }
      BlockStmtOrExpr::BlockStmt(block) => {
        self.vm.bind_block(block);
        block.visit_children_with(self);
      }
    }

    self.in_function = prev_in_function;

    self.vm.exit();
  }

  fn visit_function(&mut self, function: &Function) {
    self.vm.enter();

    let prev_in_function = self.in_function;

    self.in_function = true;

    // greedily bind all of the parameters into the block scope (any functions within the default arguments ca)
    // a default arrow function can access any of the parameters
    // function foo(a, b = () => a) {}
    self.vm.bind_all_params(&function.params);

    function.params.visit_with(self);

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
          self.vm.bind_param(param);

          // evaluate the pattern which may contain a default expression
          param.visit_with(self);
        });

        // step into the body and evaluate all statements
        // skip the BlockStmt (i.e. `body.visit_with(self)` because we have already created a function scope)
        body.visit_children_with(self);
      }
      _ => {}
    }

    self.in_function = prev_in_function;

    self.vm.exit();
  }

  fn visit_block_stmt(&mut self, block: &BlockStmt) {
    self.vm.enter();

    self.vm.bind_block(block);

    block.visit_children_with(self);

    self.vm.exit();
  }

  fn visit_this_expr(&mut self, _this: &ThisExpr) {
    if !self.in_function {
      // if we're in an arrow function, then `this` is a free variable
      self
        .free_variables
        .insert((js_word!("this"), SyntaxContext::from_u32(0)));
    }
  }

  fn visit_var_declarator(&mut self, var: &VarDeclarator) {
    self.vm.bind_pat(&var.name);

    match &var.init {
      Some(init) => {
        init.as_ref().visit_with(self);
      }
      _ => {}
    }
  }

  fn visit_member_expr(&mut self, member: &MemberExpr) {
    member.obj.as_ref().visit_with(self);

    match &member.prop {
      MemberProp::Computed(c) => {
        // computed properties may contain free variables
        c.visit_with(self);
      }
      _ => {}
    }
  }

  fn visit_ident(&mut self, ident: &Ident) {
    // all identifiers that are discovered by the visitor are assumed to be references
    // because of the order in which we traverse
    if self.vm.is_id_visible(ident) {
      self.free_variables.insert(ident.to_id());
    }
  }
}
