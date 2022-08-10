use swc_common::DUMMY_SP;
use swc_plugin::ast::*;

pub fn str(str: &str) -> Box<Expr> {
  Box::new(Expr::Lit(Lit::Str(Str {
    raw: None,
    span: DUMMY_SP,
    value: JsWord::from(str),
  })))
}

pub fn num(i: u32) -> Box<Expr> {
  Box::new(Expr::Lit(Lit::Num(Number {
    raw: None,
    span: DUMMY_SP,
    value: i as u32 as f64,
  })))
}

pub fn bool_expr(value: bool) -> Box<Expr> {
  Box::new(Expr::Lit(Lit::Bool(Bool {
    span: DUMMY_SP,
    value,
  })))
}

pub fn true_expr() -> Box<Expr> {
  bool_expr(true)
}

pub fn false_expr() -> Box<Expr> {
  bool_expr(false)
}

pub fn undefined_expr() -> Box<Expr> {
  Box::new(Expr::Ident(Ident {
    optional: false,
    span: DUMMY_SP,
    sym: JsWord::from("undefined"),
  }))
}

pub fn empty_array_expr() -> Box<Expr> {
  Box::new(Expr::Array(ArrayLit {
    elems: vec![],
    span: DUMMY_SP,
  }))
}
