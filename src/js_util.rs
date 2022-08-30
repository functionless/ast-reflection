use swc_atoms::JsWord;
use swc_common::source_map::Pos;
use swc_common::{BytePos, Span, SyntaxContext, DUMMY_SP};
use swc_core::ast::*;
use swc_core::utils::quote_ident;

use crate::ast::*;
use crate::parse::new_node;
use crate::span::get_expr_span;

#[inline]
pub fn ident_expr(ident: Ident) -> Box<Expr> {
  Box::new(Expr::Ident(ident))
}

#[inline]
pub fn require_expr(module: &str) -> Box<Expr> {
  Box::new(Expr::Call(CallExpr {
    type_args: None,
    span: DUMMY_SP,
    callee: Callee::Expr(ident_expr(quote_ident!("require"))),
    args: vec![ExprOrSpread {
      expr: string_expr(module),
      spread: None,
    }],
  }))
}

#[inline]
pub fn string_expr(str: &str) -> Box<Expr> {
  Box::new(Expr::Lit(Lit::Str(Str {
    raw: None,
    span: DUMMY_SP,
    value: JsWord::from(str),
  })))
}

#[inline]
pub fn type_of(expr: Box<Expr>) -> Box<Expr> {
  Box::new(Expr::Unary(UnaryExpr {
    op: UnaryOp::TypeOf,
    arg: expr,
    span: DUMMY_SP,
  }))
}

#[inline]
pub fn not_eq_eq(left: Box<Expr>, right: Box<Expr>) -> Box<Expr> {
  Box::new(Expr::Bin(BinExpr {
    left,
    op: BinaryOp::NotEqEq,
    right,
    span: DUMMY_SP,
  }))
}

#[inline]
pub fn number_u32(i: u32) -> Box<Expr> {
  number_f64(i as f64)
}

#[inline]
pub fn number_i32(i: i32) -> Box<Expr> {
  number_f64(i as f64)
}

#[inline]
pub fn number_f64(i: f64) -> Box<Expr> {
  Box::new(Expr::Lit(Lit::Num(Number {
    raw: None,
    span: DUMMY_SP,
    value: i,
  })))
}

#[inline]
pub fn prop_access_expr(expr: Box<Expr>, prop: Ident) -> Box<Expr> {
  Box::new(Expr::Member(MemberExpr {
    obj: expr,
    prop: MemberProp::Ident(prop),
    span: DUMMY_SP,
  }))
}

#[inline]
pub fn ref_expr(expr: Box<Expr>) -> Box<Expr> {
  let body = expr.as_ref().clone();
  let span = get_expr_span(&body);
  let pointer = arrow_pointer(expr);
  new_node(
    Node::ReferenceExpr,
    span,
    vec![string_expr(""), pointer, number_i32(-1)],
  )
}

#[inline]
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

#[inline]
pub fn this_expr() -> Box<Expr> {
  Box::new(Expr::This(ThisExpr { span: DUMMY_SP }))
}

#[inline]
pub fn bool_expr(value: bool) -> Box<Expr> {
  Box::new(Expr::Lit(Lit::Bool(Bool {
    span: DUMMY_SP,
    value,
  })))
}

#[inline]
pub fn true_expr() -> Box<Expr> {
  bool_expr(true)
}

#[inline]
pub fn false_expr() -> Box<Expr> {
  bool_expr(false)
}

#[inline]
pub fn undefined_expr() -> Box<Expr> {
  Box::new(Expr::Ident(Ident {
    optional: false,
    span: DUMMY_SP,
    sym: JsWord::from("undefined"),
  }))
}

#[inline]
pub fn empty_array_expr() -> Box<Expr> {
  Box::new(Expr::Array(ArrayLit {
    elems: vec![],
    span: DUMMY_SP,
  }))
}

#[inline]
pub fn __filename() -> Box<Expr> {
  Box::new(Expr::Ident(Ident {
    optional: false,
    span: empty_span(),
    sym: JsWord::from("__filename"),
  }))
}

#[inline]
pub fn empty_span() -> Span {
  Span {
    ctxt: SyntaxContext::from_u32(0),
    hi: BytePos::from_u32(0),
    lo: BytePos::from_u32(0),
  }
}
