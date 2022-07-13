global.__fnl_func(_a, () => ({
  __filename,
  free: [["i", 1, i]],
}));

let i = 0;
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
const b = global.__fnl_func(
  () => {
    let i = 0;
    i; // not a free variable since it is shadowed within this scope
  },
  () => ({
    __filename,
    free: [],
  })
);
