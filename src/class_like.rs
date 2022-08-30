use swc_core::ast::*;

use crate::ast::Node;

/**
 * Defines a generic interface over AST [ClassDecl](ClassDecl) and [ClassExpr](ClassExpr).
 */
pub trait ClassLike {
  /**
   * Returns the corresponding [NodeKind](Node), either `Node::ClassExpr` or `Node::ClassExpr`.
   */
  fn kind(&self) -> Node;
  /**
   * Returns the Class's name. Optional because a ClassExpr may not have a name.
   */
  fn name<'a>(&'a self) -> Option<&'a Ident>;
  /**
   * Returns a mutable reference to the actual contents of the [Class](Class) (shared by both ClassExpr and ClassDecl structs).
   */
  fn class_mut<'a>(&'a mut self) -> &'a mut Class;

  /**
   * Returns an immutable reference to the actual contents of the [Class](Class) (shared by both ClassExpr and ClassDecl structs).
   */
  fn class<'a>(&'a self) -> &'a Class;
}

impl ClassLike for ClassDecl {
  fn kind(&self) -> Node {
    Node::ClassDecl
  }

  fn name<'a>(&'a self) -> Option<&'a Ident> {
    Some(&self.ident)
  }

  fn class_mut<'a>(&'a mut self) -> &'a mut Class {
    &mut self.class
  }

  fn class<'a>(&'a self) -> &'a Class {
    &self.class
  }
}

impl ClassLike for ClassExpr {
  fn kind(&self) -> Node {
    Node::ClassExpr
  }

  fn name<'a>(&'a self) -> Option<&'a Ident> {
    self.ident.as_ref()
  }

  fn class_mut<'a>(&'a mut self) -> &'a mut Class {
    &mut self.class
  }

  fn class<'a>(&'a self) -> &'a Class {
    &self.class
  }
}
