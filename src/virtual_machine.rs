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
    self.bind_stmts(items);
  }

  pub fn bind_block(&mut self, block: &BlockStmt) {
    self.bind_stmts(&block.stmts)
  }

  pub fn bind_stmts<T>(&mut self, stmts: &[T])
  where
    T: StmtLike,
  {
    stmts.iter().for_each(|stmt| match stmt.as_stmt() {
      Some(stmt) => match stmt {
        Stmt::Decl(Decl::Class(class_decl)) => self.bind_class_decl(class_decl),
        Stmt::Decl(Decl::Var(var)) => self.bind_var_decl(var),
        Stmt::Decl(Decl::Fn(func)) => self.bind_ident(&func.ident),
        _ => {}
      },
      _ => {}
    });
  }

  pub fn bind_class_decl(&mut self, class_decl: &ClassDecl) {
    self.bind_ident(&class_decl.ident);
  }

  pub fn bind_all_constructor_params(&mut self, params: &[ParamOrTsParamProp]) {
    params.iter().for_each(|param| match param {
      ParamOrTsParamProp::Param(p) => self.bind_param(p),
      ParamOrTsParamProp::TsParamProp(p) => match &p.param {
        TsParamPropParam::Assign(assign) => self.bind_pat(assign.left.as_ref()),
        TsParamPropParam::Ident(ident) => self.bind_ident(&ident.id),
      },
    })
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

  pub fn bind_all_params(&mut self, params: &[Param]) {
    params.iter().for_each(|param| self.bind_param(&param));
  }

  pub fn bind_param(&mut self, param: &Param) {
    self.bind_pat(&param.pat);
  }

  pub fn bind_all_pats(&mut self, pats: &[Pat]) {
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
        self.bind_pat(assign.left.as_ref());
      }

      Pat::Rest(rest) => {
        self.bind_pat(rest.arg.as_ref());
      }

      _ => {}
    }
  }
}
