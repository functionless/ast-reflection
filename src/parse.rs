use core::panic;

use swc_common::DUMMY_SP;
use swc_plugin::utils::quote_ident;
use swc_plugin::{
  ast::*,
};

use crate::{closure_decorator::ClosureDecorator};
use crate::ast::Node;

const EMPTY_VEC: Vec<Box<Expr>> = vec![];

impl ClosureDecorator {
  pub fn parse_class_decl(&self, class_decl: &ClassDecl) -> Box<Expr> {
    self.parse_class(Node::ClassDecl, Some(&class_decl.ident), &class_decl.class)
  }

  pub fn parse_class_expr(&self, class_expr: &ClassExpr) -> Box<Expr> {
    self.parse_class(Node::ClassExpr, class_expr.ident.as_ref(), &class_expr.class)
  }

  fn parse_class(&self, kind: Node, ident: Option<&Ident>, class: &Class) -> Box<Expr> {
    self.new_node(
      kind,
      vec![
        ident
          .as_ref()
          .map(|i| self.parse_ident(&i, false))
          .unwrap_or(undefined_expr()),
        class
          .super_class
          .as_ref()
          .map(|sup| self.parse_expr(sup.as_ref()))
          .unwrap_or(undefined_expr()),
        Box::new(Expr::Array(ArrayLit {
          elems: class
            .body
            .iter()
            .map(|member| {
              self
                .parse_class_member(member)
                .map(|expr| ExprOrSpread { expr, spread: None })
            })
            .collect(),
          span: DUMMY_SP,
        })),
      ],
    )
  }

  pub fn parse_class_method(&self, method: &ClassMethod) -> Box<Expr> {
    self.new_node(
      Node::MethodDecl,
      vec![
        //
        self.parse_prop_name(&method.key),
        self.parse_params(&method.function.params),
        self.parse_block(method.function.body.as_ref().unwrap()),
      ],
    )
  }

  pub fn parse_constructor(&self, ctor: &Constructor) -> Box<Expr> {
    self.new_node(
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
                  ParamOrTsParamProp::Param(p) => self.parse_param(p),
                  ParamOrTsParamProp::TsParamProp(p) => match &p.param {
                    TsParamPropParam::Ident(i) => self.new_node(
                      Node::ParameterDecl,
                      vec![
                        //
                        self.parse_ident(&i, false),
                      ],
                      
                    ),
                    TsParamPropParam::Assign(i) => self.new_node(
                      Node::ParameterDecl,
                      vec![
                        self.parse_pat(i.left.as_ref()),
                        self.parse_expr(i.right.as_ref()),
                      ],
                      
                    ),
                  },
                },
              })
            })
            .collect(),
          span: DUMMY_SP,
        })),
        // block
        self.parse_block(ctor.body.as_ref().unwrap()),
      ],
    )
  }

  pub fn parse_function_decl(&self, function: &FnDecl) -> Box<Expr> {
    self.parse_function(Node::FunctionDecl, Some(&function.ident), &function.function)
  }

  pub fn parse_function_expr(&self, function: &FnExpr) -> Box<Expr> {
    self.parse_function(Node::FunctionExpr, function.ident.as_ref(), &function.function)
  }

  fn parse_function(&self, kind: Node, name: Option<&Ident>, function: &Function) -> Box<Expr> {
    self.new_node(
      kind,
      vec![
        name.map(|ident| str(&ident.to_id().0)).unwrap_or(undefined_expr()),
        self.parse_params(&function.params), 
        function.body.as_ref().map(|b| self.parse_block(b)).unwrap_or(undefined_expr()),
        if function.is_async { true_expr() } else { false_expr() },
        if function.is_generator { true_expr() } else { false_expr() }
      ],
    )
  }

  pub fn parse_arrow(&self, arrow: &ArrowExpr) -> Box<Expr> {
    self.new_node(
      Node::ArrowFunctionExpr,
      vec![
        self.parse_pats_as_params(&arrow.params), 
        match &arrow.body {
          BlockStmtOrExpr::BlockStmt(block) => self.parse_block(block),
          BlockStmtOrExpr::Expr(expr) => self.new_node(Node::BlockStmt, vec![
            Box::new(Expr::Array(ArrayLit {
              elems: vec![
                Some(ExprOrSpread {
                  spread: None,
                  expr: self.new_node(Node::ReturnStmt, vec![self.parse_expr(expr)])
                }),
              ],
              span: DUMMY_SP,
            })) 
          ]),
        },
        if arrow.is_async { true_expr() } else { false_expr() },
      ],
    )
  }

  fn parse_params(&self, params: &[Param]) -> Box<Expr> {
    Box::new(Expr::Array(ArrayLit {
      elems: params
        .iter()
        .map(|param| {
          Some(ExprOrSpread {
            spread: None,
            expr: self.parse_param(param),
          })
        })
        .collect(),
      span: DUMMY_SP,
    }))
  }

  fn parse_param(&self, param: &Param) -> Box<Expr> {
    self.new_node(Node::ParameterDecl, vec![
      self.parse_pat(&param.pat)
    ])
  }
  
  fn parse_decl(&self, decl: &Decl) -> Box<Expr> {
    match decl {
      Decl::Class(class_decl) => self.parse_class_decl(class_decl),
      Decl::Fn(function) => self.parse_function_decl(function),
      Decl::TsEnum(_) => panic!("enums not supported"),
      Decl::TsInterface(_) => panic!("interface not supported"),
      Decl::TsModule(_) => panic!("module declarations not supported"),
      Decl::TsTypeAlias(_) => panic!("type alias not supported"),
      Decl::Var(var_decl) => self.new_node(Node::VariableStmt, vec![self.parse_var_decl(var_decl)]),
    }
  }

  fn parse_stmt(&self, stmt: &Stmt) -> Box<Expr> {
    match stmt {
      Stmt::Block(block) => self.parse_block(block),
      Stmt::Break(_break_stmt) => self.new_node(Node::BreakStmt, EMPTY_VEC),
      Stmt::Continue(_continue_stmt) => self.new_node(Node::ContinueStmt, EMPTY_VEC),
      Stmt::Debugger(_debugger) => self.new_node(Node::DebuggerStmt, EMPTY_VEC),
      Stmt::Decl(decl) => self.parse_decl(decl),
      Stmt::DoWhile(do_while) => self.new_node(
        Node::DoStmt,
        vec![
          // block
          match do_while.body.as_ref() {
            Stmt::Block(block) => self.parse_block(&block),
            stmt => self.new_node(Node::BlockStmt, vec![
              Box::new(Expr::Array(ArrayLit {
                elems: vec![Some(ExprOrSpread {
                  expr: self.parse_stmt(stmt),
                  spread: None,
                })],
                span: DUMMY_SP
              }))
            ])
          },
          // condition
          self.parse_expr(do_while.test.as_ref()),
        ],
      ),
      Stmt::Empty(_empty) => self.new_node(Node::EmptyStmt, EMPTY_VEC),
      Stmt::Expr(expr_stmt) => self.new_node(
        Node::ExprStmt,
        vec![
          //expr
          self.parse_expr(expr_stmt.expr.as_ref()),
        ],
      ),
      // TODO
      Stmt::For(for_stmt) => self.new_node(
        Node::ForStmt,
        vec![
          self.parse_stmt(for_stmt.body.as_ref()),
          for_stmt
            .init
            .as_ref()
            .map(|init| match init {
              VarDeclOrExpr::Expr(expr) => self.parse_expr(expr.as_ref()),
              VarDeclOrExpr::VarDecl(var) => self.parse_var_decl(&var),
            })
            .unwrap_or(undefined_expr()),
          for_stmt.test.as_ref().map(|test| self.parse_expr(test)).unwrap_or(undefined_expr()),
          for_stmt.update.as_ref().map(|test| self.parse_expr(test)).unwrap_or(undefined_expr())
        ], 
      ),
      // for (const left in right)
      Stmt::ForIn(for_in) => self.new_node(
        Node::ForInStmt,
        vec![
          match &for_in.left {
            VarDeclOrPat::VarDecl(var_decl) => self.parse_var_declarator(var_decl.decls.first().unwrap()),
            VarDeclOrPat::Pat(pat) => self.parse_pat(pat),
          },
          self.parse_expr(&for_in.right),
          self.parse_stmt(for_in.body.as_ref()),
        ],
      ),
      // for (const left of right)
      Stmt::ForOf(for_of) => self.new_node(
        Node::ForOfStmt,
        vec![
          match &for_of.left {
            VarDeclOrPat::VarDecl(var_decl) => self.parse_var_declarator(var_decl.decls.first().unwrap()),
            VarDeclOrPat::Pat(pat) => self.parse_pat(pat),
          },
          self.parse_expr(&for_of.right),
          self.parse_stmt(for_of.body.as_ref()),
        ],
      ),
      Stmt::If(if_stmt) => self.new_node(
        Node::IfStmt,
        vec![
          // when
          self.parse_expr(&if_stmt.test),
          // then
          self.parse_stmt(if_stmt.cons.as_ref()),
          // else
          if_stmt
            .alt
            .as_ref()
            .map(|alt| self.parse_stmt(alt.as_ref()))
            .unwrap_or(undefined_expr()),
        ],
      ),
      // https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Statements/label
      // for now, we just erase the label
      Stmt::Labeled(labelled) => self.new_node(
        Node::LabelledStmt,
        vec![self.parse_stmt(&labelled.body)],
      ),
      Stmt::Return(return_stmt) => self.new_node(
        Node::ReturnStmt,
        vec![match return_stmt.arg.as_ref() {
          Some(arg) => self.parse_expr(&arg),
          // encode an empty `return;` as `return undefined();`
          None => self.new_node(Node::UndefinedLiteralExpr, EMPTY_VEC),
        }],
      ),
      // TODO: support switch - https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Statements/switch
      Stmt::Switch(switch) => self.new_node(
        Node::SwitchStmt,
        vec![
          self.parse_expr(&switch.discriminant),
          // case
          Box::new(Expr::Array(ArrayLit {
            span: DUMMY_SP,
            elems: switch
              .cases
              .iter()
              .map(|case| {
                let stmts = Box::new(Expr::Array(ArrayLit {
                  span: DUMMY_SP,
                  elems: case.cons.iter().map(|stmt| self.parse_stmt(stmt)).map(|expr| Some(ExprOrSpread {
                    expr,
                    spread: None
                  })).collect()
                }));

                Some(ExprOrSpread {
                  expr: match case.test.as_ref() {
                    Some(test) => self.new_node(Node::CaseClause, vec![
                      self.parse_expr(test),
                      stmts
                    ]),
                    None => self.new_node(Node::DefaultClause, vec![
                      stmts
                    ]),
                  },
                  spread: None,
                })
              })
              .collect(),
          })),
        ],
      ),
      Stmt::Throw(throw) => self.new_node(
        Node::ThrowStmt,
        vec![self.parse_expr(throw.arg.as_ref())],
      ),
      Stmt::Try(try_stmt) => self.new_node(
        Node::TryStmt,
        vec![
          self.parse_block(&try_stmt.block),
          try_stmt
            .handler
            .as_ref()
            .map(|catch| self.new_node(Node::CatchClause, vec![
              match &catch.param {
                Some(pat) => self.new_node(Node::VariableDecl, match pat {
                  Pat::Assign(assign) => vec![
                    self.parse_pat(&assign.left),
                    self.parse_expr(&assign.right)
                  ],
                  _ => vec![
                    self.parse_pat(pat)
                  ]
                }),
                None => undefined_expr(),
              },
              self.parse_block(&catch.body)
            ]))
            .unwrap_or(undefined_expr()),
          try_stmt
            .finalizer
            .as_ref()
            .map(|finalizer| self.parse_block(&finalizer))
            .unwrap_or(undefined_expr()),
        ],
      ),
      Stmt::While(while_stmt) => self.new_node(
        Node::WhileStmt,
        vec![
          self.parse_expr(while_stmt.test.as_ref()),
          match while_stmt.body.as_ref() {
            Stmt::Block(block) => self.parse_block(&block),
            stmt => self.new_node(Node::BlockStmt, vec![
              Box::new(Expr::Array(ArrayLit {
                elems: vec![Some(ExprOrSpread {
                  expr: self.parse_stmt(stmt),
                  spread: None,
                })],
                span: DUMMY_SP
              }))
            ])
          },
        ],
      ),
      // TODO: support with https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Statements/with
      Stmt::With(with) => self.new_node(
        Node::WithStmt,
        vec![
          self.parse_expr(with.obj.as_ref()),
          self.parse_stmt(with.body.as_ref()),
        ],
      ),
    }
  }

  fn parse_expr(&self, expr: &Expr) -> Box<Expr> {
    match expr {
      Expr::Array(array) => self.new_node(
        Node::ArrayLiteralExpr,
        vec![
          Box::new(Expr::Array(ArrayLit {
            elems: array
              .elems  
              .iter()
              .map(|element| Some(ExprOrSpread {
                expr: match element {
                  Some(e) => if e.spread.is_some() {
                    self.new_node(Node::SpreadElementExpr, vec![self.parse_expr(e.expr.as_ref())])
                  } else {
                    self.parse_expr(e.expr.as_ref())
                  },
                  None => self.new_node(Node::OmittedExpr, vec![])
                },
                spread: None
              }))
              .collect(),
            span: DUMMY_SP
          }))
        ],
      ),
      Expr::Arrow(arrow) => self.parse_arrow(arrow),
      Expr::Assign(assign) => self.new_node(
        Node::BinaryExpr,
        vec![
          match &assign.left {
            PatOrExpr::Expr(expr) => self.parse_expr(expr),
            PatOrExpr::Pat(pat) => self.parse_pat(pat),
          },
          Box::new(Expr::Lit(Lit::Str(Str {
            raw: None,
            span: DUMMY_SP,
            value: match assign.op {
              AssignOp::Assign => JsWord::from("="),
              AssignOp::AddAssign => JsWord::from("+="),
              AssignOp::SubAssign => JsWord::from("-="),
              AssignOp::MulAssign => JsWord::from("*="),
              AssignOp::DivAssign => JsWord::from("/="),
              AssignOp::ModAssign => JsWord::from("%="),
              AssignOp::LShiftAssign => JsWord::from("<<="),
              AssignOp::RShiftAssign => JsWord::from(">>="),
              AssignOp::ZeroFillRShiftAssign => JsWord::from(">>>="),
              AssignOp::BitOrAssign => JsWord::from("|="),
              AssignOp::BitXorAssign => JsWord::from("^="),
              AssignOp::BitAndAssign => JsWord::from("&="),
              AssignOp::ExpAssign => JsWord::from("**="),
              AssignOp::AndAssign => JsWord::from("&&="),
              AssignOp::OrAssign => JsWord::from("||="),
              AssignOp::NullishAssign => JsWord::from("??="),
            },
          }))),
          self.parse_expr(assign.right.as_ref()),
        ],
      ),
      Expr::Await(a_wait) => self.new_node(
        Node::AwaitExpr,
        vec![self.parse_expr(a_wait.arg.as_ref())],
      ),
      Expr::Bin(binary_op) => self.new_node(
        Node::BinaryExpr,
        vec![
          self.parse_expr(binary_op.left.as_ref()),
          Box::new(Expr::Lit(Lit::Str(Str {
            span: DUMMY_SP,
            raw: None,
            value: match binary_op.op {
              BinaryOp::Add => JsWord::from("+"),
              BinaryOp::BitAnd => JsWord::from("&"),
              BinaryOp::BitOr => JsWord::from("|"),
              BinaryOp::BitXor => JsWord::from("^"),
              BinaryOp::Div => JsWord::from("/"),
              BinaryOp::EqEq => JsWord::from("=="),
              BinaryOp::EqEqEq => JsWord::from("==="),
              BinaryOp::Exp => JsWord::from("**"),
              BinaryOp::Gt => JsWord::from(">"),
              BinaryOp::GtEq => JsWord::from(">="),
              BinaryOp::In => JsWord::from("in"),
              BinaryOp::InstanceOf => JsWord::from("instanceof"),
              BinaryOp::LogicalAnd => JsWord::from("&&"),
              BinaryOp::LogicalOr => JsWord::from("||"),
              BinaryOp::LShift => JsWord::from("<<"),
              BinaryOp::Lt => JsWord::from("<"),
              BinaryOp::LtEq => JsWord::from("<="),
              BinaryOp::Mod => JsWord::from("%"),
              BinaryOp::Mul => JsWord::from("*"),
              BinaryOp::NotEq => JsWord::from("!="),
              BinaryOp::NotEqEq => JsWord::from("!=="),
              BinaryOp::NullishCoalescing => JsWord::from("??"),
              BinaryOp::RShift => JsWord::from(">>"),
              BinaryOp::Sub => JsWord::from("-"),
              BinaryOp::ZeroFillRShift => JsWord::from(">>>"),
            },
          }))),
          self.parse_expr(binary_op.right.as_ref()),
        ],
      ),
      Expr::Call(call) => self.parse_callee(&call.callee, &call.args, false),
      // TODO: extract properties from ts-parameters
      Expr::Class(class_expr) => self.parse_class_expr(class_expr),
      Expr::Cond(cond) => self.new_node(
        Node::ConditionExpr,
        vec![
          // when
          self.parse_expr(&cond.test.as_ref()),
          // then
          self.parse_expr(&cond.cons.as_ref()),
          // else
          self.parse_expr(&cond.alt.as_ref()),
        ],
      ),
      Expr::Fn(function) => self.parse_function_expr(&function),
      Expr::Ident(id) => self.parse_ident(id, true),
      Expr::Invalid(_invalid) => self.new_error_node("Syntax Error"),
      Expr::JSXElement(_jsx_element) => self.new_error_node("not sure what to do with JSXElement"),
      Expr::JSXEmpty(_jsx_empty) => self.new_error_node("not sure what to do with JSXEmpty"),
      Expr::JSXFragment(_jsx_fragment) => self.new_error_node("not sure what to do with JSXFragment"),
      Expr::JSXMember(_jsx_member) => self.new_error_node("not sure what to do with JSXMember"),
      Expr::JSXNamespacedName(_jsx_namespace_name) => {
        self.new_error_node("not sure what to do with JSXNamespacedName")
      }
      Expr::Lit(literal) => match &literal {
        // not sure what type of node this is, will just error for now
        Lit::JSXText(_) => self.new_error_node("not sure what to do with JSXText"),
        _ => self.new_node(
          match literal {
            Lit::Bool(_) => Node::BooleanLiteralExpr,
            Lit::BigInt(_) => Node::BigIntExpr,
            Lit::Null(_) => Node::NullLiteralExpr,
            Lit::Num(_) => Node::NumberLiteralExpr,
            Lit::Regex(_) => Node::RegexExpr,
            Lit::Str(_) => Node::StringLiteralExpr,
            // impossible to reach here
            Lit::JSXText(_text) => panic!("not sure what to do with JSXText"),
          },
          vec![Box::new(expr.clone())],
          
        ),
      },
      Expr::Member(member) => self.parse_member(member, false),
      Expr::MetaProp(_meta_prop) => self.new_error_node("MetaProp is not supported"),
      Expr::New(call) => self.new_node(
        Node::NewExpr,
        vec![
          //
          self.parse_expr(&call.callee),
          call.args.as_ref().map(|args| self.parse_call_args(args)).unwrap_or(empty_array_expr()) ,
        ],
      ),
      Expr::Object(object) => self.new_node(
        Node::ObjectLiteralExpr,
        vec![
          Box::new(Expr::Array(ArrayLit {
            span: DUMMY_SP,
            elems: object
              .props
              .iter()
              .map(|prop| match prop {
                PropOrSpread::Prop(prop) => match prop.as_ref() {
                  // invalid according to SWC's docs on Prop::Assign
                  Prop::Assign(_assign) => panic!("Invalid Syntax in Object Literal"),
                  Prop::Getter(getter) => self.new_node(
                    Node::GetAccessorDecl,
                    vec![
                      self.parse_prop_name(&getter.key),
                      self.parse_block(&getter.body.as_ref().unwrap()),
                    ],
                    
                  ),
                  Prop::KeyValue(assign) => self.new_node(
                    Node::PropAssignExpr,
                    vec![
                      self.parse_prop_name(&assign.key),
                      self.parse_expr(assign.value.as_ref()),
                    ],
                    
                  ),
                  Prop::Method(method) => self.new_node(
                    Node::MethodDecl,
                    vec![
                      //
                      self.parse_prop_name(&method.key),
                      self.parse_params(&method.function.params),
                      self.parse_block(method.function.body.as_ref().unwrap()),
                    ],
                    
                  ),
                  Prop::Setter(setter) => self.new_node(
                    Node::SetAccessorDecl,
                    vec![
                      self.parse_prop_name(&setter.key),
                      self.parse_pat(&setter.param),
                      self.parse_block(setter.body.as_ref().unwrap()),
                    ],
                    
                  ),
                  Prop::Shorthand(ident) => self.new_node(
                    Node::PropAssignExpr,
                    vec![self.parse_ident(ident, false), self.parse_ident(ident, true)],
                  ),
                },
                PropOrSpread::Spread(spread) => self.new_node(
                  Node::SpreadAssignExpr,
                  vec![self.parse_expr(spread.expr.as_ref())],
                  
                ),
              })
              .map(|prop| Some(ExprOrSpread {
                expr: prop,
                spread: None
              }))
              .collect(),
          }))
        ]
      ),
      Expr::OptChain(opt_chain) => match &opt_chain.base {
        OptChainBase::Call(call) => self.parse_call_expr(&call.callee, &call.args, true),
        OptChainBase::Member(member) => self.parse_member(&member, true),
      },
      Expr::Paren(paren) => self.new_node(
        Node::ParenthesizedExpr,
        vec![self.parse_expr(paren.expr.as_ref())],
      ),
      Expr::PrivateName(private_name) => self.new_node(
        Node::PrivateIdentifier,
        vec![Box::new(Expr::Lit(Lit::Str(Str {
          raw: None,
          span: DUMMY_SP,
          value: JsWord::from(format!("{}{}", "#", private_name.id.sym)),
        })))],
      ),
      Expr::Seq(seq) => {
        if seq.exprs.len() < 2 {
          panic!("SequenceExpression with less than 2 expressions");
        }
        let first = self.parse_expr(seq.exprs.first().unwrap());
        seq.exprs.iter().skip(1).fold(first, |left, right| {
          self.new_node(
            Node::BinaryExpr,
            vec![
              //
              left,
              Box::new(Expr::Lit(Lit::Str(Str {
                raw: None,
                span: DUMMY_SP,
                value: JsWord::from(","),
              }))),
              self.parse_expr(right),
            ],
            
          )
        })
      }
      Expr::SuperProp(super_prop) => self.new_node(
        Node::PropAccessExpr,
        vec![
          self.new_node(Node::SuperKeyword, vec![]),
          match &super_prop.prop {
            SuperProp::Ident(ident) => self.parse_ident(ident, false),
            SuperProp::Computed(comp) => self.new_node(
              Node::ComputedPropertyNameExpr,
              vec![self.parse_expr(comp.expr.as_ref())],
              
            ),
          },
        ],
      ),
      Expr::TaggedTpl(tagged_template) => self.new_node(
        Node::TaggedTemplateExpr,
        vec![
          self.parse_expr(tagged_template.tag.as_ref()),
          self.parse_template(&tagged_template.tpl),
        ],
      ),
      Expr::This(_this) => self.new_node(Node::ThisExpr, vec![Box::new(expr.clone())]),
      Expr::Tpl(template) => self.new_node(
        Node::TemplateExpr,
        vec![self.parse_template(&template)],
      ),
      // erase <expr> as <type> - take <expr> only
      Expr::TsAs(ts_as) => self.parse_expr(&ts_as.expr),
      // erase <expr> as const - take <expr>
      Expr::TsConstAssertion(as_const) => self.parse_expr(&as_const.expr),
      // const getPerson = get<Person>; // replace with `get`
      Expr::TsInstantiation(ts_instantiation) => self.parse_expr(&ts_instantiation.expr),
      // .prop! // erase the !
      Expr::TsNonNull(ts_non_null) => self.parse_expr(&ts_non_null.expr),
      // <type>expr // erase <type> - take <expr> only
      Expr::TsTypeAssertion(as_type) => self.parse_expr(&as_type.expr),
      Expr::Unary(unary) => match unary.op {
        UnaryOp::TypeOf | UnaryOp::Void | UnaryOp::Delete => self.new_node(
          match unary.op {
            UnaryOp::TypeOf => Node::TypeOfExpr,
            UnaryOp::Void => Node::VoidExpr,
            UnaryOp::Delete => Node::DeleteExpr,
            _ => panic!("impossible"),
          },
          vec![self.parse_expr(unary.arg.as_ref())],
          
        ),
        _ => self.new_node(
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
            self.parse_expr(unary.arg.as_ref()),
          ],
          
        ),
      },
      Expr::Update(update) => self.new_node(
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
          self.parse_expr(update.arg.as_ref()),
        ],
      ),
      Expr::Yield(yield_expr) => self.new_node(
        Node::YieldExpr,
        vec![
          yield_expr
            .arg
            .as_ref()
            .map(|expr| self.parse_expr(expr.as_ref()))
            .unwrap_or(undefined_expr()),
          Box::new(Expr::Lit(Lit::Bool(Bool {
            value: yield_expr.delegate,
            span: DUMMY_SP,
          }))),
        ],
      ),
    }
  }

  fn parse_call_expr(
    &self,
    expr: &Expr,
    args: &[ExprOrSpread],
    is_optional: bool,
  ) -> Box<Expr> {
    self.new_node(
      Node::CallExpr,
      vec![
        //
        self.parse_expr(expr),
        self.parse_call_args(args),
        if is_optional { true_expr() } else { false_expr() }
      ], 
    )
  }

  fn parse_callee(
    &self,
    callee: &Callee,
    args: &[ExprOrSpread],
    is_optional: bool,
  ) -> Box<Expr> {
    self.new_node(
      Node::CallExpr,
      vec![
        //
        match callee {
          Callee::Super(_) => self.new_node(Node::SuperKeyword, vec![]),
          Callee::Import(_) => self.new_node(Node::ImportKeyword, vec![]),
          Callee::Expr(expr) => self.parse_expr(expr),
        },
        self.parse_call_args(args),
        if is_optional { true_expr() } else { false_expr() }
      ],
      
    )
  }

  fn parse_call_args(&self, args: &[ExprOrSpread]) -> Box<Expr> {
    Box::new(Expr::Array(ArrayLit {
      span: DUMMY_SP,
      elems: args
        .iter()
        .map(|arg| {
          Some(ExprOrSpread {
            spread: None,
            expr: if arg.spread.is_some() {
              self.new_node(Node::Argument, vec![
                self.new_node(Node::SpreadElementExpr, vec![self.parse_expr(arg.expr.as_ref())])
              ])
            } else {
              self.new_node(Node::Argument, vec![
                self.parse_expr(arg.expr.as_ref())
              ])
            },
          })
        })
        .collect(),
    }))
  }

  fn parse_member(&self, member: &MemberExpr, is_optional: bool) -> Box<Expr> {
    self.new_node(
      match member.prop {
        MemberProp::Computed(_) =>  Node::ElementAccessExpr,
        _ =>  Node::PropAccessExpr
      },
      vec![
        self.parse_expr(member.obj.as_ref()),
        match &member.prop {
          MemberProp::Ident(ident) => self.parse_ident(ident, false),
          MemberProp::PrivateName(private_name) => self.new_node(
            Node::PrivateIdentifier,
            vec![Box::new(Expr::Lit(Lit::Str(Str {
              raw: None,
              span: DUMMY_SP,
              value: JsWord::from(format!("#{}", private_name.id.sym)),
            })))],
            
          ),
          MemberProp::Computed(comp) => self.parse_expr(comp.expr.as_ref()),
        },
        if is_optional {
          true_expr()
        } else {
          false_expr()
        },
      ],
      
    )
  }

  fn parse_block(&self, block: &BlockStmt) -> Box<Expr> {
    self.new_node(
      Node::BlockStmt,
      vec![
        Box::new(Expr::Array(ArrayLit {
          elems: block
            .stmts
            .iter()
            .map(|stmt| Some(ExprOrSpread {
              expr: self.parse_stmt(stmt),
              spread: None
            }))
            .collect(),
          span: DUMMY_SP
        }))
      ],
    )
  }

  fn parse_class_member(&self, member: &ClassMember) -> Option<Box<Expr>> {
    match member {
      ClassMember::ClassProp(prop) => Some(self.new_node(
        Node::PropDecl,
        vec![
          //
          self.parse_prop_name(&prop.key),
          Box::new(Expr::Lit(Lit::Bool(Bool {
            value: prop.is_static,
            span: DUMMY_SP,
          }))),
          prop
            .value
            .as_ref()
            .map(|v| self.parse_expr(v.as_ref()))
            .unwrap_or(undefined_expr()),
        ],
      )),
      ClassMember::Constructor(ctor) => Some(self.parse_constructor(ctor)),
      ClassMember::Empty(_) => None,
      ClassMember::Method(method) => Some(self.parse_class_method(method)),
      ClassMember::PrivateMethod(method) => Some(self.new_node(
        Node::MethodDecl,
        vec![
          //
          self.new_node(
            Node::PrivateIdentifier,
            vec![
              //
              Box::new(Expr::Lit(Lit::Str(Str {
                raw: None,
                span: DUMMY_SP,
                value: JsWord::from(format!("#{}", method.key.id.sym)),
              }))),
            ],
            
          ),
          self.parse_params(&method.function.params),
          self.parse_block(method.function.body.as_ref().unwrap()),
        ],
      )),
      ClassMember::PrivateProp(prop) => Some(self.new_node(
        Node::PropDecl,
        vec![
          //
          self.new_node(
            Node::PrivateIdentifier,
            vec![
              //
              Box::new(Expr::Lit(Lit::Str(Str {
                raw: None,
                span: DUMMY_SP,
                value: JsWord::from(format!("#{}", prop.key.id.sym)),
              }))),
            ],
            
          ),
          Box::new(Expr::Lit(Lit::Bool(Bool {
            value: prop.is_static,
            span: DUMMY_SP,
          }))),
          prop
            .value
            .as_ref()
            .map(|v| self.parse_expr(v.as_ref()))
            .unwrap_or(undefined_expr()),
        ],
      )),
      ClassMember::StaticBlock(static_block) => Some(self.new_node(
        Node::ClassStaticBlockDecl,
        vec![self.parse_block(&static_block.body)],
      )),
      ClassMember::TsIndexSignature(_) => None,
    }
  }

  fn parse_prop_name(&self, prop: &PropName) -> Box<Expr> {
    match prop {
      PropName::BigInt(i) => self.new_node(
        Node::BigIntExpr,
        vec![Box::new(Expr::Lit(Lit::BigInt(i.clone())))],
      ),
      PropName::Computed(c) => self.new_node(
        Node::ComputedPropertyNameExpr,
        vec![self.parse_expr(c.expr.as_ref())],
      ),
      PropName::Ident(i) => self.parse_ident(i, false),
      PropName::Num(n) => self.new_node(
        Node::NumberLiteralExpr,
        vec![Box::new(Expr::Lit(Lit::Num(n.clone())))],
      ),
      PropName::Str(s) => self.new_node(
        Node::StringLiteralExpr,
        vec![Box::new(Expr::Lit(Lit::Str(s.clone())))],
      ),
    }
  }

  fn parse_var_decl(&self, var_decl: &VarDecl) -> Box<Expr> {
    self.new_node(
      Node::VariableDeclList,
      vec![
        Box::new(Expr::Array(ArrayLit {
          elems: var_decl
            .decls
            .iter()
            .map(|decl| Some(ExprOrSpread {
              expr: self.parse_var_declarator(decl),
              spread: None
            }))
            .collect(),
          span: DUMMY_SP
        }))
      ]
    )
  }

  fn parse_var_declarator(&self, decl: &VarDeclarator) -> Box<Expr> {
    self.new_node(Node::VariableDecl, vec![
      self.parse_pat(&decl.name),
      decl.init.as_ref().map(|init| self.parse_expr(init)).unwrap_or(undefined_expr())
    ])
  }

  fn parse_template(&self, tpl: &Tpl) -> Box<Expr> {
    Box::new(Expr::Array(ArrayLit {
      elems: tpl
        .exprs
        .iter()
        .zip(&tpl.quasis)
        .flat_map(|(expr, quasi)| vec![self.parse_template_element(&quasi), self.parse_expr(expr)])
        .chain(if tpl.quasis.len() > tpl.exprs.len() {
          vec![self.parse_template_element(&tpl.quasis.last().unwrap())]
        } else {
          vec![]
        })
        .map(|expr| Some(ExprOrSpread { expr, spread: None }))
        .collect(),
      span: DUMMY_SP,
    }))
  }

  fn parse_template_element(&self, element: &TplElement) -> Box<Expr> {
    self.new_node(
      Node::QuasiString,
      vec![
        //
        Box::new(Expr::Lit(Lit::Str(Str {
          raw: None,
          span: DUMMY_SP,
          value: element.raw.clone(),
        }))),
      ],
      
    )
  }

  fn parse_ident(&self, ident: &Ident, is_ref: bool) -> Box<Expr> {
    if is_ref && self.vm.is_id_visible(ident) {
      // if this is a free variable, then create a new ReferenceExpr(() => ident)
      self.new_node(Node::ReferenceExpr, vec![
        str(&ident.sym),
        Box::new(Expr::Arrow(ArrowExpr {
          is_async: false,
          is_generator: false,
          params: vec![],
          return_type: None,
          span: DUMMY_SP,
          type_params: None,
          body: BlockStmtOrExpr::Expr(Box::new(Expr::Ident(ident.clone())))
        })),
        num(ident.to_id().1.as_u32()),
        Box::new(Expr::Ident(quote_ident!("__filename")))
      ])
    } else if is_ref && &ident.sym == "undefined" {
      self.new_node(
        Node::UndefinedLiteralExpr,
        vec![],
      )
    } else {
      self.new_node(
        Node::Identifier,
        vec![str(&ident.sym)],
      )
    }
  }

  fn parse_pats_as_params(&self, pats: &[Pat]) -> Box<Expr> {
    Box::new(Expr::Array(ArrayLit {
      elems: pats
        .iter()
        .map(|pat| {
          Some(ExprOrSpread {
            spread: None,
            expr: self.new_node(Node::ParameterDecl, vec![
              self.parse_pat(pat)
            ]),
          })
        })
        .collect(),
      span: DUMMY_SP,
    }))
  }

  fn parse_pats(&self, pats: &[Pat]) -> Box<Expr> {
    Box::new(Expr::Array(ArrayLit {
      elems: pats
        .iter()
        .map(|pat| {
          Some(ExprOrSpread {
            spread: None,
            expr: self.parse_pat(pat),
          })
        })
        .collect(),
      span: DUMMY_SP,
    }))
  }

  fn parse_pat(&self, pat: &Pat) -> Box<Expr> {
    match pat {
      Pat::Array(array_binding) => self.new_node(
        Node::ArrayBinding,
        vec![
          Box::new(Expr::Array(ArrayLit {
            span: DUMMY_SP,
            elems: array_binding
              .elems
              .iter()
              .map(|elem| Some(ExprOrSpread {
                expr: match elem {
                  Some(pat @ Pat::Ident(_)) => self.new_node(
                    Node::BindingElem,
                    vec![self.parse_pat(&pat), false_expr()],
                  ),
                  Some(pat)=> self.parse_pat(pat),
                  None => self.new_node(Node::OmittedExpr, vec![]),
                },
                spread: None
              }))
              .collect(),
          }))
        ]
      ),
      Pat::Object(object_binding) => self.new_node(
        Node::ObjectBinding,
        vec![
          Box::new(Expr::Array(ArrayLit {
            span: DUMMY_SP,
            elems: object_binding
            .props
            .iter()
            .map(|prop| Some(ExprOrSpread {
              spread: None,
              expr: match prop {
                ObjectPatProp::Assign(assign) => self.new_node(
                  Node::BindingElem,
                  match &assign.value {
                    // {key: value}
                    Some(value) => vec![
                      self.parse_ident(&assign.key, true),
                      false_expr(),
                      undefined_expr(),
                      self.parse_expr(value.as_ref()),
                    ],
                    // {key}
                    None => vec![self.parse_ident(&assign.key, false), false_expr()],
                  },
                  
                ),
                // {key: value}
                ObjectPatProp::KeyValue(kv) => self.new_node(
                  Node::BindingElem,
                  vec![
                    match kv.value.as_ref() {
                      // if this is an assign pattern, e.g. {key = value}
                      // then parse `key` as the `BindingElement.name` in FunctionlessAST
                      Pat::Assign(assign) => self.parse_pat(assign.left.as_ref()),
                      value => self.parse_pat(value),
                    },
                    false_expr(),
                    self.parse_prop_name(&kv.key),
                    match kv.value.as_ref() {
                      // if this is an assign patter, e.g. `{key = value}`
                      // then parse `value` as the `BindingElement.initializer` in FunctionlessAST
                      Pat::Assign(assign) => self.parse_expr(assign.right.as_ref()),
                      _ => undefined_expr(),
                    },
                  ],
                  
                ),
                // { ...rest }
                ObjectPatProp::Rest(rest) => self.new_node(
                  Node::BindingElem,
                  vec![self.parse_pat(&rest.arg), true_expr()],
                ),
              }
            }))
            .collect()
          }))
        ],
      ),
      Pat::Assign(assign) => self.new_node(
        Node::BindingElem,
        vec![
          self.parse_pat(assign.left.as_ref()),
          false_expr(),
          undefined_expr(),
          self.parse_expr(assign.right.as_ref()),
        ],
      ),
      Pat::Expr(expr) => self.parse_expr(expr),
      Pat::Ident(ident) => self.parse_ident(ident, false),
      Pat::Invalid(_invalid) => self.new_error_node("Invalid Node"),
      Pat::Rest(rest) => self.new_node(Node::BindingElem, vec![self.parse_pat(rest.arg.as_ref()), true_expr()]),
    }
  }

  fn new_node(&self, kind: Node, args: Vec<Box<Expr>>) -> Box<Expr> {
    let mut elems: Vec<Option<ExprOrSpread>> = vec![Some(ExprOrSpread {
      expr: Box::new(Expr::Lit(Lit::Num(Number {
        raw: None,
        span: DUMMY_SP,
        value: kind as u32 as f64
      }))),
      spread: None
    })];

    args
    .iter()
    .for_each(|arg| elems.push(Some(ExprOrSpread {
      expr: arg.to_owned(),
      spread: None,
    })));

    Box::new(Expr::Array(ArrayLit {
      span: DUMMY_SP,
      elems: elems
    }))
  }

  fn new_error_node(&self, message: &str) -> Box<Expr> {
    self.new_node(
      Node::Err,
      vec![Box::new(Expr::Lit(Lit::Str(Str {
        raw: None,
        span: DUMMY_SP,
        value: JsWord::from(message),
      })))],
    )
  }
}

// fn arr(elems: Vec<Box<Expr>>) -> Box<Expr> {
//   Box::new(Expr::Array(ArrayLit {
//     elems: elems.iter().map(|expr| Some(ExprOrSpread {
//       expr: expr.clone(),
//       spread: None
//     })).collect(),
//     span: DUMMY_SP
//   }))
// }

fn str(str: &JsWord) -> Box<Expr> {
  Box::new(Expr::Lit(Lit::Str(Str {
    raw: None,
    span: DUMMY_SP,
    value: str.clone(),
  })))
}

fn num(i: u32) -> Box<Expr> {
  Box::new(Expr::Lit(Lit::Num(Number {
    raw: None,
    span: DUMMY_SP,
    value: i as u32 as f64
  })))
}

fn true_expr() -> Box<Expr> {
  Box::new(Expr::Lit(Lit::Bool(Bool {
    span: DUMMY_SP,
    value: true,
  })))
}

fn false_expr() -> Box<Expr> {
  Box::new(Expr::Lit(Lit::Bool(Bool {
    span: DUMMY_SP,
    value: false,
  })))
}

fn undefined_expr() -> Box<Expr> {
  Box::new(Expr::Ident(Ident {
    optional: false,
    span: DUMMY_SP,
    sym: JsWord::from("undefined"),
  }))
}

fn empty_array_expr() -> Box<Expr> {
  Box::new(Expr::Array(ArrayLit {
    elems: vec![],
    span: DUMMY_SP
  }))
}