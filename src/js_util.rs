use swc_common::DUMMY_SP;
use swc_plugin::ast::*;

use crate::ast::*;
use crate::parse::new_node;
use crate::span::get_expr_span;

pub fn str(str: &str) -> Box<Expr> {
  Box::new(Expr::Lit(Lit::Str(Str {
    raw: None,
    span: DUMMY_SP,
    value: JsWord::from(str),
  })))
}

pub fn number_u32(i: u32) -> Box<Expr> {
  number_f64(i as f64)
}

pub fn number_i32(i: i32) -> Box<Expr> {
  number_f64(i as f64)
}

pub fn number_f64(i: f64) -> Box<Expr> {
  Box::new(Expr::Lit(Lit::Num(Number {
    raw: None,
    span: DUMMY_SP,
    value: i,
  })))
}

pub fn ident_expr(ident: Ident) -> Box<Expr> {
  Box::new(Expr::Ident(ident))
}

pub fn prop_access_expr(expr: Box<Expr>, prop: Ident) -> Box<Expr> {
  Box::new(Expr::Member(MemberExpr {
    obj: expr,
    prop: MemberProp::Ident(prop),
    span: DUMMY_SP,
  }))
}

pub fn ref_expr(expr: Box<Expr>) -> Box<Expr> {
  let body = expr.as_ref().clone();
  let span = get_expr_span(&body);
  let pointer = arrow_pointer(expr);
  new_node(
    Node::ReferenceExpr,
    span,
    vec![str(""), pointer, number_i32(-1)],
  )
}

pub fn arrow_pointer(expr: Box<Expr>) -> Box<Expr> {
  Box::new(Expr::Arrow(ArrowExpr {
    span: get_expr_span(expr.as_ref()).clone(),
    body: BlockStmtOrExpr::Expr(expr),
    is_async: false,
    is_generator: false,
    params: vec![],
    return_type: None,
    type_params: None,
  }))
}

pub fn this_expr() -> Box<Expr> {
  Box::new(Expr::This(ThisExpr { span: DUMMY_SP }))
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
