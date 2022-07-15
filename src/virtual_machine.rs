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
      stack: Stack::from(vec![Names::new()]),
    }
  }
}

pub type Stack = Vec<Names>;

impl VirtualMachine {
  pub fn lookup_ident(&self, ident: &Ident) -> Option<u32> {
    self.lookup_name(&ident.sym)
  }

  pub fn lookup_name(&self, name: &JsWord) -> Option<u32> {
    self.stack.last().and_then(|lex| lex.get(name).map(|u| *u))
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
    // cloning immutable hash maps is fast - O(1)
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

  pub fn update_name(&mut self, name: JsWord, id: u32) {
    let names = self.stack.last_mut().expect("stack underflow");
    *names = names.update(name, id);
  }

  /**
   * Binds the name of an [ident](Ident) to the current [lexical scope](LexicalScope).
   */
  pub fn bind_ident(&mut self, ident: &Ident) {
    let id = self.get_unique_id(&ident);

    self.update_name(ident.sym.clone(), id);
  }

  /**
   * Get (or assign) a [unique id](u32) for an [identifier](Ident).
   *
   * The ID will be used to uniquely identify a variable (regardless of name shadowing/collisions).
   */
  pub fn get_unique_id(&mut self, ident: &Ident) -> u32 {
    ident.to_id().1.to_owned().as_u32()
  }

  pub fn bind_block(&mut self, block: &BlockStmt) {
    self.bind_block_stmts(&block.stmts)
  }

  pub fn bind_block_stmts<T>(&mut self, stmts: &[T])
  where
    T: StmtLike,
  {
    stmts.iter().for_each(|stmt| match stmt.as_stmt() {
      Some(stmt) => match stmt {
        Stmt::Decl(Decl::Var(var)) => var.decls.iter().for_each(|decl| {
          self.bind_var_declarator(decl);
        }),
        Stmt::Decl(Decl::Fn(func)) => {
          // function decl() {} // function declaration - hoist to block scope
          self.bind_ident(&func.ident);
        }
        _ => {}
      },
      _ => {}
    });
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

  /**
   * Bind names produced by a [VarDeclarator](VarDeclarator)
   */
  pub fn bind_var_declarator(&mut self, decl: &VarDeclarator) {
    self.bind_pat(&decl.name);
  }
}
