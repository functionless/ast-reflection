use core::panic;

use swc_common::DUMMY_SP;
use swc_plugin::ast::*;
use swc_plugin::utils::quote_ident;

use crate::ast::Node;
use crate::closure_decorator::ClosureDecorator;

const EMPTY_VEC: Vec<Box<Expr>> = vec![];

impl ClosureDecorator {
  pub fn parse_class_decl(&mut self, class_decl: &ClassDecl) -> Box<Expr> {
    self.parse_class(Node::ClassDecl, Some(&class_decl.ident), &class_decl.class)
  }

  pub fn parse_class_expr(&mut self, class_expr: &ClassExpr) -> Box<Expr> {
    self.parse_class(
      Node::ClassExpr,
      class_expr.ident.as_ref(),
      &class_expr.class,
    )
  }

  fn parse_class(&mut self, kind: Node, ident: Option<&Ident>, class: &Class) -> Box<Expr> {
    new_node(
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

  pub fn parse_constructor(&mut self, ctor: &Constructor) -> Box<Expr> {
    self.vm.bind_constructor_params(&ctor.params);

    new_node(
      Node::ConstructorDecl,
      vec![
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
                    TsParamPropParam::Ident(i) => {
                      new_node(Node::ParameterDecl, vec![self.parse_ident(&i, false)])
                    }
                    TsParamPropParam::Assign(i) => new_node(
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
        self.parse_block(ctor.body.as_ref().unwrap()),
      ],
    )
  }

  pub fn parse_function_decl(&mut self, function: &FnDecl) -> Box<Expr> {
    self.parse_function(
      Node::FunctionDecl,
      Some(&function.ident),
      &function.function,
    )
  }

  pub fn parse_function_expr(&mut self, function: &FnExpr) -> Box<Expr> {
    self.parse_function(
      Node::FunctionExpr,
      function.ident.as_ref(),
      &function.function,
    )
  }

  fn parse_function(&mut self, kind: Node, name: Option<&Ident>, function: &Function) -> Box<Expr> {
    self.vm.bind_params(&function.params);

    let params = self.parse_params(&function.params);

    let body = function
      .body
      .as_ref()
      .map(|b| self.parse_block(b))
      .unwrap_or(undefined_expr());

    new_node(
      kind,
      vec![
        name
          .map(|ident| str(&ident.to_id().0))
          .unwrap_or(undefined_expr()),
        params,
        body,
        if function.is_async {
          true_expr()
        } else {
          false_expr()
        },
        if function.is_generator {
          true_expr()
        } else {
          false_expr()
        },
      ],
    )
  }

  pub fn parse_method(&mut self, method: &ClassMethod) -> Box<Expr> {
    self.vm.enter();

    self.vm.bind_params(&method.function.params);

    let node = new_node(
      Node::MethodDecl,
      vec![
        self.parse_prop_name(&method.key),
        self.parse_params(&method.function.params),
        self.parse_block(method.function.body.as_ref().unwrap()),
      ],
    );

    self.vm.exit();

    node
  }

  pub fn parse_private_method(&mut self, method: &PrivateMethod) -> Box<Expr> {
    self.vm.enter();

    self.vm.bind_params(&method.function.params);

    let node = new_node(
      Node::MethodDecl,
      vec![
        self.parse_private_name(&method.key),
        self.parse_params(&method.function.params),
        self.parse_block(method.function.body.as_ref().unwrap()),
      ],
    );

    self.vm.exit();

    node
  }

  pub fn parse_arrow(&mut self, arrow: &ArrowExpr) -> Box<Expr> {
    self.vm.enter();

    self.vm.bind_pats(&arrow.params);

    let node = new_node(
      Node::ArrowFunctionExpr,
      vec![
        self.parse_pats_as_params(&arrow.params),
        match &arrow.body {
          BlockStmtOrExpr::BlockStmt(block) => self.parse_block(block),
          BlockStmtOrExpr::Expr(expr) => new_node(
            Node::BlockStmt,
            vec![Box::new(Expr::Array(ArrayLit {
              elems: vec![Some(ExprOrSpread {
                spread: None,
                expr: new_node(Node::ReturnStmt, vec![self.parse_expr(expr)]),
              })],
              span: DUMMY_SP,
            }))],
          ),
        },
        if arrow.is_async {
          true_expr()
        } else {
          false_expr()
        },
      ],
    );

    self.vm.exit();

    node
  }

  fn parse_pats_as_params(&mut self, pats: &[Pat]) -> Box<Expr> {
    Box::new(Expr::Array(ArrayLit {
      elems: pats
        .iter()
        .map(|pat| {
          Some(ExprOrSpread {
            spread: None,
            expr: self.parse_pat_param(pat),
          })
        })
        .collect(),
      span: DUMMY_SP,
    }))
  }

  fn parse_params(&mut self, params: &[Param]) -> Box<Expr> {
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

  fn parse_param(&mut self, param: &Param) -> Box<Expr> {
    self.parse_pat_param(&param.pat)
  }

  fn parse_pat_param(&mut self, pat: &Pat) -> Box<Expr> {
    new_node(
      Node::ParameterDecl,
      match pat {
        // foo(...a)
        Pat::Rest(rest) => vec![
          self.parse_pat(rest.arg.as_ref()),
          undefined_expr(),
          true_expr(),
        ],
        // foo(a = b)
        Pat::Assign(assign) => vec![
          self.parse_pat(assign.left.as_ref()),
          self.parse_expr(assign.right.as_ref()),
          false_expr(),
        ],
        pat => vec![self.parse_pat(pat), undefined_expr(), false_expr()],
      },
    )
  }

  fn parse_decl(&mut self, decl: &Decl) -> Box<Expr> {
    match decl {
      Decl::Class(class_decl) => self.parse_class_decl(class_decl),
      Decl::Fn(function) => self.parse_function_decl(function),
      Decl::TsEnum(_) => panic!("enums not supported"),
      Decl::TsInterface(_) => panic!("interface not supported"),
      Decl::TsModule(_) => panic!("module declarations not supported"),
      Decl::TsTypeAlias(_) => panic!("type alias not supported"),
      Decl::Var(var_decl) => new_node(Node::VariableStmt, vec![self.parse_var_decl(var_decl)]),
    }
  }

  fn parse_stmt(&mut self, stmt: &Stmt) -> Box<Expr> {
    match stmt {
      Stmt::Block(block) => self.parse_block(block),
      Stmt::Break(break_stmt) => new_node(
        Node::BreakStmt,
        vec![break_stmt
          .label
          .as_ref()
          .map(|label| self.parse_ident(label, false))
          .unwrap_or(undefined_expr())],
      ),
      Stmt::Continue(continue_stmt) => new_node(
        Node::ContinueStmt,
        vec![continue_stmt
          .label
          .as_ref()
          .map(|label| self.parse_ident(label, false))
          .unwrap_or(undefined_expr())],
      ),
      Stmt::Debugger(_debugger) => new_node(Node::DebuggerStmt, EMPTY_VEC),
      Stmt::Decl(decl) => self.parse_decl(decl),
      Stmt::DoWhile(do_while) => new_node(
        Node::DoStmt,
        vec![
          // block
          match do_while.body.as_ref() {
            Stmt::Block(block) => self.parse_block(&block),
            stmt => new_node(
              Node::BlockStmt,
              vec![Box::new(Expr::Array(ArrayLit {
                elems: vec![Some(ExprOrSpread {
                  expr: self.parse_stmt(stmt),
                  spread: None,
                })],
                span: DUMMY_SP,
              }))],
            ),
          },
          // condition
          self.parse_expr(do_while.test.as_ref()),
        ],
      ),
      Stmt::Empty(_empty) => new_node(Node::EmptyStmt, EMPTY_VEC),
      Stmt::Expr(expr_stmt) => new_node(
        Node::ExprStmt,
        vec![
          //expr
          self.parse_expr(expr_stmt.expr.as_ref()),
        ],
      ),
      Stmt::For(for_stmt) => {
        // create a unique scope for the contents for the variables declared in the for-loop's initializer
        self.vm.enter();

        let init = for_stmt
          .init
          .as_ref()
          .map(|init| match init {
            VarDeclOrExpr::Expr(expr) => self.parse_expr(expr.as_ref()),
            VarDeclOrExpr::VarDecl(var) => {
              // bind the for's variable declaration
              // for (let i = 0; ..)
              //          ^
              self.vm.bind_var_decl(var);

              self.parse_var_decl(&var)
            }
          })
          .unwrap_or(undefined_expr());

        let node = new_node(
          Node::ForStmt,
          vec![
            self.parse_stmt(for_stmt.body.as_ref()),
            init,
            for_stmt
              .test
              .as_ref()
              .map(|test| self.parse_expr(test))
              .unwrap_or(undefined_expr()),
            for_stmt
              .update
              .as_ref()
              .map(|update| self.parse_expr(update))
              .unwrap_or(undefined_expr()),
          ],
        );

        // exit the for-loop's initializer scope
        self.vm.exit();

        node
      }
      // for (const left in right)
      Stmt::ForIn(for_in) => {
        // create a unique scope for the contents for the variables declared in the for-in-loop's initializer
        self.vm.enter();

        let var = match &for_in.left {
          VarDeclOrPat::VarDecl(var_decl) => {
            // bind the for-in's variable declaration
            // for (const k in ..)
            //            ^
            self.vm.bind_var_decl(var_decl);

            self.parse_var_declarator(var_decl.decls.first().unwrap())
          }
          //  for (i in items)
          //       ^ not a new name, so no binding created
          VarDeclOrPat::Pat(pat) => self.parse_pat(pat),
        };

        let node = new_node(
          Node::ForInStmt,
          vec![
            var,
            self.parse_expr(&for_in.right),
            self.parse_stmt(for_in.body.as_ref()),
          ],
        );

        // exit the for-in-loop's initializer scope
        self.vm.exit();

        node
      }
      // for (const left of right)
      Stmt::ForOf(for_of) => {
        // create a unique scope for the contents for the variables declared in the for-of-loop's initializer
        self.vm.enter();

        let var = match &for_of.left {
          VarDeclOrPat::VarDecl(var_decl) => {
            // bind the for-of's variable declaration
            // for (const item of ..)
            //             ^
            self.vm.bind_var_decl(var_decl);

            self.parse_var_declarator(var_decl.decls.first().unwrap())
          }
          // for (i of items)
          //      ^ not a new name, so no binding created
          VarDeclOrPat::Pat(pat) => self.parse_pat(pat),
        };

        let node = new_node(
          Node::ForOfStmt,
          vec![
            var,
            self.parse_expr(&for_of.right),
            self.parse_stmt(for_of.body.as_ref()),
            if for_of.await_token.is_some() {
              true_expr()
            } else {
              false_expr()
            },
          ],
        );

        // exit the for-in-loop's initializer scope
        self.vm.exit();

        node
      }
      Stmt::If(if_stmt) => new_node(
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
      Stmt::Labeled(labelled) => new_node(
        Node::LabelledStmt,
        vec![
          self.parse_ident(&labelled.label, false),
          self.parse_stmt(&labelled.body),
        ],
      ),
      Stmt::Return(return_stmt) => new_node(
        Node::ReturnStmt,
        match return_stmt.arg.as_ref() {
          Some(arg) => vec![self.parse_expr(&arg)],
          // encode an empty `return;` as `return undefined();`
          None => vec![],
        },
      ),
      // TODO: support switch - https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Statements/switch
      Stmt::Switch(switch) => new_node(
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
                  elems: case
                    .cons
                    .iter()
                    .map(|stmt| self.parse_stmt(stmt))
                    .map(|expr| Some(ExprOrSpread { expr, spread: None }))
                    .collect(),
                }));

                Some(ExprOrSpread {
                  expr: match case.test.as_ref() {
                    Some(test) => new_node(Node::CaseClause, vec![self.parse_expr(test), stmts]),
                    None => new_node(Node::DefaultClause, vec![stmts]),
                  },
                  spread: None,
                })
              })
              .collect(),
          })),
        ],
      ),
      Stmt::Throw(throw) => new_node(Node::ThrowStmt, vec![self.parse_expr(throw.arg.as_ref())]),
      Stmt::Try(try_stmt) => new_node(
        Node::TryStmt,
        vec![
          self.parse_block(&try_stmt.block),
          try_stmt
            .handler
            .as_ref()
            .map(|catch| {
              // create a scope for the catch block including the catch variable decl
              self.vm.enter();

              // if the catch has a variable decl, bind it to lexical scope
              catch.param.iter().for_each(|pat| self.vm.bind_pat(pat));

              let node = new_node(
                Node::CatchClause,
                vec![
                  match &catch.param {
                    Some(pat) => new_node(
                      Node::VariableDecl,
                      match pat {
                        Pat::Assign(assign) => {
                          vec![self.parse_pat(&assign.left), self.parse_expr(&assign.right)]
                        }
                        _ => vec![self.parse_pat(pat)],
                      },
                    ),
                    None => undefined_expr(),
                  },
                  self.parse_block(&catch.body),
                ],
              );

              // exit the catch's block
              self.vm.exit();

              node
            })
            .unwrap_or(undefined_expr()),
          try_stmt
            .finalizer
            .as_ref()
            .map(|finalizer| self.parse_block(&finalizer))
            .unwrap_or(undefined_expr()),
        ],
      ),
      Stmt::While(while_stmt) => new_node(
        Node::WhileStmt,
        vec![
          self.parse_expr(while_stmt.test.as_ref()),
          match while_stmt.body.as_ref() {
            Stmt::Block(block) => self.parse_block(&block),
            stmt => new_node(
              Node::BlockStmt,
              vec![Box::new(Expr::Array(ArrayLit {
                elems: vec![Some(ExprOrSpread {
                  expr: self.parse_stmt(stmt),
                  spread: None,
                })],
                span: DUMMY_SP,
              }))],
            ),
          },
        ],
      ),
      // TODO: support with https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Statements/with
      Stmt::With(with) => new_node(
        Node::WithStmt,
        vec![
          self.parse_expr(with.obj.as_ref()),
          self.parse_stmt(with.body.as_ref()),
        ],
      ),
    }
  }

  fn parse_expr(&mut self, expr: &Expr) -> Box<Expr> {
    match expr {
      Expr::Array(array) => new_node(
        Node::ArrayLiteralExpr,
        vec![Box::new(Expr::Array(ArrayLit {
          elems: array
            .elems
            .iter()
            .map(|element| {
              Some(ExprOrSpread {
                expr: match element {
                  Some(e) => {
                    if e.spread.is_some() {
                      new_node(
                        Node::SpreadElementExpr,
                        vec![self.parse_expr(e.expr.as_ref())],
                      )
                    } else {
                      self.parse_expr(e.expr.as_ref())
                    }
                  }
                  None => new_node(Node::OmittedExpr, vec![]),
                },
                spread: None,
              })
            })
            .collect(),
          span: DUMMY_SP,
        }))],
      ),
      Expr::Arrow(arrow) => self.parse_arrow(arrow),
      Expr::Assign(assign) => new_node(
        Node::BinaryExpr,
        vec![
          match &assign.left {
            PatOrExpr::Expr(expr) => self.parse_expr(expr),
            PatOrExpr::Pat(pat) => self.parse_pat(pat),
          },
          str(match assign.op {
            AssignOp::Assign => "=",
            AssignOp::AddAssign => "+=",
            AssignOp::SubAssign => "-=",
            AssignOp::MulAssign => "*=",
            AssignOp::DivAssign => "/=",
            AssignOp::ModAssign => "%=",
            AssignOp::LShiftAssign => "<<=",
            AssignOp::RShiftAssign => ">>=",
            AssignOp::ZeroFillRShiftAssign => ">>>=",
            AssignOp::BitOrAssign => "|=",
            AssignOp::BitXorAssign => "^=",
            AssignOp::BitAndAssign => "&=",
            AssignOp::ExpAssign => "**=",
            AssignOp::AndAssign => "&&=",
            AssignOp::OrAssign => "||=",
            AssignOp::NullishAssign => "??=",
          }),
          self.parse_expr(assign.right.as_ref()),
        ],
      ),
      Expr::Await(a_wait) => new_node(Node::AwaitExpr, vec![self.parse_expr(a_wait.arg.as_ref())]),
      Expr::Bin(binary_op) => new_node(
        Node::BinaryExpr,
        vec![
          self.parse_expr(binary_op.left.as_ref()),
          str(match binary_op.op {
            BinaryOp::Add => "+",
            BinaryOp::BitAnd => "&",
            BinaryOp::BitOr => "|",
            BinaryOp::BitXor => "^",
            BinaryOp::Div => "/",
            BinaryOp::EqEq => "==",
            BinaryOp::EqEqEq => "===",
            BinaryOp::Exp => "**",
            BinaryOp::Gt => ">",
            BinaryOp::GtEq => ">=",
            BinaryOp::In => "in",
            BinaryOp::InstanceOf => "instanceof",
            BinaryOp::LogicalAnd => "&&",
            BinaryOp::LogicalOr => "||",
            BinaryOp::LShift => "<<",
            BinaryOp::Lt => "<",
            BinaryOp::LtEq => "<=",
            BinaryOp::Mod => "%",
            BinaryOp::Mul => "*",
            BinaryOp::NotEq => "!=",
            BinaryOp::NotEqEq => "!==",
            BinaryOp::NullishCoalescing => "??",
            BinaryOp::RShift => ">>",
            BinaryOp::Sub => "-",
            BinaryOp::ZeroFillRShift => ">>>",
          }),
          self.parse_expr(binary_op.right.as_ref()),
        ],
      ),
      Expr::Call(call) => self.parse_callee(&call.callee, &call.args, false),
      // TODO: extract properties from ts-parameters
      Expr::Class(class_expr) => self.parse_class_expr(class_expr),
      Expr::Cond(cond) => new_node(
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
      Expr::Invalid(_invalid) => new_error_node("Syntax Error"),
      Expr::JSXElement(_jsx_element) => new_error_node("not sure what to do with JSXElement"),
      Expr::JSXEmpty(_jsx_empty) => new_error_node("not sure what to do with JSXEmpty"),
      Expr::JSXFragment(_jsx_fragment) => new_error_node("not sure what to do with JSXFragment"),
      Expr::JSXMember(_jsx_member) => new_error_node("not sure what to do with JSXMember"),
      Expr::JSXNamespacedName(_jsx_namespace_name) => {
        new_error_node("not sure what to do with JSXNamespacedName")
      }
      Expr::Lit(literal) => match &literal {
        // not sure what type of node this is, will just error for now
        Lit::JSXText(_) => new_error_node("not sure what to do with JSXText"),
        _ => new_node(
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
      Expr::MetaProp(_meta_prop) => new_error_node("MetaProp is not supported"),
      Expr::New(call) => new_node(
        Node::NewExpr,
        vec![
          //
          self.parse_expr(&call.callee),
          call
            .args
            .as_ref()
            .map(|args| self.parse_call_args(args))
            .unwrap_or(empty_array_expr()),
        ],
      ),
      Expr::Object(object) => new_node(
        Node::ObjectLiteralExpr,
        vec![Box::new(Expr::Array(ArrayLit {
          span: DUMMY_SP,
          elems: object
            .props
            .iter()
            .map(|prop| match prop {
              PropOrSpread::Prop(prop) => match prop.as_ref() {
                // invalid according to SWC's docs on Prop::Assign
                Prop::Assign(_assign) => panic!("Invalid Syntax in Object Literal"),
                Prop::Getter(getter) => new_node(
                  Node::GetAccessorDecl,
                  vec![
                    self.parse_prop_name(&getter.key),
                    self.parse_block(&getter.body.as_ref().unwrap()),
                  ],
                ),
                Prop::KeyValue(assign) => new_node(
                  Node::PropAssignExpr,
                  vec![
                    self.parse_prop_name(&assign.key),
                    self.parse_expr(assign.value.as_ref()),
                  ],
                ),
                Prop::Method(method) => new_node(
                  Node::MethodDecl,
                  vec![
                    //
                    self.parse_prop_name(&method.key),
                    self.parse_params(&method.function.params),
                    self.parse_block(method.function.body.as_ref().unwrap()),
                  ],
                ),
                Prop::Setter(setter) => new_node(
                  Node::SetAccessorDecl,
                  vec![
                    self.parse_prop_name(&setter.key),
                    self.parse_pat(&setter.param),
                    self.parse_block(setter.body.as_ref().unwrap()),
                  ],
                ),
                Prop::Shorthand(ident) => new_node(
                  Node::PropAssignExpr,
                  vec![
                    self.parse_ident(ident, false),
                    self.parse_ident(ident, true),
                  ],
                ),
              },
              PropOrSpread::Spread(spread) => new_node(
                Node::SpreadAssignExpr,
                vec![self.parse_expr(spread.expr.as_ref())],
              ),
            })
            .map(|prop| {
              Some(ExprOrSpread {
                expr: prop,
                spread: None,
              })
            })
            .collect(),
        }))],
      ),
      Expr::OptChain(opt_chain) => match &opt_chain.base {
        OptChainBase::Call(call) => self.parse_call_expr(&call.callee, &call.args, true),
        OptChainBase::Member(member) => self.parse_member(&member, true),
      },
      Expr::Paren(paren) => new_node(
        Node::ParenthesizedExpr,
        vec![self.parse_expr(paren.expr.as_ref())],
      ),
      Expr::PrivateName(private_name) => self.parse_private_name(private_name),
      Expr::Seq(seq) => {
        if seq.exprs.len() < 2 {
          panic!("SequenceExpression with less than 2 expressions");
        }
        let first = self.parse_expr(seq.exprs.first().unwrap());
        seq.exprs.iter().skip(1).fold(first, |left, right| {
          new_node(
            Node::BinaryExpr,
            vec![
              //
              left,
              str(","),
              self.parse_expr(right),
            ],
          )
        })
      }
      Expr::SuperProp(super_prop) => new_node(
        Node::PropAccessExpr,
        vec![
          new_node(Node::SuperKeyword, vec![]),
          match &super_prop.prop {
            SuperProp::Ident(ident) => self.parse_ident(ident, false),
            SuperProp::Computed(comp) => new_node(
              Node::ComputedPropertyNameExpr,
              vec![self.parse_expr(comp.expr.as_ref())],
            ),
          },
        ],
      ),
      Expr::Tpl(tpl) => self.parse_template(tpl),
      Expr::TaggedTpl(tagged_template) => new_node(
        Node::TaggedTemplateExpr,
        vec![
          self.parse_expr(&tagged_template.tag),
          self.parse_template(&tagged_template.tpl),
        ],
      ),
      Expr::This(_this) => new_node(Node::ThisExpr, vec![Box::new(expr.clone())]),
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
        UnaryOp::TypeOf | UnaryOp::Void | UnaryOp::Delete => new_node(
          match unary.op {
            UnaryOp::TypeOf => Node::TypeOfExpr,
            UnaryOp::Void => Node::VoidExpr,
            UnaryOp::Delete => Node::DeleteExpr,
            _ => panic!("impossible"),
          },
          vec![self.parse_expr(unary.arg.as_ref())],
        ),
        _ => new_node(
          Node::UnaryExpr,
          vec![
            // op
            str(match unary.op {
              UnaryOp::Minus => "-",
              UnaryOp::Plus => "+",
              UnaryOp::Bang => "!",
              UnaryOp::Tilde => "~",
              UnaryOp::TypeOf => panic!("unexpected typeof operator"),
              UnaryOp::Void => panic!("unexpected void operator"),
              UnaryOp::Delete => panic!("unexpected delete operator"),
            }),
            // expr
            self.parse_expr(unary.arg.as_ref()),
          ],
        ),
      },
      Expr::Update(update) => new_node(
        if update.prefix {
          Node::UnaryExpr
        } else {
          Node::PostfixUnaryExpr
        },
        vec![
          // op
          str(match update.op {
            UpdateOp::PlusPlus => "++",
            UpdateOp::MinusMinus => "--",
          }),
          // expr
          self.parse_expr(update.arg.as_ref()),
        ],
      ),
      Expr::Yield(yield_expr) => new_node(
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
    &mut self,
    expr: &Expr,
    args: &[ExprOrSpread],
    is_optional: bool,
  ) -> Box<Expr> {
    new_node(
      Node::CallExpr,
      vec![
        //
        self.parse_expr(expr),
        self.parse_call_args(args),
        if is_optional {
          true_expr()
        } else {
          false_expr()
        },
      ],
    )
  }

  fn parse_callee(
    &mut self,
    callee: &Callee,
    args: &[ExprOrSpread],
    is_optional: bool,
  ) -> Box<Expr> {
    new_node(
      Node::CallExpr,
      vec![
        //
        match callee {
          Callee::Super(_) => new_node(Node::SuperKeyword, vec![]),
          Callee::Import(_) => new_node(Node::ImportKeyword, vec![]),
          Callee::Expr(expr) => self.parse_expr(expr),
        },
        self.parse_call_args(args),
        if is_optional {
          true_expr()
        } else {
          false_expr()
        },
      ],
    )
  }

  fn parse_call_args(&mut self, args: &[ExprOrSpread]) -> Box<Expr> {
    Box::new(Expr::Array(ArrayLit {
      span: DUMMY_SP,
      elems: args
        .iter()
        .map(|arg| {
          Some(ExprOrSpread {
            spread: None,
            expr: if arg.spread.is_some() {
              new_node(
                Node::Argument,
                vec![new_node(
                  Node::SpreadElementExpr,
                  vec![self.parse_expr(arg.expr.as_ref())],
                )],
              )
            } else {
              new_node(Node::Argument, vec![self.parse_expr(arg.expr.as_ref())])
            },
          })
        })
        .collect(),
    }))
  }

  fn parse_member(&mut self, member: &MemberExpr, is_optional: bool) -> Box<Expr> {
    new_node(
      match member.prop {
        MemberProp::Computed(_) => Node::ElementAccessExpr,
        _ => Node::PropAccessExpr,
      },
      vec![
        self.parse_expr(member.obj.as_ref()),
        match &member.prop {
          MemberProp::Ident(ident) => self.parse_ident(ident, false),
          MemberProp::PrivateName(private_name) => self.parse_private_name(private_name),
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

  fn parse_block(&mut self, block: &BlockStmt) -> Box<Expr> {
    self.vm.enter();

    self.vm.bind_block(block);

    let node = new_node(
      Node::BlockStmt,
      vec![Box::new(Expr::Array(ArrayLit {
        elems: block
          .stmts
          .iter()
          .map(|stmt| {
            Some(ExprOrSpread {
              expr: self.parse_stmt(stmt),
              spread: None,
            })
          })
          .collect(),
        span: DUMMY_SP,
      }))],
    );

    self.vm.exit();

    node
  }

  fn parse_class_member(&mut self, member: &ClassMember) -> Option<Box<Expr>> {
    match member {
      ClassMember::ClassProp(prop) => Some(new_node(
        Node::PropDecl,
        vec![
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
      ClassMember::Method(method) => Some(self.parse_method(method)),
      ClassMember::PrivateMethod(method) => Some(self.parse_private_method(method)),
      ClassMember::PrivateProp(prop) => Some(new_node(
        Node::PropDecl,
        vec![
          self.parse_private_name(&prop.key),
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
      ClassMember::StaticBlock(static_block) => Some(new_node(
        Node::ClassStaticBlockDecl,
        vec![self.parse_block(&static_block.body)],
      )),
      ClassMember::TsIndexSignature(_) => None,
    }
  }

  fn parse_prop_name(&mut self, prop: &PropName) -> Box<Expr> {
    match prop {
      PropName::BigInt(i) => new_node(
        Node::BigIntExpr,
        vec![Box::new(Expr::Lit(Lit::BigInt(i.clone())))],
      ),
      PropName::Computed(c) => new_node(
        Node::ComputedPropertyNameExpr,
        vec![self.parse_expr(c.expr.as_ref())],
      ),
      PropName::Ident(i) => self.parse_ident(i, false),
      PropName::Num(n) => new_node(
        Node::NumberLiteralExpr,
        vec![Box::new(Expr::Lit(Lit::Num(n.clone())))],
      ),
      PropName::Str(s) => new_node(
        Node::StringLiteralExpr,
        vec![Box::new(Expr::Lit(Lit::Str(s.clone())))],
      ),
    }
  }

  fn parse_private_name(&mut self, name: &PrivateName) -> Box<Expr> {
    new_node(
      Node::PrivateIdentifier,
      vec![str(&format!("#{}", name.id.sym))],
    )
  }

  fn parse_var_decl(&mut self, var_decl: &VarDecl) -> Box<Expr> {
    new_node(
      Node::VariableDeclList,
      vec![
        Box::new(Expr::Array(ArrayLit {
          elems: var_decl
            .decls
            .iter()
            .map(|decl| {
              Some(ExprOrSpread {
                expr: self.parse_var_declarator(decl),
                spread: None,
              })
            })
            .collect(),
          span: DUMMY_SP,
        })),
        num(match var_decl.kind {
          VarDeclKind::Const => 0,
          VarDeclKind::Let => 1,
          VarDeclKind::Var => 2,
        }),
      ],
    )
  }

  fn parse_var_declarator(&mut self, decl: &VarDeclarator) -> Box<Expr> {
    new_node(
      Node::VariableDecl,
      vec![
        self.parse_pat(&decl.name),
        decl
          .init
          .as_ref()
          .map(|init| self.parse_expr(init))
          .unwrap_or(undefined_expr()),
      ],
    )
  }

  fn parse_template(&mut self, tpl: &Tpl) -> Box<Expr> {
    if tpl.exprs.len() == 0 {
      new_node(
        Node::NoSubstitutionTemplateLiteral,
        vec![str(&tpl.quasis.first().unwrap().raw)],
      )
    } else {
      new_node(
        Node::TemplateExpr,
        vec![
          new_node(
            Node::TemplateHead,
            vec![str(&tpl.quasis.first().unwrap().raw)],
          ),
          Box::new(Expr::Array(ArrayLit {
            span: DUMMY_SP,
            elems: tpl
              .exprs
              .iter()
              .zip(tpl.quasis.iter().skip(1))
              .map(|(expr, literal)| {
                Some(ExprOrSpread {
                  expr: new_node(
                    Node::TemplateSpan,
                    vec![
                      // expr
                      self.parse_expr(expr),
                      // literal
                      new_node(
                        if literal.tail {
                          Node::TemplateTail
                        } else {
                          Node::TemplateMiddle
                        },
                        vec![str(&literal.raw)],
                      ),
                    ],
                  ),
                  spread: None,
                })
              })
              .collect(),
          })),
        ],
      )
    }
  }

  fn parse_ident(&self, ident: &Ident, is_ref: bool) -> Box<Expr> {
    if is_ref && &ident.sym == "undefined" {
      new_node(Node::UndefinedLiteralExpr, vec![])
    } else if is_ref && !self.vm.is_id_visible(ident) {
      // if this is a free variable, then create a new ReferenceExpr(() => ident)
      new_node(
        Node::ReferenceExpr,
        vec![
          str(&ident.sym),
          Box::new(Expr::Arrow(ArrowExpr {
            is_async: false,
            is_generator: false,
            params: vec![],
            return_type: None,
            span: DUMMY_SP,
            type_params: None,
            body: BlockStmtOrExpr::Expr(Box::new(Expr::Ident(ident.clone()))),
          })),
          num(ident.to_id().1.as_u32()),
          Box::new(Expr::Ident(quote_ident!("__filename"))),
        ],
      )
    } else {
      new_node(Node::Identifier, vec![str(&ident.sym)])
    }
  }

  fn parse_pat(&mut self, pat: &Pat) -> Box<Expr> {
    match pat {
      Pat::Array(array_binding) => new_node(
        Node::ArrayBinding,
        vec![Box::new(Expr::Array(ArrayLit {
          span: DUMMY_SP,
          elems: array_binding
            .elems
            .iter()
            .map(|elem| {
              Some(ExprOrSpread {
                expr: match elem {
                  Some(pat @ Pat::Ident(_)) => {
                    new_node(Node::BindingElem, vec![self.parse_pat(&pat), false_expr()])
                  }
                  Some(pat) => self.parse_pat(pat),
                  None => new_node(Node::OmittedExpr, vec![]),
                },
                spread: None,
              })
            })
            .collect(),
        }))],
      ),
      Pat::Object(object_binding) => new_node(
        Node::ObjectBinding,
        vec![Box::new(Expr::Array(ArrayLit {
          span: DUMMY_SP,
          elems: object_binding
            .props
            .iter()
            .map(|prop| {
              Some(ExprOrSpread {
                spread: None,
                expr: match prop {
                  ObjectPatProp::Assign(assign) => new_node(
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
                  ObjectPatProp::KeyValue(kv) => new_node(
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
                  ObjectPatProp::Rest(rest) => new_node(
                    Node::BindingElem,
                    vec![self.parse_pat(&rest.arg), true_expr()],
                  ),
                },
              })
            })
            .collect(),
        }))],
      ),
      Pat::Assign(assign) => new_node(
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
      Pat::Invalid(_invalid) => new_error_node("Invalid Node"),
      Pat::Rest(rest) => new_node(
        Node::BindingElem,
        vec![self.parse_pat(rest.arg.as_ref()), true_expr()],
      ),
    }
  }
}

fn new_node(kind: Node, args: Vec<Box<Expr>>) -> Box<Expr> {
  let mut elems: Vec<Option<ExprOrSpread>> = vec![Some(ExprOrSpread {
    expr: Box::new(Expr::Lit(Lit::Num(Number {
      raw: None,
      span: DUMMY_SP,
      value: kind as u32 as f64,
    }))),
    spread: None,
  })];

  args.iter().for_each(|arg| {
    elems.push(Some(ExprOrSpread {
      expr: arg.to_owned(),
      spread: None,
    }))
  });

  Box::new(Expr::Array(ArrayLit {
    span: DUMMY_SP,
    elems: elems,
  }))
}

fn new_error_node(message: &str) -> Box<Expr> {
  new_node(Node::Err, vec![str(message)])
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

fn str(str: &str) -> Box<Expr> {
  Box::new(Expr::Lit(Lit::Str(Str {
    raw: None,
    span: DUMMY_SP,
    value: JsWord::from(str),
  })))
}

fn num(i: u32) -> Box<Expr> {
  Box::new(Expr::Lit(Lit::Num(Number {
    raw: None,
    span: DUMMY_SP,
    value: i as u32 as f64,
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
    span: DUMMY_SP,
  }))
}
