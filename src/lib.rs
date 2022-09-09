use closure_decorator::ClosureDecorator;

use swc_core::{
  ast::Program,
  plugin::{plugin_transform, proxies::TransformPluginProgramMetadata},
  visit::VisitMutWith,
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
pub fn wrap_closures(mut program: Program, meta: TransformPluginProgramMetadata) -> Program {
  program.visit_mut_with(&mut ClosureDecorator::new(&meta.source_map));

  program
}
