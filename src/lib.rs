use closure_decorator::ClosureDecorator;
use swc_common::Mark;
use swc_ecma_visit::{Fold, VisitMut};
use swc_plugin::{ast::*, plugin_transform, TransformPluginProgramMetadata};

mod ast;
mod closure_decorator;
mod free_variables;
mod parse;
mod virtual_machine;

#[plugin_transform]
pub fn wrap_closures(mut program: Program, _metadata: TransformPluginProgramMetadata) -> Program {
  program.visit_mut_with(&mut ClosureDecorator::new());

  program
}

pub fn wrap(_top_level_mark: Mark) -> impl Fold + VisitMut {
  as_folder(ClosureDecorator::new())
}
