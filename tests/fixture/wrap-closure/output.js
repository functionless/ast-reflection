global.__fnl_func(_a, () => ({
  __filename,
  free: [["i", 1, i]],
}));
let i = 0; // let i#1 = 0
const a = global.__fnl_func(
  () => {
    i;
  },
  () => ({
    __filename,
    free: [["i", 1, i]],
  })
);
function _a() {
  i;
}
const b = () => {
  let i = 0; // let i#2 = 0
  i; // not a free variable since it is shadowed within this scope
};
const c = () => {
  let i = 0; // let i#3 = 0
  const d = global.__fnl_func(
    () => {
      i;
    },
    () => ({
      __filename,
      free: [["i", 3, i]],
    })
  );
};
const d = () => {
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
