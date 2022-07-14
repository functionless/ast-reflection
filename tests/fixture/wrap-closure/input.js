let i = 0; // let i#1 = 0

const capture_module_scoped_let = () => {
  i;
};

function function_decl_captures_module_scope_let() {
  i;
}

const dont_capture_shadowed_let = () => {
  let i = 0; // let i#2 = 0
  i; // not a free variable since it is shadowed within this scope
};

() => {
  let i = 0; // let i#3 = 0

  const capture_shadowed_let = () => {
    i; // i#3
  };
};

const capture_hoisted_var = () => {
  {
    () => {
      a; // capture hoisted free variable, a
    };
  }

  var a;
};
