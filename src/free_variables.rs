use crate::{closure_decorator::ClosureDecorator, lexical_scope::Scope};
use swc_plugin::ast::*;

pub enum ArrowOrFunction<'a> {
  ArrowFunction(&'a ArrowExpr),
  Function(&'a Function),
}

pub struct FreeVariable {
  pub name: JsWord,
  pub id: u32,
}

impl ClosureDecorator {
  pub fn discover_free_variables(&mut self, func: ArrowOrFunction) -> Vec<FreeVariable> {
    // store the state of the outer function scope
    let outer_scope = self.get_names(Scope::Block);

    // push an empty scope onto the stack
    // any variable we encounter that is not in the isolated scope must be a free variable
    self.enter_isolation();

    match func {
      ArrowOrFunction::ArrowFunction(arrow) => {
        self.bind_pats(&arrow.params);
      }
      ArrowOrFunction::Function(function) => {
        self.bind_params(&function.params);
      }
    };

    // restore the stack state to where it was before exploring this function
    self.exit();

    Vec::new()
  }
}

// read-only visitor that will discover free variables
impl Visit for ClosureDecorator {
  // TODO
}
