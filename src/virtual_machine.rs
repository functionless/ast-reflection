use im::HashSet;
use swc_plugin::{ast::*, utils::StmtLike};

/**
 * A mapping of [reference name](JsWord) to the [unique id](u32) of that reference.
 */
pub type Names = HashSet<Id>;

#[derive(Clone)]
pub struct VirtualMachine {
  pub stack: Stack,
}

impl VirtualMachine {
  pub fn new() -> VirtualMachine {
    VirtualMachine {
      stack: Stack::from(vec![Names::new()]),
    }
  }
}

pub type Stack = Vec<Names>;

impl VirtualMachine {
  pub fn is_id_visible(&self, ident: &Ident) -> bool {
    self.lookup_id(ident.to_id())
  }

  pub fn lookup_id(&self, id: Id) -> bool {
    self
      .stack
      .last()
      .map(|lex| lex.contains(&id))
      .unwrap_or(false)
  }

  /**
   * Push a new [scope](Scope) onto the stack and inherit names from the parent scope.
   *
   * ## Case 1 - enter [Block Scope](Scope::Block)
   *
   * Only variables from the parent's [Block Scope](Scope::Block) are inherited.
   *
   * ```ts
   * let a;
   *
   * function foo() {
   *   a; // visible - in both Block and Function scope
   *   b; // visible - only in Function scope
   * }
   *
   * let b;
   * ```
   *
   * ## Case 2 - enter [Function Scope](Scope::Function)
   *
   * All variables from the parent's [Function Scope](Scope::Function) are inherited.
   *
   * ```ts
   * let a;
   *
   * function foo() {
   *   a; // visible - in both Block and Function scope
   *   b; // visible - only in Function scope
   * }
   *
   * let b;
   * ```
   */
  pub fn enter(&mut self) {
    // cloning immutable hash setss is fast - O(1)
    // data sharing of immutable data structures should also be memory efficient - O(lg n)
    self.stack.push(match self.stack.last() {
      Some(parent) => parent.clone(),
      None => Names::new(),
    });
  }

  pub fn exit(&mut self) -> Names {
    match self.stack.pop() {
      Some(popped) => popped,
      None => panic!("stack underflow"),
    }
  }

  pub fn bind_id(&mut self, id: Id) {
    let names = self.stack.last_mut().expect("stack underflow");
    *names = names.update(id);
  }

  pub fn bind_ident(&mut self, ident: &Ident) {
    self.bind_id(ident.to_id());
  }

  pub fn bind_module_items(&mut self, items: &[ModuleItem]) {
    items.iter().for_each(|item| match item {
      ModuleItem::ModuleDecl(decl) => self.bind_module_decl(decl),
      ModuleItem::Stmt(stmt) => self.bind_stmt(stmt),
    });
  }

  pub fn bind_module_decl(&mut self, decl: &ModuleDecl) {
    match decl {
      ModuleDecl::Import(import) => import.specifiers.iter().for_each(|spec| match spec {
        ImportSpecifier::Default(default) => self.bind_ident(&default.local),
        ImportSpecifier::Named(name) => self.bind_ident(&name.local),
        ImportSpecifier::Namespace(namespace) => self.bind_ident(&namespace.local),
      }),
      _ => {}
    }
  }

  pub fn bind_block(&mut self, block: &BlockStmt) {
    self.bind_stmts(&block.stmts);
  }

  pub fn bind_stmts<T>(&mut self, stmts: &[T])
  where
    T: StmtLike,
  {
    stmts
      .iter()
      .filter_map(|stmt| stmt.as_stmt())
      .for_each(|stmt| self.bind_stmt(stmt));
  }

  fn bind_stmt(&mut self, stmt: &Stmt) {
    match stmt {
      Stmt::Decl(Decl::Class(class_decl)) => self.bind_class_decl(class_decl),
      Stmt::Decl(Decl::Var(var)) => self.bind_var_decl(var),
      Stmt::Decl(Decl::Fn(func)) => self.bind_ident(&func.ident),
      _ => {}
    }
  }

  pub fn bind_class_decl(&mut self, class_decl: &ClassDecl) {
    self.bind_ident(&class_decl.ident);
  }

  /**
   * Bind names produced by a [VarDeclarator](VarDeclarator)
   */
  pub fn bind_var_decl(&mut self, var: &VarDecl) {
    var.decls.iter().for_each(|decl| {
      self.bind_var_declarator(decl);
    })
  }

  /**
   * Bind names produced by a [VarDeclarator](VarDeclarator)
   */
  pub fn bind_var_declarator(&mut self, decl: &VarDeclarator) {
    self.bind_pat(&decl.name);
  }

  pub fn bind_constructor_params(&mut self, params: &[ParamOrTsParamProp]) {
    params.iter().for_each(|param| match param {
      ParamOrTsParamProp::Param(param) => self.bind_param(param),
      ParamOrTsParamProp::TsParamProp(param) => match &param.param {
        TsParamPropParam::Ident(ident) => self.bind_ident(ident),
        TsParamPropParam::Assign(assign) => self.bind_assign(assign),
      },
    });
  }

  pub fn bind_params(&mut self, params: &[Param]) {
    params.iter().for_each(|param| self.bind_param(&param));
  }

  pub fn bind_param(&mut self, param: &Param) {
    self.bind_pat(&param.pat);
  }

  pub fn bind_pats(&mut self, pats: &[Pat]) {
    pats.iter().for_each(|p| self.bind_pat(p));
  }

  /**
   * Binds the names produced by a [binding pattern](Pat) to the current [lexical scope](LexicalScope).
   *
   * ```ts
   * // patterns:
   * a
   * {b}
   * {d: c}
   * [d];
   * ```
   */
  pub fn bind_pat(&mut self, pat: &Pat) {
    match pat {
      // (a, b) => {}
      Pat::Ident(ident) => {
        self.bind_ident(&ident.id);
      }
      // ({a, b: c}) => {}
      Pat::Object(o) => {
        for prop in o.props.iter() {
          match prop {
            ObjectPatProp::Assign(a) => {
              self.bind_ident(&a.key);
            }
            ObjectPatProp::KeyValue(kv) => {
              self.bind_pat(kv.value.as_ref());
            }
            ObjectPatProp::Rest(rest) => {
              self.bind_pat(rest.arg.as_ref());
            }
          }
        }
      }
      // ([a, b]) => {}
      Pat::Array(a) => {
        for element in a.elems.iter() {
          if element.is_some() {
            self.bind_pat(element.as_ref().unwrap());
          }
        }
      }
      // (a = value) => {}
      Pat::Assign(assign) => {
        // bind the variable onto the scope
        self.bind_assign(assign);
      }

      Pat::Rest(rest) => {
        self.bind_pat(rest.arg.as_ref());
      }

      _ => {}
    }
  }

  pub fn bind_assign(&mut self, assign: &AssignPat) {
    // bind the variable onto the scope
    self.bind_pat(assign.left.as_ref());
  }
}
