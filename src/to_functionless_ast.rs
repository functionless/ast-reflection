use core::panic;

use strum_macros::AsRefStr;
use swc_common::DUMMY_SP;
use swc_plugin::{
  ast::*,
  utils::{private_ident, quote_ident},
};

pub struct ToAST {
  functionless_import_ident: Ident,
}

const EMPTY_VEC: Vec<Box<Expr>> = vec![];

const True: Expr = Expr::Lit(Lit::Bool(Bool {
  span: DUMMY_SP,
  value: true,
}));

const False: Expr = Expr::Lit(Lit::Bool(Bool {
  span: DUMMY_SP,
  value: false,
}));

fn undefined() -> Expr {
  Expr::Ident(Ident {
    optional: false,
    span: DUMMY_SP,
    sym: JsWord::from("undefined"),
  })
}

impl ToAST {
  fn new() -> ToAST {
    ToAST {
      functionless_import_ident: private_ident!("functionless"),
    }
  }
}

pub fn parse_decl(decl: &Decl, import: &Ident) -> Box<Expr> {
  match decl {
    Decl::Class(class_decl) => Node::new(
      Node::ClassDecl,
      vec![
        //
        parse_ident(&class_decl.ident, import),
        class_decl
          .class
          .super_class
          .as_ref()
          .map(|sup| parse_expr(sup.as_ref(), import))
          .unwrap_or(Box::new(undefined())),
        Box::new(Expr::Array(ArrayLit {
          elems: class_decl
            .class
            .body
            .iter()
            .map(|member| {
              parse_class_member(member, import).map(|expr| ExprOrSpread { expr, spread: None })
            })
            .collect(),
          span: DUMMY_SP,
        })),
      ],
      import,
    ),
    Decl::Fn(function) => Node::new(
      Node::FunctionDecl,
      vec![Box::new(Expr::Array(ArrayLit {
        span: DUMMY_SP,
        elems: function
          .function
          .params
          .iter()
          .map(|param| {
            Some(ExprOrSpread {
              expr: Node::new(
                Node::ParameterDecl,
                vec![parse_pattern(&param.pat, import)],
                import,
              ),
              spread: None,
            })
          })
          .collect(),
      }))],
      import,
    ),
    Decl::TsEnum(ts_enum) => panic!("enums not supported"),
    Decl::TsInterface(interface) => panic!("interface not supported"),
    Decl::TsModule(module) => panic!("module declarations not supported"),
    Decl::TsTypeAlias(type_alias) => panic!("type alias not supported"),
    Decl::Var(var_decl) => parse_var_decl(var_decl, import),
  }
}

fn parse_stmt(stmt: &Stmt, import: &Ident) -> Box<Expr> {
  match stmt {
    Stmt::Block(block) => parse_block(block, import),
    Stmt::Break(break_stmt) => Node::new(Node::BreakStmt, EMPTY_VEC, import),
    Stmt::Continue(continue_stmt) => Node::new(Node::ContinueStmt, EMPTY_VEC, import),
    Stmt::Debugger(debugger) => Node::new(Node::DebuggerStmt, EMPTY_VEC, import),
    Stmt::Decl(decl) => parse_decl(decl, import),
    Stmt::DoWhile(do_while) => Node::new(
      Node::WhileStmt,
      vec![
        // condition
        parse_expr(do_while.test.as_ref(), import),
        // block
        parse_stmt(do_while.body.as_ref(), import),
      ],
      import,
    ),
    Stmt::Empty(empty) => Node::new(Node::EmptyStmt, EMPTY_VEC, import),
    Stmt::Expr(expr_stmt) => Node::new(
      Node::ExprStmt,
      vec![
        //expr
        parse_expr(expr_stmt.expr.as_ref(), import),
      ],
      import,
    ),
    // TODO
    Stmt::For(for_stmt) => Node::new(
      Node::ForStmt,
      vec![
        parse_stmt(for_stmt.body.as_ref(), import),
        for_stmt
          .init
          .as_ref()
          .map(|init| match init {
            VarDeclOrExpr::Expr(expr) => parse_expr(expr.as_ref(), import),
            VarDeclOrExpr::VarDecl(var) => parse_var_decl(&var, import),
          })
          .unwrap_or(Box::new(undefined())),
      ],
      import,
    ),
    // for (const left in right)
    Stmt::ForIn(for_in) => Node::new(
      Node::ForInStmt,
      vec![
        match &for_in.left {
          VarDeclOrPat::VarDecl(var_decl) => parse_var_decl(var_decl, import),
          VarDeclOrPat::Pat(pat) => parse_pattern(pat, import),
        },
        parse_expr(&for_in.right, import),
        parse_stmt(for_in.body.as_ref(), import),
      ],
      import,
    ),
    // for (const left of right)
    Stmt::ForOf(for_of) => Node::new(
      Node::ForInStmt,
      vec![
        match &for_of.left {
          VarDeclOrPat::VarDecl(var_decl) => parse_var_decl(var_decl, import),
          VarDeclOrPat::Pat(pat) => parse_pattern(pat, import),
        },
        parse_expr(&for_of.right, import),
        parse_stmt(for_of.body.as_ref(), import),
      ],
      import,
    ),
    Stmt::If(if_stmt) => Node::new(
      Node::IfStmt,
      vec![
        // when
        parse_expr(&if_stmt.test, import),
        // then
        parse_stmt(if_stmt.cons.as_ref(), import),
        // else
        if_stmt
          .alt
          .as_ref()
          .map(|alt| parse_stmt(alt.as_ref(), import))
          .unwrap_or(Box::new(undefined())),
      ],
      import,
    ),
    // https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Statements/label
    // for now, we just erase the label
    Stmt::Labeled(labelled) => Node::new(
      Node::LabelledStmt,
      vec![parse_stmt(&labelled.body, import)],
      import,
    ),
    Stmt::Return(return_stmt) => Node::new(
      Node::ReturnStmt,
      vec![match return_stmt.arg.as_ref() {
        Some(arg) => parse_expr(&arg, import),
        // encode an empty `return;` as `return undefined();`
        None => Node::new(Node::UndefinedLiteralExpr, EMPTY_VEC, import),
      }],
      import,
    ),
    // TODO: support switch - https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Statements/switch
    Stmt::Switch(switch) => Node::new(
      Node::SwitchStmt,
      vec![
        parse_expr(&switch.discriminant, import),
        // case
        Box::new(Expr::Array(ArrayLit {
          span: DUMMY_SP,
          elems: switch
            .cases
            .iter()
            .map(|case| {
              let stmts: Vec<Box<Expr>> = case
                .cons
                .iter()
                .map(|stmt| parse_stmt(stmt, import))
                .collect();
              //
              Some(ExprOrSpread {
                expr: match case.test.as_ref() {
                  Some(test) => Node::new(Node::CaseClause, stmts.clone(), import),
                  None => Node::new(Node::DefaultClause, stmts.clone(), import),
                },
                spread: None,
              })
            })
            .collect(),
        })),
      ],
      import,
    ),
    Stmt::Throw(throw) => Node::new(
      Node::ThrowStmt,
      vec![parse_expr(throw.arg.as_ref(), import)],
      import,
    ),
    Stmt::Try(try_stmt) => Node::new(
      Node::TryStmt,
      vec![
        parse_block(&try_stmt.block, import),
        try_stmt
          .handler
          .as_ref()
          .map(|catch| match &catch.param {
            Some(pat) => parse_pattern(pat, import),
            None => Box::new(undefined()),
          })
          .unwrap_or(Box::new(undefined())),
        try_stmt
          .finalizer
          .as_ref()
          .map(|finalizer| parse_block(&finalizer, import))
          .unwrap_or(Box::new(undefined())),
      ],
      import,
    ),
    Stmt::While(while_stmt) => Node::new(
      Node::WhileStmt,
      vec![
        parse_expr(while_stmt.test.as_ref(), import),
        parse_stmt(while_stmt.body.as_ref(), import),
      ],
      import,
    ),
    // TODO: support with https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Statements/with
    Stmt::With(with) => Node::new(
      Node::WithStmt,
      vec![
        parse_expr(with.obj.as_ref(), import),
        parse_stmt(with.body.as_ref(), import),
      ],
      import,
    ),
  }
}

fn parse_expr(expr: &Expr, import: &Ident) -> Box<Expr> {
  match expr {
    Expr::Array(array) => Node::new(
      Node::ArrayLiteralExpr,
      array
        .elems
        .iter()
        .filter_map(|element| {
          element
            .as_ref()
            .map(|e| parse_expr(e.expr.as_ref(), import))
        })
        .collect(),
      import,
    ),
    Expr::Arrow(arrow) => Node::new(
      Node::ArrowFunctionExpr,
      vec![
        // params
        parse_patterns(&arrow.params, import),
      ],
      import,
    ),
    Expr::Assign(assign) => Node::new(
      Node::BinaryExpr,
      vec![
        match &assign.left {
          PatOrExpr::Expr(expr) => parse_expr(expr, import),
          PatOrExpr::Pat(pat) => parse_pattern(pat, import),
        },
        Box::new(Expr::Lit(Lit::Str(Str {
          raw: None,
          span: DUMMY_SP,
          value: match assign.op {
            Assign => JsWord::from("="),
            AddAssign => JsWord::from("+="),
            SubAssign => JsWord::from("-="),
            MulAssign => JsWord::from("*="),
            DivAssign => JsWord::from("/="),
            ModAssign => JsWord::from("%="),
            LShiftAssign => JsWord::from("<<="),
            RShiftAssign => JsWord::from(">>="),
            ZeroFillRShiftAssign => JsWord::from(">>>="),
            BitOrAssign => JsWord::from("|="),
            BitXorAssign => JsWord::from("^="),
            BitAndAssign => JsWord::from("&="),
            ExpAssign => JsWord::from("**="),
            AndAssign => JsWord::from("&&="),
            OrAssign => JsWord::from("||="),
            NullishAssign => JsWord::from("??="),
          },
        }))),
        parse_expr(assign.right.as_ref(), import),
      ],
      import,
    ),
    Expr::Await(a_wait) => Node::new(
      Node::AwaitExpr,
      vec![parse_expr(a_wait.arg.as_ref(), import)],
      import,
    ),
    Expr::Bin(binary_op) => Node::new(
      Node::BinaryExpr,
      vec![
        parse_expr(binary_op.left.as_ref(), import),
        Box::new(Expr::Lit(Lit::Str(Str {
          span: DUMMY_SP,
          raw: None,
          value: match binary_op.op {
            Add => JsWord::from("+"),
            BitAnd => JsWord::from("&"),
            BitOr => JsWord::from("|"),
            BitXor => JsWord::from("^"),
            Div => JsWord::from("/"),
            EqEq => JsWord::from("=="),
            EqEqEq => JsWord::from("==="),
            Exp => JsWord::from("**"),
            Gt => JsWord::from(">"),
            GtEq => JsWord::from(">="),
            In => JsWord::from("in"),
            InstanceOf => JsWord::from("instanceof"),
            LogicalAnd => JsWord::from("&&"),
            LogicalOr => JsWord::from("||"),
            LShift => JsWord::from("<<"),
            Lt => JsWord::from("<"),
            LtEq => JsWord::from("<="),
            Mod => JsWord::from("%"),
            Mul => JsWord::from("*"),
            NotEq => JsWord::from("!="),
            NotEqEq => JsWord::from("!=="),
            NullishCoalescing => JsWord::from("??"),
            RShift => JsWord::from(">>"),
            Sub => JsWord::from("-"),
            ZeroFillRShift => JsWord::from(">>>"),
          },
        }))),
        parse_expr(binary_op.right.as_ref(), import),
      ],
      import,
    ),
    Expr::Call(call) => parse_callee(&call.callee, &call.args, false, import),
    // TODO: extract properties from ts-parameters
    Expr::Class(class_expr) => Node::new(
      Node::ClassExpr,
      vec![
        //
        class_expr
          .ident
          .as_ref()
          .map(|i| parse_ident(&i, import))
          .unwrap_or(Box::new(undefined())),
        class_expr
          .class
          .super_class
          .as_ref()
          .map(|sup| parse_expr(sup.as_ref(), import))
          .unwrap_or(Box::new(undefined())),
        Box::new(Expr::Array(ArrayLit {
          elems: class_expr
            .class
            .body
            .iter()
            .map(|member| {
              parse_class_member(member, import).map(|expr| ExprOrSpread { expr, spread: None })
            })
            .collect(),
          span: DUMMY_SP,
        })),
      ],
      import,
    ),
    Expr::Cond(cond) => Node::new(
      Node::ConditionExpr,
      vec![
        // when
        parse_expr(&cond.test.as_ref(), import),
        // then
        parse_expr(&cond.cons.as_ref(), import),
        // else
        parse_expr(&cond.alt.as_ref(), import),
      ],
      import,
    ),
    Expr::Fn(function_expr) => Node::new(
      Node::FunctionExpr,
      vec![
        // params
        parse_params(&function_expr.function.params, import),
      ],
      import,
    ),
    // TODO: check if is in scope and convert to a ReferenceExpr
    Expr::Ident(id) => parse_ident(id, import),
    Expr::Invalid(invalid) => Node::error("Syntax Error", import),
    Expr::JSXElement(jsx_element) => Node::error("not sure what to do with JSXElement", import),
    Expr::JSXEmpty(jsx_empty) => Node::error("not sure what to do with JSXEmpty", import),
    Expr::JSXFragment(jsx_fragment) => Node::error("not sure what to do with JSXFragment", import),
    Expr::JSXMember(jsx_member) => Node::error("not sure what to do with JSXMember", import),
    Expr::JSXNamespacedName(jsx_namespace_name) => {
      Node::error("not sure what to do with JSXNamespacedName", import)
    }
    Expr::Lit(literal) => match &literal {
      // not sure what type of node this is, will just error for now
      Lit::JSXText(_) => Node::error("not sure what to do with JSXText", import),
      _ => Node::new(
        match literal {
          Lit::Bool(_) => Node::BooleanLiteralExpr,
          Lit::BigInt(_) => Node::BigIntExpr,
          Lit::Null(_) => Node::NullLiteralExpr,
          Lit::Num(_) => Node::NumberLiteralExpr,
          Lit::Regex(_) => Node::RegexExpr,
          Lit::Str(_) => Node::StringLiteralExpr,
          // impossible to reach here
          Lit::JSXText(text) => panic!("not sure what to do with JSXText"),
        },
        vec![Box::new(expr.clone())],
        import,
      ),
    },
    Expr::Member(member) => parse_member(member, false, import),
    Expr::MetaProp(meta_prop) => Node::error("MetaProp is not supported", import),
    Expr::New(call) => Node::new(
      Node::NewExpr,
      vec![
        //
        parse_expr(&call.callee, import),
        Box::new(Expr::Array(ArrayLit {
          span: DUMMY_SP,
          elems: call
            .args
            .as_ref()
            .map(|args| {
              args
                .iter()
                .map(|arg| {
                  Some(ExprOrSpread {
                    spread: None,
                    expr: if arg.spread.is_some() {
                      // TODO: we can't represent this right now
                      Node::new(Node::Argument, vec![], import)
                    } else {
                      Node::new(Node::Argument, vec![], import)
                    },
                  })
                })
                .collect()
            })
            .unwrap_or(vec![]),
        })),
      ],
      import,
    ),
    Expr::Object(object) => Node::new(
      Node::ObjectLiteralExpr,
      object
        .props
        .iter()
        .map(|prop| match prop {
          PropOrSpread::Prop(prop) => match prop.as_ref() {
            // invalid according to SWC's docs on Prop::Assign
            Prop::Assign(assign) => panic!("Invalid Syntax in Object Literal"),
            Prop::Getter(getter) => Node::new(
              Node::GetAccessorDecl,
              vec![
                parse_prop_name(&getter.key, import),
                parse_block(&getter.body.as_ref().unwrap(), import),
              ],
              import,
            ),
            Prop::KeyValue(assign) => Node::new(
              Node::PropAssignExpr,
              vec![
                parse_prop_name(&assign.key, import),
                parse_expr(assign.value.as_ref(), import),
              ],
              import,
            ),
            Prop::Method(method) => Node::new(
              Node::MethodDecl,
              vec![
                //
                parse_prop_name(&method.key, import),
                parse_params(&method.function.params, import),
                parse_block(method.function.body.as_ref().unwrap(), import),
              ],
              import,
            ),
            Prop::Setter(setter) => Node::new(
              Node::SetAccessorDecl,
              vec![
                parse_prop_name(&setter.key, import),
                parse_pattern(&setter.param, import),
                parse_block(setter.body.as_ref().unwrap(), import),
              ],
              import,
            ),
            Prop::Shorthand(ident) => Node::new(
              Node::PropAssignExpr,
              vec![parse_ident(ident, import), parse_ident(ident, import)],
              import,
            ),
          },
          PropOrSpread::Spread(spread) => Node::new(
            Node::SpreadAssignExpr,
            vec![parse_expr(spread.expr.as_ref(), import)],
            import,
          ),
        })
        .collect(),
      import,
    ),
    Expr::OptChain(opt_chain) => match &opt_chain.base {
      OptChainBase::Call(call) => parse_call_expr(&call.callee, &call.args, true, import),
      OptChainBase::Member(member) => parse_member(&member, true, import),
    },
    Expr::Paren(paren) => Node::new(
      Node::ParenthesizedExpr,
      vec![parse_expr(paren.expr.as_ref(), import)],
      import,
    ),
    Expr::PrivateName(private_name) => Node::new(
      Node::PrivateIdentifier,
      vec![Box::new(Expr::Lit(Lit::Str(Str {
        raw: None,
        span: DUMMY_SP,
        value: JsWord::from(format!("{}{}", "#", private_name.id.sym)),
      })))],
      import,
    ),
    Expr::Seq(seq) => {
      if (seq.exprs.len() < 2) {
        panic!("SequenceExpression with less than 2 expressions");
      }
      let first = parse_expr(seq.exprs.first().unwrap(), import);
      seq.exprs.iter().skip(1).fold(first, |left, right| {
        Node::new(
          Node::BinaryExpr,
          vec![
            //
            left,
            Box::new(Expr::Lit(Lit::Str(Str {
              raw: None,
              span: DUMMY_SP,
              value: JsWord::from(","),
            }))),
            parse_expr(right, import),
          ],
          import,
        )
      })
    }
    Expr::SuperProp(super_prop) => Node::new(
      Node::PropAccessExpr,
      vec![
        Node::new(Node::SuperKeyword, vec![], import),
        match &super_prop.prop {
          SuperProp::Ident(ident) => parse_ident(ident, import),
          SuperProp::Computed(comp) => Node::new(
            Node::ComputedPropertyNameExpr,
            vec![parse_expr(comp.expr.as_ref(), import)],
            import,
          ),
        },
      ],
      import,
    ),
    Expr::TaggedTpl(tagged_template) => Node::new(
      Node::TaggedTemplateExpr,
      vec![
        parse_expr(tagged_template.tag.as_ref(), import),
        parse_template(&tagged_template.tpl, import),
      ],
      import,
    ),
    Expr::This(this) => Node::new(Node::ThisExpr, vec![Box::new(expr.clone())], import),
    Expr::Tpl(template) => Node::new(
      Node::TemplateExpr,
      vec![parse_template(&template, import)],
      import,
    ),
    // erase <expr> as <type> - take <expr> only
    Expr::TsAs(ts_as) => parse_expr(&ts_as.expr, import),
    // erase <expr> as const - take <expr>
    Expr::TsConstAssertion(as_const) => parse_expr(&as_const.expr, import),
    // const getPerson = get<Person>; // replace with `get`
    Expr::TsInstantiation(ts_instantiation) => parse_expr(&ts_instantiation.expr, import),
    // .prop! // erase the !
    Expr::TsNonNull(ts_non_null) => parse_expr(&ts_non_null.expr, import),
    // <type>expr // erase <type> - take <expr> only
    Expr::TsTypeAssertion(as_type) => parse_expr(&as_type.expr, import),
    Expr::Unary(unary) => match unary.op {
      UnaryOp::TypeOf | UnaryOp::Void | UnaryOp::Delete => Node::new(
        match unary.op {
          UnaryOp::TypeOf => Node::TypeOfExpr,
          UnaryOp::Void => Node::VoidExpr,
          UnaryOp::Delete => Node::DeleteExpr,
          _ => panic!("impossible"),
        },
        vec![parse_expr(unary.arg.as_ref(), import)],
        import,
      ),
      _ => Node::new(
        Node::UnaryExpr,
        vec![
          // op
          Box::new(Expr::Lit(Lit::Str(Str {
            raw: None,
            span: DUMMY_SP,
            value: match unary.op {
              UnaryOp::Minus => JsWord::from("-"),
              UnaryOp::Plus => JsWord::from("+"),
              UnaryOp::Bang => JsWord::from("!"),
              UnaryOp::Tilde => JsWord::from("~"),
              UnaryOp::TypeOf => panic!("unexpected typeof operator"),
              UnaryOp::Void => panic!("unexpected void operator"),
              UnaryOp::Delete => panic!("unexpected delete operator"),
            },
          }))),
          // expr
          parse_expr(unary.arg.as_ref(), import),
        ],
        import,
      ),
    },
    Expr::Update(update) => Node::new(
      if update.prefix {
        Node::UnaryExpr
      } else {
        Node::PostfixUnaryExpr
      },
      vec![
        // op
        Box::new(Expr::Lit(Lit::Str(Str {
          raw: None,
          span: DUMMY_SP,
          value: match update.op {
            UpdateOp::PlusPlus => JsWord::from("++"),
            UpdateOp::MinusMinus => JsWord::from("--"),
          },
        }))),
        // expr
        parse_expr(update.arg.as_ref(), import),
      ],
      import,
    ),
    Expr::Yield(yield_expr) => Node::new(
      Node::YieldExpr,
      vec![
        yield_expr
          .arg
          .as_ref()
          .map(|expr| parse_expr(expr.as_ref(), import))
          .unwrap_or(Box::new(undefined())),
        Box::new(Expr::Lit(Lit::Bool(Bool {
          value: yield_expr.delegate,
          span: DUMMY_SP,
        }))),
      ],
      import,
    ),
  }
}

fn parse_call_expr(
  expr: &Expr,
  args: &[ExprOrSpread],
  is_optional: bool,
  import: &Ident,
) -> Box<Expr> {
  Node::new(
    Node::CallExpr,
    vec![
      //
      parse_expr(expr, import),
      Box::new(Expr::Array(ArrayLit {
        span: DUMMY_SP,
        elems: args
          .iter()
          .map(|arg| {
            Some(ExprOrSpread {
              spread: None,
              expr: if arg.spread.is_some() {
                // TODO: we can't represent this right now
                Node::new(Node::Argument, vec![], import)
              } else {
                Node::new(Node::Argument, vec![], import)
              },
            })
          })
          .collect(),
      })),
    ],
    import,
  )
}

fn parse_callee(
  callee: &Callee,
  args: &[ExprOrSpread],
  is_optional: bool,
  import: &Ident,
) -> Box<Expr> {
  Node::new(
    Node::CallExpr,
    vec![
      //
      match callee {
        Callee::Super(sup) => Node::new(Node::SuperKeyword, vec![], import),
        Callee::Import(_) => Node::new(Node::ImportKeyword, vec![], import),
        Callee::Expr(expr) => parse_expr(expr, import),
      },
      parse_call_args(args, import),
    ],
    import,
  )
}

fn parse_call_args(args: &[ExprOrSpread], import: &Ident) -> Box<Expr> {
  Box::new(Expr::Array(ArrayLit {
    span: DUMMY_SP,
    elems: args
      .iter()
      .map(|arg| {
        Some(ExprOrSpread {
          spread: None,
          expr: if arg.spread.is_some() {
            // TODO: we can't represent this right now
            Node::new(Node::Argument, vec![], import)
          } else {
            Node::new(Node::Argument, vec![], import)
          },
        })
      })
      .collect(),
  }))
}

fn parse_member(member: &MemberExpr, is_optional: bool, import: &Ident) -> Box<Expr> {
  Node::new(
    Node::PropAccessExpr,
    vec![
      parse_expr(member.obj.as_ref(), import),
      match &member.prop {
        MemberProp::Ident(ident) => parse_ident(ident, import),
        MemberProp::PrivateName(private_name) => Node::new(
          Node::PrivateIdentifier,
          vec![Box::new(Expr::Lit(Lit::Str(Str {
            raw: None,
            span: DUMMY_SP,
            value: JsWord::from(format!("#{}", private_name.id.sym)),
          })))],
          import,
        ),
        MemberProp::Computed(comp) => Node::new(
          Node::ComputedPropertyNameExpr,
          vec![parse_expr(comp.expr.as_ref(), import)],
          import,
        ),
      },
      if is_optional {
        Box::new(True)
      } else {
        Box::new(False)
      },
    ],
    import,
  )
}

fn parse_block(block: &BlockStmt, import: &Ident) -> Box<Expr> {
  Node::new(
    Node::BlockStmt,
    block
      .stmts
      .iter()
      .map(|stmt| parse_stmt(stmt, import))
      .collect(),
    import,
  )
}

fn parse_class_member(member: &ClassMember, import: &Ident) -> Option<Box<Expr>> {
  match member {
    ClassMember::ClassProp(prop) => Some(Node::new(
      Node::PropDecl,
      vec![
        //
        parse_prop_name(&prop.key, import),
        Box::new(Expr::Lit(Lit::Bool(Bool {
          value: prop.is_static,
          span: DUMMY_SP,
        }))),
        prop
          .value
          .as_ref()
          .map(|v| parse_expr(v.as_ref(), import))
          .unwrap_or(Box::new(undefined())),
      ],
      import,
    )),
    ClassMember::Constructor(ctor) => Some(Node::new(
      Node::ConstructorDecl,
      vec![
        // params
        Box::new(Expr::Array(ArrayLit {
          elems: ctor
            .params
            .iter()
            .map(|param| {
              Some(ExprOrSpread {
                spread: None,
                expr: match param {
                  ParamOrTsParamProp::Param(p) => parse_param(p, import),
                  ParamOrTsParamProp::TsParamProp(p) => match &p.param {
                    TsParamPropParam::Ident(i) => Node::new(
                      Node::ParameterDecl,
                      vec![
                        //
                        parse_ident(&i, import),
                      ],
                      import,
                    ),
                    TsParamPropParam::Assign(i) => Node::new(
                      Node::ParameterDecl,
                      vec![
                        parse_pattern(i.left.as_ref(), import),
                        parse_expr(i.right.as_ref(), import),
                      ],
                      import,
                    ),
                  },
                },
              })
            })
            .collect(),
          span: DUMMY_SP,
        })),
        // block
        parse_block(ctor.body.as_ref().unwrap(), import),
      ],
      import,
    )),
    ClassMember::Empty(_) => None,
    ClassMember::Method(method) => Some(Node::new(
      Node::MethodDecl,
      vec![
        //
        parse_prop_name(&method.key, import),
        parse_params(&method.function.params, import),
        parse_block(method.function.body.as_ref().unwrap(), import),
      ],
      import,
    )),
    ClassMember::PrivateMethod(method) => Some(Node::new(
      Node::MethodDecl,
      vec![
        //
        Node::new(
          Node::PrivateIdentifier,
          vec![
            //
            Box::new(Expr::Lit(Lit::Str(Str {
              raw: None,
              span: DUMMY_SP,
              value: JsWord::from(format!("#{}", method.key.id.sym)),
            }))),
          ],
          import,
        ),
        parse_params(&method.function.params, import),
        parse_block(method.function.body.as_ref().unwrap(), import),
      ],
      import,
    )),
    ClassMember::PrivateProp(prop) => Some(Node::new(
      Node::PropDecl,
      vec![
        //
        Node::new(
          Node::PrivateIdentifier,
          vec![
            //
            Box::new(Expr::Lit(Lit::Str(Str {
              raw: None,
              span: DUMMY_SP,
              value: JsWord::from(format!("#{}", prop.key.id.sym)),
            }))),
          ],
          import,
        ),
        Box::new(Expr::Lit(Lit::Bool(Bool {
          value: prop.is_static,
          span: DUMMY_SP,
        }))),
        prop
          .value
          .as_ref()
          .map(|v| parse_expr(v.as_ref(), import))
          .unwrap_or(Box::new(undefined())),
      ],
      import,
    )),
    ClassMember::StaticBlock(static_block) => Some(Node::new(
      Node::ClassStaticBlockDecl,
      vec![
        //
        parse_block(&static_block.body, import),
      ],
      import,
    )),
    ClassMember::TsIndexSignature(_) => None,
  }
}

fn parse_prop_name(prop: &PropName, import: &Ident) -> Box<Expr> {
  match prop {
    PropName::BigInt(i) => Node::new(
      Node::BigIntExpr,
      vec![Box::new(Expr::Lit(Lit::BigInt(i.clone())))],
      import,
    ),
    PropName::Computed(c) => Node::new(
      Node::ComputedPropertyNameExpr,
      vec![parse_expr(c.expr.as_ref(), import)],
      import,
    ),
    PropName::Ident(i) => parse_ident(i, import),
    PropName::Num(n) => Node::new(
      Node::BigIntExpr,
      vec![Box::new(Expr::Lit(Lit::Num(n.clone())))],
      import,
    ),
    PropName::Str(s) => Node::new(
      Node::BigIntExpr,
      vec![Box::new(Expr::Lit(Lit::Str(s.clone())))],
      import,
    ),
  }
}

fn parse_var_decl(var_decl: &VarDecl, import: &Ident) -> Box<Expr> {
  Node::new(
    Node::VariableStmt,
    vec![Node::new(
      Node::VariableDeclList,
      var_decl
        .decls
        .iter()
        .filter_map(|decl| match &decl.name {
          Pat::Ident(ident) => Some(parse_ident(&ident.id, import)),
          _ => None,
        })
        .collect(),
      import,
    )],
    import,
  )
}

fn parse_template(tpl: &Tpl, import: &Ident) -> Box<Expr> {
  Box::new(Expr::Array(ArrayLit {
    elems: tpl
      .exprs
      .iter()
      .zip(&tpl.quasis)
      .flat_map(|(expr, quasi)| {
        vec![
          //
          parse_template_element(&quasi, import),
          parse_expr(expr, import),
        ]
      })
      .chain(if tpl.quasis.len() > tpl.exprs.len() {
        vec![parse_template_element(&tpl.quasis.last().unwrap(), import)]
      } else {
        vec![]
      })
      .map(|expr| Some(ExprOrSpread { expr, spread: None }))
      .collect(),
    span: DUMMY_SP,
  }))
}

fn parse_template_element(element: &TplElement, import: &Ident) -> Box<Expr> {
  Node::new(
    Node::StringLiteralExpr,
    vec![
      //
      Box::new(Expr::Lit(Lit::Str(Str {
        raw: None,
        span: DUMMY_SP,
        value: element.raw.clone(),
      }))),
    ],
    import,
  )
}

fn parse_ident(ident: &Ident, import: &Ident) -> Box<Expr> {
  Node::new(
    Node::Identifier,
    vec![Node::new(
      Node::StringLiteralExpr,
      vec![Box::new(Expr::Lit(Lit::Str(Str {
        raw: None,
        span: DUMMY_SP,
        value: ident.sym.clone(),
      })))],
      import,
    )],
    import,
  )
}

fn parse_params(params: &[Param], import: &Ident) -> Box<Expr> {
  Box::new(Expr::Array(ArrayLit {
    elems: params
      .iter()
      .map(|param| {
        Some(ExprOrSpread {
          spread: None,
          expr: parse_param(param, import),
        })
      })
      .collect(),
    span: DUMMY_SP,
  }))
}

fn parse_param(param: &Param, import: &Ident) -> Box<Expr> {
  parse_pattern(&param.pat, import)
}

fn parse_patterns(pats: &[Pat], import: &Ident) -> Box<Expr> {
  Box::new(Expr::Array(ArrayLit {
    elems: pats
      .iter()
      .map(|pat| {
        Some(ExprOrSpread {
          spread: None,
          expr: parse_pattern(pat, import),
        })
      })
      .collect(),
    span: DUMMY_SP,
  }))
}

fn parse_pattern(pat: &Pat, import: &Ident) -> Box<Expr> {
  match pat {
    Pat::Array(array_binding) => Node::new(
      Node::ArrayBinding,
      array_binding
        .elems
        .iter()
        .filter_map(|e| e.as_ref())
        .map(|elem| match elem {
          Pat::Ident(ident) => Node::new(
            Node::BindingElem,
            vec![
              //
              parse_pattern(&elem, import),
              Box::new(False),
            ],
            import,
          ),
          _ => parse_pattern(&elem, import),
        })
        .collect(),
      import,
    ),
    Pat::Object(object_binding) => Node::new(
      Node::ObjectBinding,
      object_binding
        .props
        .iter()
        .map(|prop| match prop {
          ObjectPatProp::Assign(assign) => Node::new(
            Node::BindingElem,
            match &assign.value {
              // {key: value}
              Some(value) => vec![
                parse_expr(value.as_ref(), import),
                Box::new(False),
                Box::new(Expr::Ident(assign.key.clone())),
              ],
              // {key}
              None => vec![Box::new(Expr::Ident(assign.key.clone())), Box::new(False)],
            },
            import,
          ),
          // {key: value}
          ObjectPatProp::KeyValue(kv) => Node::new(
            Node::BindingElem,
            vec![
              match kv.value.as_ref() {
                // if this is an assign pattern, e.g. {key = value}
                // then parse `key` as the `BindingElement.name` in FunctionlessAST
                Pat::Assign(assign) => parse_pattern(assign.left.as_ref(), import),
                value => parse_pattern(value, import),
              },
              Box::new(False),
              parse_prop_name(&kv.key, import),
              match kv.value.as_ref() {
                // if this is an assign patter, e.g. `{key = value}`
                // then parse `value` as the `BindingElement.initializer` in FunctionlessAST
                Pat::Assign(assign) => parse_expr(assign.right.as_ref(), import),
                _ => Box::new(undefined()),
              },
            ],
            import,
          ),
          // { ...rest }
          ObjectPatProp::Rest(rest) => Node::new(
            Node::BindingElem,
            vec![
              //
              parse_pattern(&rest.arg, import),
              Box::new(True),
            ],
            import,
          ),
        })
        .collect(),
      import,
    ),
    Pat::Assign(assign) => Node::new(
      Node::BindingElem,
      vec![
        //
        parse_pattern(assign.left.as_ref(), import),
        Box::new(False),
        Box::new(undefined()),
        parse_expr(assign.right.as_ref(), import),
      ],
      import,
    ),
    Pat::Expr(expr) => parse_expr(expr, import),
    Pat::Ident(ident) => parse_ident(ident, import),
    Pat::Invalid(invalid) => panic!("Invalid Syntax"),
    Pat::Rest(rest) => Node::new(Node::BindingElem, vec![], import),
  }
}

#[derive(AsRefStr)]
enum Node {
  Err,
  // Expr
  Argument,
  ArrayLiteralExpr,
  ArrowFunctionExpr,
  AwaitExpr,
  BigIntExpr,
  BinaryExpr,
  BooleanLiteralExpr,
  CallExpr,
  ClassExpr,
  ComputedPropertyNameExpr,
  ConditionExpr,
  DeleteExpr,
  ElementAccessExpr,
  FunctionExpr,
  Identifier,
  NewExpr,
  NullLiteralExpr,
  NumberLiteralExpr,
  ObjectLiteralExpr,
  ParenthesizedExpr,
  PostfixUnaryExpr,
  PrivateIdentifier,
  PromiseArrayExpr,
  PromiseExpr,
  PropAccessExpr,
  PropAssignExpr,
  ReferenceExpr,
  RegexExpr,
  SpreadAssignExpr,
  SpreadElementExpr,
  StringLiteralExpr,
  TaggedTemplateExpr,
  TemplateExpr,
  ThisExpr,
  TypeOfExpr,
  UnaryExpr,
  UndefinedLiteralExpr,
  VoidExpr,
  YieldExpr,
  // Stmt
  BlockStmt,
  BreakStmt,
  CaseClause,
  CatchClause,
  ContinueStmt,
  DebuggerStmt,
  DefaultClause,
  DoStmt,
  EmptyStmt,
  ExprStmt,
  ForInStmt,
  ForOfStmt,
  ForStmt,
  IfStmt,
  LabelledStmt,
  ReturnStmt,
  SwitchStmt,
  ThrowStmt,
  TryStmt,
  VariableDecl,
  VariableDeclList,
  VariableStmt,
  WhileStmt,
  WithStmt,
  // Decl
  ArrayBinding,
  BindingElem,
  ClassDecl,
  ClassStaticBlockDecl,
  ConstructorDecl,
  FunctionDecl,
  GetAccessorDecl,
  MethodDecl,
  ObjectBinding,
  ParameterDecl,
  PropDecl,
  SetAccessorDecl,
  // Keywords
  ImportKeyword,
  SuperKeyword,
}

impl Node {
  fn error(message: &str, import: &Ident) -> Box<Expr> {
    Node::new(
      Node::Err,
      vec![Box::new(Expr::Lit(Lit::Str(Str {
        raw: None,
        span: DUMMY_SP,
        value: JsWord::from(message),
      })))],
      import,
    )
  }

  fn new(kind: Node, args: Vec<Box<Expr>>, import: &Ident) -> Box<Expr> {
    Box::new(Expr::Call(CallExpr {
      callee: Callee::Expr(Box::new(Expr::Member(MemberExpr {
        obj: Box::new(Expr::Ident(import.clone())),
        prop: MemberProp::Ident(quote_ident!(kind.as_ref())),
        span: DUMMY_SP,
      }))),
      args: args
        .iter()
        .map(|arg| ExprOrSpread {
          expr: arg.clone(),
          spread: None,
        })
        .collect(),
      type_args: None,
      span: DUMMY_SP,
    }))
  }
}
