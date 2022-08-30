use swc_common::Span;
use swc_core::ast::*;

pub fn get_expr_span<'a>(expr: &'a Expr) -> &'a Span {
  match expr {
    Expr::Array(e) => &e.span,
    Expr::Arrow(e) => &e.span,
    Expr::Assign(e) => &e.span,
    Expr::Await(e) => &e.span,
    Expr::Bin(e) => &e.span,
    Expr::Call(e) => &e.span,
    Expr::Class(e) => &e.class.span,
    Expr::Cond(e) => &e.span,
    Expr::Fn(e) => &e.function.span,
    Expr::Ident(e) => &e.span,
    Expr::Invalid(e) => &e.span,
    Expr::This(e) => &e.span,
    Expr::Object(e) => &e.span,
    Expr::Unary(e) => &e.span,
    Expr::Update(e) => &e.span,
    Expr::Member(e) => &e.span,
    Expr::SuperProp(e) => &e.span,
    Expr::New(e) => &e.span,
    Expr::Seq(e) => &e.span,
    Expr::Lit(e) => get_lit_span(e),
    Expr::Tpl(e) => &e.span,
    Expr::TaggedTpl(e) => &e.span,
    Expr::Yield(e) => &e.span,
    Expr::MetaProp(e) => &e.span,
    Expr::Paren(e) => &e.span,
    Expr::JSXMember(e) => get_jsx_object_span(&e.obj), // TODO: combine spans? this is wrong, why don't these nodes have spans?
    Expr::JSXNamespacedName(e) => &e.name.span, // TODO: combine spans? this is wrong, why don't these nodes have spans?
    Expr::JSXEmpty(e) => &e.span,
    Expr::JSXElement(e) => &e.span,
    Expr::JSXFragment(e) => &e.span,
    Expr::TsTypeAssertion(e) => &e.span,
    Expr::TsConstAssertion(e) => &e.span,
    Expr::TsNonNull(e) => &e.span,
    Expr::TsAs(e) => &e.span,
    Expr::TsInstantiation(e) => &e.span,
    Expr::PrivateName(e) => &e.span,
    Expr::OptChain(e) => &e.span,
  }
}

pub fn get_stmt_span<'a>(stmt: &'a Stmt) -> &'a Span {
  match stmt {
    Stmt::Block(s) => &s.span,
    Stmt::Empty(s) => &s.span,
    Stmt::Debugger(s) => &s.span,
    Stmt::With(s) => &s.span,
    Stmt::Return(s) => &s.span,
    Stmt::Labeled(s) => &s.span,
    Stmt::Break(s) => &s.span,
    Stmt::Continue(s) => &s.span,
    Stmt::If(s) => &s.span,
    Stmt::Switch(s) => &s.span,
    Stmt::Throw(s) => &s.span,
    Stmt::Try(s) => &s.span,
    Stmt::While(s) => &s.span,
    Stmt::DoWhile(s) => &s.span,
    Stmt::For(s) => &s.span,
    Stmt::ForIn(s) => &s.span,
    Stmt::ForOf(s) => &s.span,
    Stmt::Decl(s) => get_decl_span(s),
    Stmt::Expr(s) => &s.span,
  }
}

pub fn get_decl_span<'a>(decl: &'a Decl) -> &'a Span {
  match decl {
    Decl::Class(d) => &d.class.span,
    Decl::Fn(d) => &d.function.span,
    Decl::Var(d) => &d.span,
    Decl::TsInterface(d) => &d.span,
    Decl::TsTypeAlias(d) => &d.span,
    Decl::TsEnum(d) => &d.span,
    Decl::TsModule(d) => &d.span,
  }
}

pub fn get_lit_span<'a>(lit: &'a Lit) -> &'a Span {
  match lit {
    Lit::Str(l) => &l.span,
    Lit::Bool(l) => &l.span,
    Lit::Null(l) => &l.span,
    Lit::Num(l) => &l.span,
    Lit::BigInt(l) => &l.span,
    Lit::Regex(l) => &l.span,
    Lit::JSXText(l) => &l.span,
  }
}

pub fn get_pat_span<'a>(pat: &'a Pat) -> &'a Span {
  match pat {
    Pat::Array(a) => &a.span,
    Pat::Assign(a) => &a.span,
    Pat::Expr(e) => get_expr_span(e),
    Pat::Ident(i) => &i.span,
    Pat::Invalid(i) => &i.span,
    Pat::Object(o) => &o.span,
    Pat::Rest(r) => &r.span,
  }
}

pub fn get_prop_name_span<'a>(name: &'a PropName) -> &'a Span {
  match name {
    PropName::Ident(n) => &n.span,
    PropName::Str(n) => &n.span,
    PropName::Num(n) => &n.span,
    PropName::Computed(n) => &n.span,
    PropName::BigInt(n) => &n.span,
  }
}

pub fn get_jsx_object_span<'a>(obj: &'a JSXObject) -> &'a Span {
  match obj {
    JSXObject::JSXMemberExpr(e) => &e.prop.span,
    JSXObject::Ident(i) => &i.span,
  }
}

pub fn concat_span(left: &Span, right: &Span) -> Span {
  Span {
    lo: left.lo,
    hi: right.hi,
    ctxt: left.ctxt.clone(),
  }
}
