use closure_decorator::ClosureDecorator;
use swc_common::Mark;
use swc_ecma_visit::{Fold, VisitMut};
use swc_plugin::{ast::*, plugin_transform, TransformPluginProgramMetadata};
use to_functionless_ast::parse_closure;

mod closure_decorator;
mod free_variables;
mod to_functionless_ast;
mod virtual_machine;

#[plugin_transform]
pub fn wrap_closures(mut program: Program, _metadata: TransformPluginProgramMetadata) -> Program {
  program.visit_mut_with(&mut ClosureDecorator::new());

  parse_closure();
  program
}

pub fn wrap(_top_level_mark: Mark) -> impl Fold + VisitMut {
  as_folder(ClosureDecorator::new())
}
