// unstable API, but is the most efficient way to filter a vec in place
// https://github.com/rust-lang/rust/issues/43244
#![feature(drain_filter)]

use closure_decorator::ClosureDecorator;
use swc_common::Mark;
use swc_ecma_visit::{Fold, VisitMut};
use swc_plugin::{ast::*, plugin_transform, TransformPluginProgramMetadata};

mod closure_decorator;
mod free_variables;
mod virtual_machine;

#[plugin_transform]
pub fn wrap_closures(mut program: Program, _metadata: TransformPluginProgramMetadata) -> Program {
  program.visit_mut_with(&mut ClosureDecorator::new());

  program
}

pub fn wrap(top_level_mark: Mark) -> impl Fold + VisitMut {
  as_folder(ClosureDecorator::new())
}
