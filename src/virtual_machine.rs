use im::HashMap;
use swc_plugin::{ast::*, utils::StmtLike};

/**
 * A mapping of [reference name](JsWord) to the [unique id](u32) of that reference.
 */
pub type Names = HashMap<JsWord, u32>;

#[derive(Clone)]
pub struct VirtualMachine {
  pub stack: Stack,
}

impl VirtualMachine {
  pub fn new() -> VirtualMachine {
    VirtualMachine {
      stack: Stack::from(vec![LexicalScope::new()]),
    }
  }
}

#[derive(Clone)]
pub struct LexicalScope {
  /**
   * Variables visible at this point in the tree.
   */
  block: Names,
  /**
   * A Stack of variables including hoisted functions and variable declarations
   *
   * i.e. declarations captured by looking ahead in the tree.
   *
   * ```ts
   * let i = 0;
   * // local: [i]
   * // hoisted: [i, foo]
   * function foo() {
   *  let j = 0;
   *  // local: [i, foo, j]
   *  // hoisted: [i, foo, j, bar]
   *
   *  function bar() {}
   * }
   * ```
   */
  function: Names,
}

impl LexicalScope {
  fn new() -> LexicalScope {
    LexicalScope {
      block: Names::new(),
      function: Names::new(),
    }
  }
}

pub type Stack = Vec<LexicalScope>;

#[derive(Eq, PartialEq, Clone, Copy)]
pub enum Scope {
  /**
   * Functions can see all of names in their parent scopes.
   */
  Function,
  /**
   * A Block can only see hoisted function declarations and variable declarations.
   */
  Block,
}

impl VirtualMachine {
  pub fn lookup_ident(&self, ident: &Ident, scope: Scope) -> Option<u32> {
    self.lookup_name(&ident.sym, scope)
  }

  pub fn lookup_name(&self, name: &JsWord, scope: Scope) -> Option<u32> {
    self.stack.last().and_then(|lex| {
      (if scope == Scope::Block {
        &lex.block
      } else {
        &lex.function
      })
      .get(name)
      .map(|u| *u)
    })
  }

  pub fn enter_isolation(&mut self) {
    self.stack.push(LexicalScope::new());
  }

  /**
   * Push a Scope onto the Stack.
   */
  pub fn enter(&mut self, scope: Scope) {
    self.stack.push(match self.stack.last() {
      Some(parent) => match scope {
        // entering function scope
        // e.g. function foo() { (function scope) }
        Scope::Function => LexicalScope {
          // all variables from the parent's function scope are inherited
          block: parent.function.clone(),
          function: parent.function.clone(),
        },
        Scope::Block => LexicalScope {
          // cloning immutable hash maps is fast - O(1)
          // data sharing of immutable data structures should also be memory efficient - O(lg n)
          function: parent.function.clone(),
          block: parent.block.clone(),
        },
      },
      None => LexicalScope::new(),
    });
  }

  pub fn exit(&mut self) -> LexicalScope {
    match self.stack.pop() {
      Some(popped) => popped,
      None => panic!("stack underflow"),
    }
  }

  pub fn get_names(&self, scope: Scope) -> &Names {
    match self.stack.last() {
      Some(popped) => match scope {
        Scope::Block => &popped.block,
        Scope::Function => &popped.function,
      },
      None => panic!("stack underflow"),
    }
  }

  pub fn get_names_mut(&mut self, scope: Scope) -> &mut Names {
    match self.stack.last_mut() {
      Some(popped) => match scope {
        Scope::Block => &mut popped.block,
        Scope::Function => &mut popped.function,
      },
      None => panic!("stack underflow"),
    }
  }

  pub fn update_name(&mut self, name: JsWord, id: u32, scope: Scope) {
    let names = self.get_names_mut(scope);
    *names = names.update(name, id);
  }

  /**
   * Binds the name of an [ident](Ident) to the current [lexical scope](LexicalScope).
   */
  pub fn bind_ident(&mut self, ident: &Ident, scope: Scope) {
    let id = self.get_unique_id(&ident);

    if scope == Scope::Block {
      // block identifiers only come into scope when they are evaluated
      self.update_name(ident.sym.clone(), id, Scope::Block);
    }
    // all names are visible in the Function scope
    self.update_name(ident.sym.clone(), id, Scope::Function);
  }

  /**
   * Get (or assign) a [unique id](u32) for an [identifier](Ident).
   *
   * The ID will be used to uniquely identify a variable (regardless of name shadowing/collisions).
   */
  pub fn get_unique_id(&mut self, ident: &Ident) -> u32 {
    ident.to_id().1.to_owned().as_u32()
  }

  pub fn bind_all_params(&mut self, params: &[Param], scope: Scope) {
    params
      .iter()
      .for_each(|param| self.bind_param(&param, scope));
  }

  pub fn bind_param(&mut self, param: &Param, scope: Scope) {
    self.bind_pat(&param.pat, scope);
  }

  pub fn bind_all_pats(&mut self, pats: &[Pat], scope: Scope) {
    pats.iter().for_each(|p| self.bind_pat(p, scope));
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
  pub fn bind_pat(&mut self, pat: &Pat, scope: Scope) {
    match pat {
      // (a, b) => {}
      Pat::Ident(ident) => {
        self.bind_ident(&ident.id, scope);
      }
      // ({a, b: c}) => {}
      Pat::Object(o) => {
        for prop in o.props.iter() {
          match prop {
            ObjectPatProp::Assign(a) => {
              self.bind_ident(&a.key, scope);
            }
            ObjectPatProp::KeyValue(kv) => {
              self.bind_pat(kv.value.as_ref(), scope);
            }
            _ => {}
          }
        }
      }
      // ([a, b]) => {}
      Pat::Array(a) => {
        for element in a.elems.iter() {
          if element.is_some() {
            self.bind_pat(element.as_ref().unwrap(), scope);
          }
        }
      }
      // (a = value) => {}
      Pat::Assign(assign) => {
        // bind the variable onto the scope
        self.bind_pat(assign.left.as_ref(), scope);
      }
      _ => {}
    }
  }

  pub fn bind_var_decl(&mut self, var: &VarDecl, scope: Scope) {
    var
      .decls
      .iter()
      .for_each(|decl| self.bind_var_declarator(var.kind, decl, scope));
  }

  /**
   * Bind names produced by a [VarDeclarator](VarDeclarator)
   */
  pub fn bind_var_declarator(&mut self, kind: VarDeclKind, decl: &VarDeclarator, scope: Scope) {
    match decl.init.as_deref() {
      None if kind == VarDeclKind::Var => {
        // var x;
        self.bind_pat(&decl.name, scope);
      }
      Some(_) => {
        // var x = v;
        // let x = b;
        // const x = v;

        // bind the names to the current lexical scope
        self.bind_pat(&decl.name, scope);
      }
      None => {
        // let x;

        // bind the names to the current lexical scope
        self.bind_pat(&decl.name, scope);
      }
    }
  }

  /**
   * Generic function that will walk all statements in a block and hoist
   * all function declarations and any var declarations that can be hoisted.
   *
   * Stores the names produced by a [stmt](Stmt):
   * 1. function declarations
   * ```ts
   * function foo() {}
   * ```
   * 2. var declarations that have no initializer
   * ```ts
   * var foo;
   * ```
   */
  pub fn bind_hoisted_stmts<T>(&mut self, stmts: &[T], scope: Scope)
  where
    T: StmtLike,
  {
    stmts.iter().for_each(|stmt| {
      // hoist all of the function and var declarations in the module into scope
      match stmt.as_stmt() {
        Some(stmt) => {
          match stmt {
            Stmt::Decl(Decl::Var(var)) if var.kind == VarDeclKind::Var => {
              var.decls.iter().for_each(|decl| {
                if decl.init.is_none() {
                  self.bind_var_declarator(VarDeclKind::Var, decl, scope);
                }
              });
            }
            Stmt::Decl(Decl::Fn(func)) => {
              // function declarations should be hoisted to the Block scope
              self.bind_ident(&func.ident, scope);
            }
            _ => {}
          }
          // self.scope.hoist_stmt(&stmt);
        }
        _ => {}
      }
    });
  }
}
