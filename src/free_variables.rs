use crate::{closure_decorator::ClosureDecorator, lexical_scope::LexicalScope};
use swc_ecma_visit::Visit;
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
  pub fn discover_free_variables(
    &self,
    func: ArrowOrFunction,
    outer: &LexicalScope,
  ) -> Vec<FreeVariable> {
    match func {
      ArrowOrFunction::ArrowFunction(arrow) => {}
      ArrowOrFunction::Function(function) => {}
    }

    Vec::new()
  }
}
