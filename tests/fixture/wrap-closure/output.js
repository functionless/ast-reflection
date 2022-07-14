global.__fnl_func(function_decl_captures_module_scope_let, () => ({
  __filename,
  free: [["i", 1, i]],
}));
let i = 0; // let i#1 = 0
const capture_module_scoped_let = global.__fnl_func(
  () => {
    i;
  },
  () => ({
    __filename,
    free: [["i", 1, i]],
  })
);
function function_decl_captures_module_scope_let() {
  i;
}
const dont_capture_shadowed_let = () => {
  let i = 0; // let i#2 = 0
  i; // not a free variable since it is shadowed within this scope
};
() => {
  let i = 0; // let i#3 = 0
  const capture_shadowed_let = global.__fnl_func(
    () => {
      i; // i#3
    },
    () => ({
      __filename,
      free: [["i", 3, i]],
    })
  );
};
const capture_hoisted_var = () => {
  {
    global.__fnl_func(
      () => {
        a; // capture hoisted free variable, a
      },
      () => ({
        __filename,
        free: [["a", 4, a]],
      })
    );
  }
  var a;
};
