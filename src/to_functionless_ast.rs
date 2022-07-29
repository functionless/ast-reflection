use core::panic;

use strum_macros::AsRefStr;
use swc_common::DUMMY_SP;
use swc_plugin::{
  ast::*,
  utils::{private_ident, quote_ident},
};


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

pub fn parse_closure(node: &Expr) -> Box<Expr> {
  let parser = FunctionlessASTParser {
    import: private_ident!("functionless"),
  };

  parser.parse_expr(node)
}

pub struct FunctionlessASTParser {
  import: Ident,
}

impl FunctionlessASTParser {
  pub fn parse_decl(&self, decl: &Decl) -> Box<Expr> {
    match decl {
      Decl::Class(class_decl) => self.newNode(
        Node::ClassDecl,
        vec![
          //
          self.parse_ident(&class_decl.ident),
          class_decl
            .class
            .super_class
            .as_ref()
            .map(|sup| self.parse_expr(sup.as_ref()))
            .unwrap_or(Box::new(undefined())),
          Box::new(Expr::Array(ArrayLit {
            elems: class_decl
              .class
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
      ),
      Decl::Fn(function) => self.newNode(
        Node::FunctionDecl,
        vec![Box::new(Expr::Array(ArrayLit {
          span: DUMMY_SP,
          elems: function
            .function
            .params
            .iter()
            .map(|param| {
              Some(ExprOrSpread {
                expr: self.newNode(
                  Node::ParameterDecl,
                  vec![self.parse_pattern(&param.pat)],
                  
                ),
                spread: None,
              })
            })
            .collect(),
        }))],
      ),
      Decl::TsEnum(ts_enum) => panic!("enums not supported"),
      Decl::TsInterface(interface) => panic!("interface not supported"),
      Decl::TsModule(module) => panic!("module declarations not supported"),
      Decl::TsTypeAlias(type_alias) => panic!("type alias not supported"),
      Decl::Var(var_decl) => self.parse_var_decl(var_decl),
    }
  }

  fn parse_stmt(&self, stmt: &Stmt) -> Box<Expr> {
    match stmt {
      Stmt::Block(block) => self.parse_block(block),
      Stmt::Break(break_stmt) => self.newNode(Node::BreakStmt, EMPTY_VEC),
      Stmt::Continue(continue_stmt) => self.newNode(Node::ContinueStmt, EMPTY_VEC),
      Stmt::Debugger(debugger) => self.newNode(Node::DebuggerStmt, EMPTY_VEC),
      Stmt::Decl(decl) => self.parse_decl(decl),
      Stmt::DoWhile(do_while) => self.newNode(
        Node::WhileStmt,
        vec![
          // condition
          self.parse_expr(do_while.test.as_ref()),
          // block
          self.parse_stmt(do_while.body.as_ref()),
        ],
      ),
      Stmt::Empty(empty) => self.newNode(Node::EmptyStmt, EMPTY_VEC),
      Stmt::Expr(expr_stmt) => self.newNode(
        Node::ExprStmt,
        vec![
          //expr
          self.parse_expr(expr_stmt.expr.as_ref()),
        ],
      ),
      // TODO
      Stmt::For(for_stmt) => self.newNode(
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
            .unwrap_or(Box::new(undefined())),
        ], 
      ),
      // for (const left in right)
      Stmt::ForIn(for_in) => self.newNode(
        Node::ForInStmt,
        vec![
          match &for_in.left {
            VarDeclOrPat::VarDecl(var_decl) => self.parse_var_decl(var_decl),
            VarDeclOrPat::Pat(pat) => self.parse_pattern(pat),
          },
          self.parse_expr(&for_in.right),
          self.parse_stmt(for_in.body.as_ref()),
        ],
      ),
      // for (const left of right)
      Stmt::ForOf(for_of) => self.newNode(
        Node::ForInStmt,
        vec![
          match &for_of.left {
            VarDeclOrPat::VarDecl(var_decl) => self.parse_var_decl(var_decl),
            VarDeclOrPat::Pat(pat) => self.parse_pattern(pat),
          },
          self.parse_expr(&for_of.right),
          self.parse_stmt(for_of.body.as_ref()),
        ],
      ),
      Stmt::If(if_stmt) => self.newNode(
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
            .unwrap_or(Box::new(undefined())),
        ],
      ),
      // https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Statements/label
      // for now, we just erase the label
      Stmt::Labeled(labelled) => self.newNode(
        Node::LabelledStmt,
        vec![self.parse_stmt(&labelled.body)],
      ),
      Stmt::Return(return_stmt) => self.newNode(
        Node::ReturnStmt,
        vec![match return_stmt.arg.as_ref() {
          Some(arg) => self.parse_expr(&arg),
          // encode an empty `return;` as `return undefined();`
          None => self.newNode(Node::UndefinedLiteralExpr, EMPTY_VEC),
        }],
      ),
      // TODO: support switch - https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Statements/switch
      Stmt::Switch(switch) => self.newNode(
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
                let stmts: Vec<Box<Expr>> =
                  case.cons.iter().map(|stmt| self.parse_stmt(stmt)).collect();
                //
                Some(ExprOrSpread {
                  expr: match case.test.as_ref() {
                    Some(test) => self.newNode(Node::CaseClause, stmts.clone()),
                    None => self.newNode(Node::DefaultClause, stmts.clone()),
                  },
                  spread: None,
                })
              })
              .collect(),
          })),
        ],
      ),
      Stmt::Throw(throw) => self.newNode(
        Node::ThrowStmt,
        vec![self.parse_expr(throw.arg.as_ref())],
      ),
      Stmt::Try(try_stmt) => self.newNode(
        Node::TryStmt,
        vec![
          self.parse_block(&try_stmt.block),
          try_stmt
            .handler
            .as_ref()
            .map(|catch| match &catch.param {
              Some(pat) => self.parse_pattern(pat),
              None => Box::new(undefined()),
            })
            .unwrap_or(Box::new(undefined())),
          try_stmt
            .finalizer
            .as_ref()
            .map(|finalizer| self.parse_block(&finalizer))
            .unwrap_or(Box::new(undefined())),
        ],
      ),
      Stmt::While(while_stmt) => self.newNode(
        Node::WhileStmt,
        vec![
          self.parse_expr(while_stmt.test.as_ref()),
          self.parse_stmt(while_stmt.body.as_ref()),
        ],
      ),
      // TODO: support with https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Statements/with
      Stmt::With(with) => self.newNode(
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
      Expr::Array(array) => self.newNode(
        Node::ArrayLiteralExpr,
        array
          .elems
          .iter()
          .filter_map(|element| element.as_ref().map(|e| self.parse_expr(e.expr.as_ref())))
          .collect(),
      ),
      Expr::Arrow(arrow) => self.newNode(
        Node::ArrowFunctionExpr,
        vec![self.parse_patterns(&arrow.params)],
      ),
      Expr::Assign(assign) => self.newNode(
        Node::BinaryExpr,
        vec![
          match &assign.left {
            PatOrExpr::Expr(expr) => self.parse_expr(expr),
            PatOrExpr::Pat(pat) => self.parse_pattern(pat),
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
          self.parse_expr(assign.right.as_ref()),
        ],
      ),
      Expr::Await(a_wait) => self.newNode(
        Node::AwaitExpr,
        vec![self.parse_expr(a_wait.arg.as_ref())],
      ),
      Expr::Bin(binary_op) => self.newNode(
        Node::BinaryExpr,
        vec![
          self.parse_expr(binary_op.left.as_ref()),
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
          self.parse_expr(binary_op.right.as_ref()),
        ],
      ),
      Expr::Call(call) => self.parse_callee(&call.callee, &call.args, false),
      // TODO: extract properties from ts-parameters
      Expr::Class(class_expr) => self.newNode(
        Node::ClassExpr,
        vec![
          //
          class_expr
            .ident
            .as_ref()
            .map(|i| self.parse_ident(&i))
            .unwrap_or(Box::new(undefined())),
          class_expr
            .class
            .super_class
            .as_ref()
            .map(|sup| self.parse_expr(sup.as_ref()))
            .unwrap_or(Box::new(undefined())),
          Box::new(Expr::Array(ArrayLit {
            elems: class_expr
              .class
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
      ),
      Expr::Cond(cond) => self.newNode(
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
      Expr::Fn(function_expr) => self.newNode(
        Node::FunctionExpr,
        vec![
          // params
          self.parse_params(&function_expr.function.params),
        ],
      ),
      // TODO: check if is in scope and convert to a ReferenceExpr
      Expr::Ident(id) => self.parse_ident(id),
      Expr::Invalid(invalid) => self.newErrorNode("Syntax Error"),
      Expr::JSXElement(jsx_element) => self.newErrorNode("not sure what to do with JSXElement"),
      Expr::JSXEmpty(jsx_empty) => self.newErrorNode("not sure what to do with JSXEmpty"),
      Expr::JSXFragment(jsx_fragment) => self.newErrorNode("not sure what to do with JSXFragment"),
      Expr::JSXMember(jsx_member) => self.newErrorNode("not sure what to do with JSXMember"),
      Expr::JSXNamespacedName(jsx_namespace_name) => {
        self.newErrorNode("not sure what to do with JSXNamespacedName")
      }
      Expr::Lit(literal) => match &literal {
        // not sure what type of node this is, will just error for now
        Lit::JSXText(_) => self.newErrorNode("not sure what to do with JSXText"),
        _ => self.newNode(
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
          
        ),
      },
      Expr::Member(member) => self.parse_member(member, false),
      Expr::MetaProp(meta_prop) => self.newErrorNode("MetaProp is not supported"),
      Expr::New(call) => self.newNode(
        Node::NewExpr,
        vec![
          //
          self.parse_expr(&call.callee),
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
                        self.newNode(Node::Argument, vec![])
                      } else {
                        self.newNode(Node::Argument, vec![])
                      },
                    })
                  })
                  .collect()
              })
              .unwrap_or(vec![]),
          })),
        ],
      ),
      Expr::Object(object) => self.newNode(
        Node::ObjectLiteralExpr,
        object
          .props
          .iter()
          .map(|prop| match prop {
            PropOrSpread::Prop(prop) => match prop.as_ref() {
              // invalid according to SWC's docs on Prop::Assign
              Prop::Assign(assign) => panic!("Invalid Syntax in Object Literal"),
              Prop::Getter(getter) => self.newNode(
                Node::GetAccessorDecl,
                vec![
                  self.parse_prop_name(&getter.key),
                  self.parse_block(&getter.body.as_ref().unwrap()),
                ],
                
              ),
              Prop::KeyValue(assign) => self.newNode(
                Node::PropAssignExpr,
                vec![
                  self.parse_prop_name(&assign.key),
                  self.parse_expr(assign.value.as_ref()),
                ],
                
              ),
              Prop::Method(method) => self.newNode(
                Node::MethodDecl,
                vec![
                  //
                  self.parse_prop_name(&method.key),
                  self.parse_params(&method.function.params),
                  self.parse_block(method.function.body.as_ref().unwrap()),
                ],
                
              ),
              Prop::Setter(setter) => self.newNode(
                Node::SetAccessorDecl,
                vec![
                  self.parse_prop_name(&setter.key),
                  self.parse_pattern(&setter.param),
                  self.parse_block(setter.body.as_ref().unwrap()),
                ],
                
              ),
              Prop::Shorthand(ident) => self.newNode(
                Node::PropAssignExpr,
                vec![self.parse_ident(ident), self.parse_ident(ident)],
                
              ),
            },
            PropOrSpread::Spread(spread) => self.newNode(
              Node::SpreadAssignExpr,
              vec![self.parse_expr(spread.expr.as_ref())],
              
            ),
          })
          .collect(),
      ),
      Expr::OptChain(opt_chain) => match &opt_chain.base {
        OptChainBase::Call(call) => self.parse_call_expr(&call.callee, &call.args, true),
        OptChainBase::Member(member) => self.parse_member(&member, true),
      },
      Expr::Paren(paren) => self.newNode(
        Node::ParenthesizedExpr,
        vec![self.parse_expr(paren.expr.as_ref())],
      ),
      Expr::PrivateName(private_name) => self.newNode(
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
          self.newNode(
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
      Expr::SuperProp(super_prop) => self.newNode(
        Node::PropAccessExpr,
        vec![
          self.newNode(Node::SuperKeyword, vec![]),
          match &super_prop.prop {
            SuperProp::Ident(ident) => self.parse_ident(ident),
            SuperProp::Computed(comp) => self.newNode(
              Node::ComputedPropertyNameExpr,
              vec![self.parse_expr(comp.expr.as_ref())],
              
            ),
          },
        ],
      ),
      Expr::TaggedTpl(tagged_template) => self.newNode(
        Node::TaggedTemplateExpr,
        vec![
          self.parse_expr(tagged_template.tag.as_ref()),
          self.parse_template(&tagged_template.tpl),
        ],
      ),
      Expr::This(this) => self.newNode(Node::ThisExpr, vec![Box::new(expr.clone())]),
      Expr::Tpl(template) => self.newNode(
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
        UnaryOp::TypeOf | UnaryOp::Void | UnaryOp::Delete => self.newNode(
          match unary.op {
            UnaryOp::TypeOf => Node::TypeOfExpr,
            UnaryOp::Void => Node::VoidExpr,
            UnaryOp::Delete => Node::DeleteExpr,
            _ => panic!("impossible"),
          },
          vec![self.parse_expr(unary.arg.as_ref())],
          
        ),
        _ => self.newNode(
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
      Expr::Update(update) => self.newNode(
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
      Expr::Yield(yield_expr) => self.newNode(
        Node::YieldExpr,
        vec![
          yield_expr
            .arg
            .as_ref()
            .map(|expr| self.parse_expr(expr.as_ref()))
            .unwrap_or(Box::new(undefined())),
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
    self.newNode(
      Node::CallExpr,
      vec![
        //
        self.parse_expr(expr),
        Box::new(Expr::Array(ArrayLit {
          span: DUMMY_SP,
          elems: args
            .iter()
            .map(|arg| {
              Some(ExprOrSpread {
                spread: None,
                expr: if arg.spread.is_some() {
                  // TODO: we can't represent this right now
                  self.newNode(Node::Argument, vec![])
                } else {
                  self.newNode(Node::Argument, vec![])
                },
              })
            })
            .collect(),
        })),
      ],
      
    )
  }

  fn parse_callee(
    &self,
    callee: &Callee,
    args: &[ExprOrSpread],
    is_optional: bool,
  ) -> Box<Expr> {
    self.newNode(
      Node::CallExpr,
      vec![
        //
        match callee {
          Callee::Super(sup) => self.newNode(Node::SuperKeyword, vec![]),
          Callee::Import(_) => self.newNode(Node::ImportKeyword, vec![]),
          Callee::Expr(expr) => self.parse_expr(expr),
        },
        self.parse_call_args(args),
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
              // TODO: we can't represent this right now
              self.newNode(Node::Argument, vec![])
            } else {
              self.newNode(Node::Argument, vec![])
            },
          })
        })
        .collect(),
    }))
  }

  fn parse_member(&self, member: &MemberExpr, is_optional: bool) -> Box<Expr> {
    self.newNode(
      Node::PropAccessExpr,
      vec![
        self.parse_expr(member.obj.as_ref()),
        match &member.prop {
          MemberProp::Ident(ident) => self.parse_ident(ident),
          MemberProp::PrivateName(private_name) => self.newNode(
            Node::PrivateIdentifier,
            vec![Box::new(Expr::Lit(Lit::Str(Str {
              raw: None,
              span: DUMMY_SP,
              value: JsWord::from(format!("#{}", private_name.id.sym)),
            })))],
            
          ),
          MemberProp::Computed(comp) => self.newNode(
            Node::ComputedPropertyNameExpr,
            vec![self.parse_expr(comp.expr.as_ref())],
            
          ),
        },
        if is_optional {
          Box::new(True)
        } else {
          Box::new(False)
        },
      ],
      
    )
  }

  fn parse_block(&self, block: &BlockStmt) -> Box<Expr> {
    self.newNode(
      Node::BlockStmt,
      block
        .stmts
        .iter()
        .map(|stmt| self.parse_stmt(stmt))
        .collect(),
      
    )
  }

  fn parse_class_member(&self, member: &ClassMember) -> Option<Box<Expr>> {
    match member {
      ClassMember::ClassProp(prop) => Some(self.newNode(
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
            .unwrap_or(Box::new(undefined())),
        ],
      )),
      ClassMember::Constructor(ctor) => Some(self.newNode(
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
                      TsParamPropParam::Ident(i) => self.newNode(
                        Node::ParameterDecl,
                        vec![
                          //
                          self.parse_ident(&i),
                        ],
                        
                      ),
                      TsParamPropParam::Assign(i) => self.newNode(
                        Node::ParameterDecl,
                        vec![
                          self.parse_pattern(i.left.as_ref()),
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
      )),
      ClassMember::Empty(_) => None,
      ClassMember::Method(method) => Some(self.newNode(
        Node::MethodDecl,
        vec![
          //
          self.parse_prop_name(&method.key),
          self.parse_params(&method.function.params),
          self.parse_block(method.function.body.as_ref().unwrap()),
        ],
      )),
      ClassMember::PrivateMethod(method) => Some(self.newNode(
        Node::MethodDecl,
        vec![
          //
          self.newNode(
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
      ClassMember::PrivateProp(prop) => Some(self.newNode(
        Node::PropDecl,
        vec![
          //
          self.newNode(
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
            .unwrap_or(Box::new(undefined())),
        ],
      )),
      ClassMember::StaticBlock(static_block) => Some(self.newNode(
        Node::ClassStaticBlockDecl,
        vec![self.parse_block(&static_block.body)],
      )),
      ClassMember::TsIndexSignature(_) => None,
    }
  }

  fn parse_prop_name(&self, prop: &PropName) -> Box<Expr> {
    match prop {
      PropName::BigInt(i) => self.newNode(
        Node::BigIntExpr,
        vec![Box::new(Expr::Lit(Lit::BigInt(i.clone())))],
      ),
      PropName::Computed(c) => self.newNode(
        Node::ComputedPropertyNameExpr,
        vec![self.parse_expr(c.expr.as_ref())],
      ),
      PropName::Ident(i) => self.parse_ident(i),
      PropName::Num(n) => self.newNode(
        Node::BigIntExpr,
        vec![Box::new(Expr::Lit(Lit::Num(n.clone())))],
      ),
      PropName::Str(s) => self.newNode(
        Node::BigIntExpr,
        vec![Box::new(Expr::Lit(Lit::Str(s.clone())))],
      ),
    }
  }

  fn parse_var_decl(&self, var_decl: &VarDecl) -> Box<Expr> {
    self.newNode(
      Node::VariableStmt,
      vec![self.newNode(
        Node::VariableDeclList,
        var_decl
          .decls
          .iter()
          .filter_map(|decl| match &decl.name {
            Pat::Ident(ident) => Some(self.parse_ident(&ident.id)),
            _ => None,
          })
          .collect(),
      )],
      
    )
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
    self.newNode(
      Node::StringLiteralExpr,
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

  fn parse_ident(&self, ident: &Ident) -> Box<Expr> {
    self.newNode(
      Node::Identifier,
      vec![self.newNode(
        Node::StringLiteralExpr,
        vec![Box::new(Expr::Lit(Lit::Str(Str {
          raw: None,
          span: DUMMY_SP,
          value: ident.sym.clone(),
        })))],
      )],
      
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
    self.parse_pattern(&param.pat)
  }

  fn parse_patterns(&self, pats: &[Pat]) -> Box<Expr> {
    Box::new(Expr::Array(ArrayLit {
      elems: pats
        .iter()
        .map(|pat| {
          Some(ExprOrSpread {
            spread: None,
            expr: self.parse_pattern(pat),
          })
        })
        .collect(),
      span: DUMMY_SP,
    }))
  }

  fn parse_pattern(&self, pat: &Pat) -> Box<Expr> {
    match pat {
      Pat::Array(array_binding) => self.newNode(
        Node::ArrayBinding,
        array_binding
          .elems
          .iter()
          .filter_map(|e| e.as_ref())
          .map(|elem| match elem {
            Pat::Ident(ident) => self.newNode(
              Node::BindingElem,
              vec![self.parse_pattern(&elem), Box::new(False)],
              
            ),
            _ => self.parse_pattern(&elem),
          })
          .collect(),
      ),
      Pat::Object(object_binding) => self.newNode(
        Node::ObjectBinding,
        object_binding
          .props
          .iter()
          .map(|prop| match prop {
            ObjectPatProp::Assign(assign) => self.newNode(
              Node::BindingElem,
              match &assign.value {
                // {key: value}
                Some(value) => vec![
                  self.parse_expr(value.as_ref()),
                  Box::new(False),
                  Box::new(Expr::Ident(assign.key.clone())),
                ],
                // {key}
                None => vec![Box::new(Expr::Ident(assign.key.clone())), Box::new(False)],
              },
              
            ),
            // {key: value}
            ObjectPatProp::KeyValue(kv) => self.newNode(
              Node::BindingElem,
              vec![
                match kv.value.as_ref() {
                  // if this is an assign pattern, e.g. {key = value}
                  // then parse `key` as the `BindingElement.name` in FunctionlessAST
                  Pat::Assign(assign) => self.parse_pattern(assign.left.as_ref()),
                  value => self.parse_pattern(value),
                },
                Box::new(False),
                self.parse_prop_name(&kv.key),
                match kv.value.as_ref() {
                  // if this is an assign patter, e.g. `{key = value}`
                  // then parse `value` as the `BindingElement.initializer` in FunctionlessAST
                  Pat::Assign(assign) => self.parse_expr(assign.right.as_ref()),
                  _ => Box::new(undefined()),
                },
              ],
              
            ),
            // { ...rest }
            ObjectPatProp::Rest(rest) => self.newNode(
              Node::BindingElem,
              vec![self.parse_pattern(&rest.arg), Box::new(True)],
              
            ),
          })
          .collect(),
      ),
      Pat::Assign(assign) => self.newNode(
        Node::BindingElem,
        vec![
          self.parse_pattern(assign.left.as_ref()),
          Box::new(False),
          Box::new(undefined()),
          self.parse_expr(assign.right.as_ref()),
        ],
      ),
      Pat::Expr(expr) => self.parse_expr(expr),
      Pat::Ident(ident) => self.parse_ident(ident),
      Pat::Invalid(invalid) => panic!("Invalid Syntax"),
      Pat::Rest(rest) => self.newNode(Node::BindingElem, vec![]),
    }
  }

  fn newNode(&self, kind: Node, args: Vec<Box<Expr>>) -> Box<Expr> {
    Box::new(Expr::Call(CallExpr {
      callee: Callee::Expr(Box::new(Expr::Member(MemberExpr {
        obj: Box::new(Expr::Ident(self.import.clone())),
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

  fn newErrorNode(&self, message: &str) -> Box<Expr> {
    self.newNode(
      Node::Err,
      vec![Box::new(Expr::Lit(Lit::Str(Str {
        raw: None,
        span: DUMMY_SP,
        value: JsWord::from(message),
      })))],
    )
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
