use core::panic;

use swc_core::ast::*;
use swc_core::common::{SourceMapper, Span, Spanned, DUMMY_SP};
use swc_core::plugin::proxies::PluginSourceMapProxy;
use swc_core::utils::quote_ident;

use crate::ast::Node;
use crate::class_like::ClassLike;
use crate::closure_decorator::ClosureDecorator;
use crate::js_util::*;
use crate::method_like::MethodLike;
use crate::span::*;

const EMPTY_VEC: Vec<Box<Expr>> = vec![];

impl<'a> ClosureDecorator<'a> {
  /**
   * Parse a [ClassDe&cl](ClassDecl) or [ClassExpr](ClassExpr) into its FunctionlessAST form.
   */
  pub fn parse_class_like<T>(&mut self, class_like: &T, is_root: bool) -> Box<Expr>
  where
    T: ClassLike,
  {
    new_node(
      self.source_map,
      class_like.kind(),
      &class_like.class().span,
      vec![
        class_like
          .name()
          .as_ref()
          .map(|i| self.parse_ident(&i, false))
          .unwrap_or(undefined_expr()),
        class_like
          .class()
          .super_class
          .as_ref()
          .map(|sup| self.parse_expr(sup.as_ref()))
          .unwrap_or(undefined_expr()),
        Box::new(Expr::Array(ArrayLit {
          elems: class_like
            .class()
            .body
            .iter()
            .map(|member| {
              self
                .parse_class_member(
                  member,
                  if is_root {
                    Some(ref_expr(self.source_map, this_expr()))
                  } else {
                    None
                  },
                )
                .map(|expr| ExprOrSpread { expr, spread: None })
            })
            .collect(),
          span: DUMMY_SP,
        })),
        if is_root {
          __filename()
        } else {
          undefined_expr()
        },
      ],
    )
  }

  pub fn parse_ctor(&mut self, ctor: &Constructor) -> Box<Expr> {
    self.vm.enter();

    self.vm.bind_constructor_params(&ctor.params);

    let node = new_node(
      self.source_map,
      Node::ConstructorDecl,
      &ctor.span,
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
                    TsParamPropParam::Ident(i) => new_node(
                      self.source_map,
                      Node::ParameterDecl,
                      &p.span,
                      vec![self.parse_ident(&i, false)],
                    ),
                    TsParamPropParam::Assign(i) => new_node(
                      self.source_map,
                      Node::ParameterDecl,
                      &p.span,
                      vec![
                        self.parse_pat(i.left.as_ref(), false),
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
    );

    self.vm.exit();

    node
  }

  pub fn parse_function_decl(&mut self, function: &FnDecl, is_root: bool) -> Box<Expr> {
    self.vm.enter();

    self.vm.bind_ident(&function.ident);

    let node = self.parse_function(
      Node::FunctionDecl,
      Some(&function.ident),
      &function.function,
      is_root,
    );

    self.vm.exit();

    node
  }

  pub fn parse_function_expr(&mut self, function: &FnExpr, is_root: bool) -> Box<Expr> {
    self.vm.enter();

    function
      .ident
      .iter()
      .for_each(|ident| self.vm.bind_ident(ident));

    let node = self.parse_function(
      Node::FunctionExpr,
      function.ident.as_ref(),
      &function.function,
      is_root,
    );

    self.vm.exit();

    node
  }

  fn parse_function(
    &mut self,
    kind: Node,
    name: Option<&Ident>,
    function: &Function,
    is_root: bool,
  ) -> Box<Expr> {
    self.vm.bind_params(&function.params);

    let params = self.parse_params(&function.params);

    let body = function
      .body
      .as_ref()
      .map(|b| self.parse_block(b))
      .unwrap_or(undefined_expr());

    new_node(
      self.source_map,
      kind,
      &function.span,
      vec![
        name
          .map(|ident| string_expr(&ident.to_id().0))
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
        if is_root {
          __filename()
        } else {
          undefined_expr()
        },
      ],
    )
  }

  /**
   * Parse a [ClassMethod](ClassMethod) to its AST form.
   * ```ts
   * class Foo {
   *   // MethodDecl(Identifier("method"), BlockStmt([]), isAsync: false, isGenerator: false)
   *   method() {}
   * }
   * ```
   */
  pub fn parse_method_like<M>(
    &mut self,
    method: &M,
    owned_by: Option<Box<Expr>>,
    fallback_span: &Span,
  ) -> Box<Expr>
  where
    M: MethodLike,
  {
    self.vm.enter();

    self.vm.bind_params(method.function().params.as_slice());

    let node = match method.kind() {
      MethodKind::Method => new_node(
        self.source_map,
        Node::MethodDecl,
        method.span().unwrap_or(fallback_span),
        vec![
          self.parse_prop_name(&method.key()),
          self.parse_params(&method.function().params),
          self.parse_block(method.function().body.as_ref().unwrap()),
          bool_expr(method.function().is_async),
          bool_expr(method.function().is_generator),
          __filename(),
          bool_expr(method.is_static()),
          owned_by.unwrap_or(undefined_expr()),
        ],
      ),
      MethodKind::Getter => new_node(
        self.source_map,
        Node::GetAccessorDecl,
        &method.span().unwrap_or(fallback_span),
        vec![
          self.parse_prop_name(&method.key()),
          self.parse_block(method.function().body.as_ref().unwrap()),
          bool_expr(method.is_static()),
          owned_by.unwrap_or(undefined_expr()),
        ],
      ),
      MethodKind::Setter => new_node(
        self.source_map,
        Node::SetAccessorDecl,
        &method.span().unwrap_or(fallback_span),
        vec![
          self.parse_prop_name(&method.key()),
          method
            .function()
            .params
            .first()
            .map(|param| self.parse_param(param))
            .unwrap_or_else(undefined_expr),
          self.parse_block(method.function().body.as_ref().unwrap()),
          bool_expr(method.is_static()),
          owned_by.unwrap_or(undefined_expr()),
        ],
      ),
    };

    self.vm.exit();

    node
  }

  pub fn parse_private_method(
    &mut self,
    method: &PrivateMethod,
    owned_by: Option<Box<Expr>>,
  ) -> Box<Expr> {
    self.vm.enter();

    self.vm.bind_params(&method.function.params);

    let node = new_node(
      self.source_map,
      Node::MethodDecl,
      &method.span,
      vec![
        self.parse_private_name(&method.key),
        self.parse_params(&method.function.params),
        self.parse_block(method.function.body.as_ref().unwrap()),
        bool_expr(method.function.is_async),
        bool_expr(method.function.is_generator),
        bool_expr(method.is_static),
        owned_by.unwrap_or(undefined_expr()),
      ],
    );

    self.vm.exit();

    node
  }

  pub fn parse_arrow(&mut self, arrow: &ArrowExpr, is_root: bool) -> Box<Expr> {
    self.vm.enter();

    self.vm.bind_pats(&arrow.params);

    let node = new_node(
      self.source_map,
      Node::ArrowFunctionExpr,
      &arrow.span,
      vec![
        self.parse_pats_as_params(&arrow.params),
        match &arrow.body {
          BlockStmtOrExpr::BlockStmt(block) => self.parse_block(block),
          BlockStmtOrExpr::Expr(expr) => new_node(
            self.source_map,
            Node::BlockStmt,
            get_expr_span(expr),
            vec![Box::new(Expr::Array(ArrayLit {
              elems: vec![Some(ExprOrSpread {
                spread: None,
                expr: new_node(
                  self.source_map,
                  Node::ReturnStmt,
                  get_expr_span(expr),
                  vec![self.parse_expr(expr)],
                ),
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
        if is_root {
          __filename()
        } else {
          undefined_expr()
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
            expr: self.parse_pat_param(pat, None),
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
    self.parse_pat_param(&param.pat, Some(&param.span))
  }

  fn parse_pat_param(&mut self, pat: &Pat, span: Option<&Span>) -> Box<Expr> {
    new_node(
      self.source_map,
      Node::ParameterDecl,
      span.unwrap_or_else(|| get_pat_span(pat)),
      match pat {
        // foo(...a)
        Pat::Rest(rest) => vec![
          self.parse_pat(rest.arg.as_ref(), false),
          undefined_expr(),
          true_expr(),
        ],
        // foo(a = b)
        Pat::Assign(assign) => vec![
          self.parse_pat(assign.left.as_ref(), false),
          self.parse_expr(assign.right.as_ref()),
          false_expr(),
        ],
        pat => vec![self.parse_pat(pat, false), undefined_expr(), false_expr()],
      },
    )
  }

  fn parse_decl(&mut self, decl: &Decl) -> Box<Expr> {
    match decl {
      Decl::Class(class_decl) => self.parse_class_like(class_decl, false),
      Decl::Fn(function) => self.parse_function_decl(function, false),
      Decl::TsEnum(_) => panic!("enums not supported"),
      Decl::TsInterface(_) => panic!("interface not supported"),
      Decl::TsModule(_) => panic!("module declarations not supported"),
      Decl::TsTypeAlias(_) => panic!("type alias not supported"),
      Decl::Var(var_decl) => new_node(
        self.source_map,
        Node::VariableStmt,
        &var_decl.span,
        vec![self.parse_var_decl(var_decl)],
      ),
    }
  }

  fn parse_stmt(&mut self, stmt: &Stmt) -> Box<Expr> {
    match stmt {
      Stmt::Block(block) => self.parse_block(block),
      Stmt::Break(break_stmt) => new_node(
        self.source_map,
        Node::BreakStmt,
        &break_stmt.span,
        vec![break_stmt
          .label
          .as_ref()
          .map(|label| self.parse_ident(label, false))
          .unwrap_or(undefined_expr())],
      ),
      Stmt::Continue(continue_stmt) => new_node(
        self.source_map,
        Node::ContinueStmt,
        &continue_stmt.span,
        vec![continue_stmt
          .label
          .as_ref()
          .map(|label| self.parse_ident(label, false))
          .unwrap_or(undefined_expr())],
      ),
      Stmt::Debugger(debugger) => new_node(
        self.source_map,
        Node::DebuggerStmt,
        &debugger.span,
        EMPTY_VEC,
      ),
      Stmt::Decl(decl) => self.parse_decl(decl),
      Stmt::DoWhile(do_while) => new_node(
        self.source_map,
        Node::DoStmt,
        &do_while.span,
        vec![
          // block
          match do_while.body.as_ref() {
            Stmt::Block(block) => self.parse_block(&block),
            stmt => new_node(
              self.source_map,
              Node::BlockStmt,
              get_stmt_span(stmt),
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
      Stmt::Empty(empty) => new_node(self.source_map, Node::EmptyStmt, &empty.span, EMPTY_VEC),
      Stmt::Expr(expr_stmt) => new_node(
        self.source_map,
        Node::ExprStmt,
        &expr_stmt.span,
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
          self.source_map,
          Node::ForStmt,
          &for_stmt.span,
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

            self.parse_var_decl(var_decl)
          }
          //  for (i in items)
          //       ^ not a new name, so no binding created
          VarDeclOrPat::Pat(pat) => self.parse_pat(pat, true),
        };

        let node = new_node(
          self.source_map,
          Node::ForInStmt,
          &for_in.span,
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

            self.parse_var_decl(var_decl)
          }
          // for (i of items)
          //      ^ not a new name, so no binding created
          VarDeclOrPat::Pat(pat) => self.parse_pat(pat, true),
        };

        let node = new_node(
          self.source_map,
          Node::ForOfStmt,
          &for_of.span,
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
        self.source_map,
        Node::IfStmt,
        &if_stmt.span,
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
        self.source_map,
        Node::LabelledStmt,
        &labelled.span,
        vec![
          self.parse_ident(&labelled.label, false),
          self.parse_stmt(&labelled.body),
        ],
      ),
      Stmt::Return(return_stmt) => new_node(
        self.source_map,
        Node::ReturnStmt,
        &return_stmt.span,
        match return_stmt.arg.as_ref() {
          Some(arg) => vec![self.parse_expr(&arg)],
          // encode an empty `return;` as `return undefined();`
          None => vec![],
        },
      ),
      // TODO: support switch - https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Statements/switch
      Stmt::Switch(switch) => new_node(
        self.source_map,
        Node::SwitchStmt,
        &switch.span,
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
                    Some(test) => new_node(
                      self.source_map,
                      Node::CaseClause,
                      &case.span,
                      vec![self.parse_expr(test), stmts],
                    ),
                    None => new_node(
                      self.source_map,
                      Node::DefaultClause,
                      &case.span,
                      vec![stmts],
                    ),
                  },
                  spread: None,
                })
              })
              .collect(),
          })),
        ],
      ),
      Stmt::Throw(throw) => new_node(
        self.source_map,
        Node::ThrowStmt,
        &throw.span,
        vec![self.parse_expr(throw.arg.as_ref())],
      ),
      Stmt::Try(try_stmt) => new_node(
        self.source_map,
        Node::TryStmt,
        &try_stmt.span,
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
                self.source_map,
                Node::CatchClause,
                &catch.span,
                vec![
                  match &catch.param {
                    Some(pat) => new_node(
                      self.source_map,
                      Node::VariableDecl,
                      get_pat_span(pat),
                      match pat {
                        Pat::Assign(assign) => {
                          vec![
                            self.parse_pat(&assign.left, false),
                            self.parse_expr(&assign.right),
                          ]
                        }
                        _ => vec![self.parse_pat(pat, false)],
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
        self.source_map,
        Node::WhileStmt,
        &while_stmt.span,
        vec![
          self.parse_expr(while_stmt.test.as_ref()),
          match while_stmt.body.as_ref() {
            Stmt::Block(block) => self.parse_block(&block),
            stmt => new_node(
              self.source_map,
              Node::BlockStmt,
              get_stmt_span(stmt),
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
        self.source_map,
        Node::WithStmt,
        &with.span,
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
        self.source_map,
        Node::ArrayLiteralExpr,
        &array.span,
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
                        self.source_map,
                        Node::SpreadElementExpr,
                        &e.spread.unwrap(),
                        vec![self.parse_expr(e.expr.as_ref())],
                      )
                    } else {
                      self.parse_expr(e.expr.as_ref())
                    }
                  }
                  None => new_node(self.source_map, Node::OmittedExpr, &empty_span(), vec![]),
                },
                spread: None,
              })
            })
            .collect(),
          span: DUMMY_SP,
        }))],
      ),
      Expr::Arrow(arrow) => self.parse_arrow(arrow, false),
      Expr::Assign(assign) => new_node(
        self.source_map,
        Node::BinaryExpr,
        &assign.span,
        vec![
          match &assign.left {
            PatOrExpr::Expr(expr) => self.parse_expr(expr),
            PatOrExpr::Pat(pat) => self.parse_pat(pat, true),
          },
          string_expr(match assign.op {
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
      Expr::Await(a_wait) => new_node(
        self.source_map,
        Node::AwaitExpr,
        &a_wait.span,
        vec![self.parse_expr(a_wait.arg.as_ref())],
      ),
      Expr::Bin(binary_op) => new_node(
        self.source_map,
        Node::BinaryExpr,
        &binary_op.span,
        vec![
          self.parse_expr(binary_op.left.as_ref()),
          string_expr(match binary_op.op {
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
      Expr::Call(call) => self.parse_callee(&call.callee, &call.args, false, &call.span),
      // TODO: extract properties from ts-parameters
      Expr::Class(class_expr) => self.parse_class_like(class_expr, false),
      Expr::Cond(cond) => new_node(
        self.source_map,
        Node::ConditionExpr,
        &cond.span,
        vec![
          // when
          self.parse_expr(&cond.test.as_ref()),
          // then
          self.parse_expr(&cond.cons.as_ref()),
          // else
          self.parse_expr(&cond.alt.as_ref()),
        ],
      ),
      Expr::Fn(function) => self.parse_function_expr(&function, false),
      Expr::Ident(id) => self.parse_ident(id, true),
      Expr::Invalid(invalid) => new_error_node(self.source_map, "Syntax Error", &invalid.span),
      Expr::JSXElement(jsx_element) => new_error_node(
        self.source_map,
        "not sure what to do with JSXElement",
        &jsx_element.span,
      ),
      Expr::JSXEmpty(jsx_empty) => new_error_node(
        self.source_map,
        "not sure what to do with JSXEmpty",
        &jsx_empty.span,
      ),
      Expr::JSXFragment(jsx_fragment) => new_error_node(
        self.source_map,
        "not sure what to do with JSXFragment",
        &jsx_fragment.span,
      ),
      Expr::JSXMember(jsx_member) => {
        // TODO: combine spans? this is wrong, why don't these nodes have spans?
        new_error_node(
          self.source_map,
          "not sure what to do with JSXMember",
          &jsx_member.prop.span,
        )
      }
      Expr::JSXNamespacedName(jsx_namespace_name) => new_error_node(
        self.source_map,
        "not sure what to do with JSXNamespacedName",
        // TODO: combine spans? this is wrong, why don't these nodes have spans?
        &jsx_namespace_name.name.span,
      ),
      Expr::Lit(literal) => match &literal {
        // not sure what type of node this is, will just error for now
        Lit::JSXText(j) => {
          new_error_node(self.source_map, "not sure what to do with JSXText", &j.span)
        }
        _ => new_node(
          self.source_map,
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
          get_lit_span(literal),
          vec![Box::new(expr.clone())],
        ),
      },
      Expr::Member(member) => self.parse_member(member, false),
      Expr::MetaProp(meta_prop) => new_error_node(
        self.source_map,
        "MetaProp is not supported",
        &meta_prop.span,
      ),
      Expr::New(new) => new_node(
        self.source_map,
        Node::NewExpr,
        &new.span,
        vec![
          //
          self.parse_expr(&new.callee),
          new
            .args
            .as_ref()
            .map(|args| self.parse_call_args(args))
            .unwrap_or(empty_array_expr()),
        ],
      ),
      Expr::Object(object) => new_node(
        self.source_map,
        Node::ObjectLiteralExpr,
        &object.span,
        vec![Box::new(Expr::Array(ArrayLit {
          span: DUMMY_SP,
          elems: object
            .props
            .iter()
            .map(|prop_or_spread| match prop_or_spread {
              PropOrSpread::Prop(prop) => match prop.as_ref() {
                // invalid according to SWC's docs on Prop::Assign
                Prop::Assign(_assign) => panic!("Invalid Syntax in Object Literal"),
                Prop::Getter(getter) => new_node(
                  self.source_map,
                  Node::GetAccessorDecl,
                  &getter.span,
                  vec![
                    self.parse_prop_name(&getter.key),
                    self.parse_block(&getter.body.as_ref().unwrap()),
                    false_expr(),
                    undefined_expr(),
                  ],
                ),
                Prop::KeyValue(assign) => new_node(
                  self.source_map,
                  Node::PropAssignExpr,
                  &concat_span(
                    get_prop_name_span(&assign.key),
                    get_expr_span(&assign.value),
                  ),
                  vec![
                    self.parse_prop_name(&assign.key),
                    self.parse_expr(assign.value.as_ref()),
                  ],
                ),
                Prop::Method(method) => self.parse_method_like(
                  method,
                  None,
                  &concat_span(&get_prop_name_span(&method.key), &method.function.span),
                ),
                Prop::Setter(setter) => {
                  self.vm.enter();

                  self.vm.bind_pat(&setter.param);

                  let node = new_node(
                    self.source_map,
                    Node::SetAccessorDecl,
                    &setter.span,
                    vec![
                      self.parse_prop_name(&setter.key),
                      self.parse_pat_param(&setter.param, None),
                      self.parse_block(setter.body.as_ref().unwrap()),
                      false_expr(),
                      undefined_expr(),
                    ],
                  );

                  self.vm.exit();

                  node
                }
                Prop::Shorthand(ident) => new_node(
                  self.source_map,
                  Node::PropAssignExpr,
                  &ident.span,
                  vec![
                    self.parse_ident(ident, false),
                    self.parse_ident(ident, true),
                  ],
                ),
              },
              PropOrSpread::Spread(spread) => new_node(
                self.source_map,
                Node::SpreadAssignExpr,
                &concat_span(&spread.dot3_token, get_expr_span(&spread.expr)),
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
        OptChainBase::Call(call) => {
          self.parse_call_expr(&call.callee, &call.args, true, &call.span)
        }
        OptChainBase::Member(member) => self.parse_member(&member, true),
      },
      Expr::Paren(paren) => new_node(
        self.source_map,
        Node::ParenthesizedExpr,
        &paren.span,
        vec![self.parse_expr(paren.expr.as_ref())],
      ),
      Expr::PrivateName(private_name) => self.parse_private_name(private_name),
      Expr::Seq(seq) => {
        let first = self.parse_expr(seq.exprs.first().unwrap());
        seq.exprs.iter().skip(1).fold(first, |left, right| {
          new_node(
            self.source_map,
            Node::BinaryExpr,
            &concat_span(get_expr_span(&left), get_expr_span(&right)),
            vec![
              //
              left,
              string_expr(","),
              self.parse_expr(right),
            ],
          )
        })
      }
      Expr::SuperProp(super_prop) => new_node(
        self.source_map,
        Node::PropAccessExpr,
        &super_prop.span,
        vec![
          new_node(
            self.source_map,
            Node::SuperKeyword,
            &super_prop.obj.span,
            vec![],
          ),
          match &super_prop.prop {
            SuperProp::Ident(ident) => self.parse_ident(ident, false),
            SuperProp::Computed(comp) => new_node(
              self.source_map,
              Node::ComputedPropertyNameExpr,
              &comp.span,
              vec![self.parse_expr(comp.expr.as_ref())],
            ),
          },
        ],
      ),
      Expr::Tpl(tpl) => self.parse_template(tpl),
      Expr::TaggedTpl(tagged_template) => new_node(
        self.source_map,
        Node::TaggedTemplateExpr,
        &tagged_template.span,
        vec![
          self.parse_expr(&tagged_template.tag),
          self.parse_template(&tagged_template.tpl),
        ],
      ),
      Expr::This(this) => new_node(
        self.source_map,
        Node::ThisExpr,
        &this.span,
        vec![arrow_pointer(Box::new(expr.clone()))],
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
        UnaryOp::TypeOf | UnaryOp::Void | UnaryOp::Delete => new_node(
          self.source_map,
          match unary.op {
            UnaryOp::TypeOf => Node::TypeOfExpr,
            UnaryOp::Void => Node::VoidExpr,
            UnaryOp::Delete => Node::DeleteExpr,
            _ => panic!("impossible"),
          },
          &unary.span,
          vec![self.parse_expr(unary.arg.as_ref())],
        ),
        _ => new_node(
          self.source_map,
          Node::UnaryExpr,
          &unary.span,
          vec![
            // op
            string_expr(match unary.op {
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
        self.source_map,
        if update.prefix {
          Node::UnaryExpr
        } else {
          Node::PostfixUnaryExpr
        },
        &update.span,
        vec![
          // op
          string_expr(match update.op {
            UpdateOp::PlusPlus => "++",
            UpdateOp::MinusMinus => "--",
          }),
          // expr
          self.parse_expr(update.arg.as_ref()),
        ],
      ),
      Expr::Yield(yield_expr) => new_node(
        self.source_map,
        Node::YieldExpr,
        &yield_expr.span,
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
    span: &Span,
  ) -> Box<Expr> {
    new_node(
      self.source_map,
      Node::CallExpr,
      span,
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
    span: &Span,
  ) -> Box<Expr> {
    new_node(
      self.source_map,
      Node::CallExpr,
      span,
      vec![
        //
        match callee {
          Callee::Super(s) => new_node(self.source_map, Node::SuperKeyword, &s.span, vec![]),
          Callee::Import(i) => new_node(self.source_map, Node::ImportKeyword, &i.span, vec![]),
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
            expr: match arg.spread {
              Some(_) => new_node(
                self.source_map,
                Node::Argument,
                &arg.span(),
                vec![new_node(
                  self.source_map,
                  Node::SpreadElementExpr,
                  &arg.span(),
                  vec![self.parse_expr(arg.expr.as_ref())],
                )],
              ),
              None => new_node(
                self.source_map,
                Node::Argument,
                &arg.span(),
                vec![self.parse_expr(arg.expr.as_ref())],
              ),
            },
          })
        })
        .collect(),
    }))
  }

  fn parse_member(&mut self, member: &MemberExpr, is_optional: bool) -> Box<Expr> {
    new_node(
      self.source_map,
      match member.prop {
        MemberProp::Computed(_) => Node::ElementAccessExpr,
        _ => Node::PropAccessExpr,
      },
      &member.span,
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
      self.source_map,
      Node::BlockStmt,
      &block.span,
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

  fn parse_class_member(
    &mut self,
    member: &ClassMember,
    owned_by: Option<Box<Expr>>,
  ) -> Option<Box<Expr>> {
    match member {
      ClassMember::ClassProp(prop) => Some(new_node(
        self.source_map,
        Node::PropDecl,
        &prop.span,
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
      ClassMember::Constructor(ctor) => Some(self.parse_ctor(ctor)),
      ClassMember::Empty(_) => None,
      ClassMember::Method(method) => Some(self.parse_method_like(method, owned_by, &method.span)),
      ClassMember::PrivateMethod(method) => Some(self.parse_private_method(method, owned_by)),
      ClassMember::PrivateProp(prop) => Some(new_node(
        self.source_map,
        Node::PropDecl,
        &prop.span,
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
        self.source_map,
        Node::ClassStaticBlockDecl,
        &static_block.span,
        vec![self.parse_block(&static_block.body)],
      )),
      ClassMember::TsIndexSignature(_) => None,
    }
  }

  fn parse_prop_name(&mut self, prop: &PropName) -> Box<Expr> {
    match prop {
      PropName::BigInt(i) => new_node(
        self.source_map,
        Node::BigIntExpr,
        &i.span,
        vec![Box::new(Expr::Lit(Lit::BigInt(i.clone())))],
      ),
      PropName::Computed(c) => new_node(
        self.source_map,
        Node::ComputedPropertyNameExpr,
        &c.span,
        vec![self.parse_expr(c.expr.as_ref())],
      ),
      PropName::Ident(i) => self.parse_ident(i, false),
      PropName::Num(n) => new_node(
        self.source_map,
        Node::NumberLiteralExpr,
        &n.span,
        vec![Box::new(Expr::Lit(Lit::Num(n.clone())))],
      ),
      PropName::Str(s) => new_node(
        self.source_map,
        Node::StringLiteralExpr,
        &s.span,
        vec![Box::new(Expr::Lit(Lit::Str(s.clone())))],
      ),
    }
  }

  fn parse_private_name(&mut self, name: &PrivateName) -> Box<Expr> {
    new_node(
      self.source_map,
      Node::PrivateIdentifier,
      &name.span,
      vec![string_expr(&format!("#{}", name.id.sym))],
    )
  }

  fn parse_var_decl(&mut self, var_decl: &VarDecl) -> Box<Expr> {
    new_node(
      self.source_map,
      Node::VariableDeclList,
      &var_decl.span,
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
        number_i32(match var_decl.kind {
          VarDeclKind::Const => 0,
          VarDeclKind::Let => 1,
          VarDeclKind::Var => 2,
        }),
      ],
    )
  }

  fn parse_var_declarator(&mut self, decl: &VarDeclarator) -> Box<Expr> {
    new_node(
      self.source_map,
      Node::VariableDecl,
      &decl.span,
      vec![
        self.parse_pat(&decl.name, false),
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
        self.source_map,
        Node::NoSubstitutionTemplateLiteral,
        &tpl.span,
        vec![string_expr(&tpl.quasis.first().unwrap().raw)],
      )
    } else {
      new_node(
        self.source_map,
        Node::TemplateExpr,
        &tpl.span,
        vec![
          {
            let head = tpl.quasis.first().unwrap();
            new_node(
              self.source_map,
              Node::TemplateHead,
              &head.span,
              vec![string_expr(&head.raw)],
            )
          },
          Box::new(Expr::Array(ArrayLit {
            span: DUMMY_SP,
            elems: tpl
              .exprs
              .iter()
              .zip(tpl.quasis.iter().skip(1))
              .map(|(expr, literal)| {
                Some(ExprOrSpread {
                  expr: new_node(
                    self.source_map,
                    Node::TemplateSpan,
                    &concat_span(get_expr_span(&expr), &literal.span),
                    vec![
                      // expr
                      self.parse_expr(expr),
                      // literal
                      new_node(
                        self.source_map,
                        if literal.tail {
                          Node::TemplateTail
                        } else {
                          Node::TemplateMiddle
                        },
                        &literal.span,
                        vec![string_expr(&literal.raw)],
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

  fn parse_ident(&mut self, ident: &Ident, is_ref: bool) -> Box<Expr> {
    if is_ref && &ident.sym == "undefined" {
      new_node(
        self.source_map,
        Node::UndefinedLiteralExpr,
        &ident.span,
        vec![],
      )
    } else if is_ref && &ident.sym == "arguments" && !self.vm.is_id_visible(ident) {
      // this is the arguments keyword
      // TODO: check our assumptions, it is only true when inside a function and when
      // no other name has been bound to to that name
      new_node(
        self.source_map,
        Node::Identifier,
        &ident.span,
        vec![string_expr(&ident.sym)],
      )
    } else if is_ref && !self.vm.is_id_visible(ident) {
      // if this is a free variable, then create a new ReferenceExpr(() => ident)
      new_node(
        self.source_map,
        Node::ReferenceExpr,
        &ident.span,
        vec![
          string_expr(&ident.sym),
          Box::new(Expr::Arrow(ArrowExpr {
            is_async: false,
            is_generator: false,
            params: vec![],
            return_type: None,
            span: DUMMY_SP,
            type_params: None,
            body: BlockStmtOrExpr::Expr(Box::new(Expr::Cond(CondExpr {
              test: not_eq_eq(type_of(ident_expr(ident.clone())), string_expr("undefined")),
              cons: ident_expr(ident.clone()),
              alt: ident_expr(quote_ident!("undefined")),
              span: DUMMY_SP,
            }))),
          })),
          number_i32(ident.to_id().1.as_u32() as i32),
          number_u32(self.next_id()),
        ],
      )
    } else {
      new_node(
        self.source_map,
        Node::Identifier,
        &ident.span,
        vec![string_expr(&ident.sym)],
      )
    }
  }

  fn parse_pat(&mut self, pat: &Pat, is_ref: bool) -> Box<Expr> {
    match pat {
      Pat::Array(array_binding) => new_node(
        self.source_map,
        Node::ArrayBinding,
        &array_binding.span,
        vec![Box::new(Expr::Array(ArrayLit {
          span: DUMMY_SP,
          elems: array_binding
            .elems
            .iter()
            .map(|elem| {
              Some(ExprOrSpread {
                expr: match elem {
                  Some(pat) => match pat {
                    Pat::Assign(_) => self.parse_pat(pat, is_ref),
                    Pat::Rest(_) => self.parse_pat(pat, is_ref),
                    _ => new_node(
                      self.source_map,
                      Node::BindingElem,
                      get_pat_span(pat),
                      vec![self.parse_pat(pat, is_ref), false_expr()],
                    ),
                  },
                  None => new_node(self.source_map, Node::OmittedExpr, &empty_span(), vec![]),
                },
                spread: None,
              })
            })
            .collect(),
        }))],
      ),
      Pat::Object(object_binding) => new_node(
        self.source_map,
        Node::ObjectBinding,
        &object_binding.span,
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
                    self.source_map,
                    Node::BindingElem,
                    &assign.span,
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
                    self.source_map,
                    Node::BindingElem,
                    &concat_span(get_prop_name_span(&kv.key), get_pat_span(&kv.value)),
                    vec![
                      match kv.value.as_ref() {
                        // if this is an assign pattern, e.g. {key = value}
                        // then parse `key` as the `BindingElement.name` in FunctionlessAST
                        Pat::Assign(assign) => self.parse_pat(assign.left.as_ref(), is_ref),
                        value => self.parse_pat(value, is_ref),
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
                    self.source_map,
                    Node::BindingElem,
                    &rest.span,
                    vec![self.parse_pat(&rest.arg, is_ref), true_expr()],
                  ),
                },
              })
            })
            .collect(),
        }))],
      ),
      Pat::Assign(assign) => new_node(
        self.source_map,
        Node::BindingElem,
        &assign.span,
        vec![
          self.parse_pat(assign.left.as_ref(), is_ref),
          false_expr(),
          undefined_expr(),
          self.parse_expr(assign.right.as_ref()),
        ],
      ),
      Pat::Expr(expr) => self.parse_expr(expr),
      Pat::Ident(ident) => self.parse_ident(ident, is_ref),
      Pat::Invalid(invalid) => new_error_node(self.source_map, "Invalid Node", &invalid.span),
      Pat::Rest(rest) => new_node(
        self.source_map,
        Node::BindingElem,
        &rest.span,
        vec![self.parse_pat(rest.arg.as_ref(), is_ref), true_expr()],
      ),
    }
  }
}

pub fn new_node(
  source_map: &PluginSourceMapProxy,
  kind: Node,
  span: &Span,
  args: Vec<Box<Expr>>,
) -> Box<Expr> {
  let (line, col) = if span.lo().0 == 0 {
    // lookup_char_pos has a terrible interface because it panics on a position of 0
    // see: https://github.com/swc-project/swc/issues/2757
    // see: https://github.com/swc-project/swc/issues/5535
    // TODO: investigate how we get here with a 0 span - a DUMMY_SP should never be used as the source of a parsed node
    //       -> may be related to why we're getting broken source maps?
    (1, 0)
  } else {
    let loc = source_map.lookup_char_pos(span.lo());
    (loc.line as u32, loc.col_display as u32)
  };

  let elems: Vec<Option<ExprOrSpread>> = [
    // kind
    Some(ExprOrSpread {
      expr: number_i32(kind as i32),
      spread: None,
    }),
    // span
    Some(ExprOrSpread {
      spread: None,
      expr: Box::new(Expr::Array(ArrayLit {
        span: DUMMY_SP,
        elems: vec![
          // line
          Some(ExprOrSpread {
            expr: number_u32(line),
            spread: None,
          }),
          // col
          Some(ExprOrSpread {
            expr: number_u32(col),
            spread: None,
          }),
        ],
      })),
    }),
  ]
  .into_iter()
  .chain(args.into_iter().map(|arg| {
    Some(ExprOrSpread {
      expr: arg,
      spread: None,
    })
  }))
  .collect();

  Box::new(Expr::Array(ArrayLit {
    span: DUMMY_SP,
    elems,
  }))
}

fn new_error_node(source_map: &PluginSourceMapProxy, message: &str, span: &Span) -> Box<Expr> {
  new_node(source_map, Node::Err, span, vec![string_expr(message)])
}
