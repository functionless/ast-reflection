use closure_decorator::ClosureDecorator;
use swc_common::Mark;

use swc_core::{
  ast::Program,
  plugin::{plugin_transform, proxies::TransformPluginProgramMetadata},
  visit::{as_folder, Fold, VisitMut, VisitMutWith},
};

mod ast;
mod class_like;
mod closure_decorator;
mod js_util;
mod method_like;
mod parse;
mod prepend;
mod span;
mod virtual_machine;

#[plugin_transform]
pub fn wrap_closures(mut program: Program, _: TransformPluginProgramMetadata) -> Program {
  program.visit_mut_with(&mut ClosureDecorator::new());

  program
}

pub fn wrap(_top_level_mark: Mark) -> impl Fold + VisitMut {
  as_folder(ClosureDecorator::new())
}
