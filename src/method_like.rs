use swc_common::Span;
use swc_core::ast::*;

pub trait MethodLike {
  fn function<'a>(&'a self) -> &'a Function;
  fn kind(&self) -> MethodKind;
  fn span<'a>(&'a self) -> Option<&'a Span>;
  fn key<'a>(&'a self) -> &'a PropName;
  fn is_static(&self) -> bool;
}

impl MethodLike for ClassMethod {
  #[inline]
  fn function<'a>(&'a self) -> &'a Function {
    &self.function
  }

  #[inline]
  fn kind(&self) -> MethodKind {
    self.kind
  }

  #[inline]
  fn span<'a>(&'a self) -> Option<&'a Span> {
    Some(&self.span)
  }

  #[inline]
  fn key<'a>(&'a self) -> &'a PropName {
    &self.key
  }

  #[inline]
  fn is_static(&self) -> bool {
    self.is_static
  }
}

impl MethodLike for MethodProp {
  #[inline]
  fn function<'a>(&'a self) -> &'a Function {
    &self.function
  }

  #[inline]
  fn kind(&self) -> MethodKind {
    MethodKind::Method
  }

  #[inline]
  fn span<'a>(&'a self) -> Option<&'a Span> {
    None
  }

  #[inline]
  fn key<'a>(&'a self) -> &'a PropName {
    &self.key
  }

  #[inline]
  fn is_static(&self) -> bool {
    false
  }
}
