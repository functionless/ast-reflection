let i = 0;
const a = () => {
  i;
};
function _a() {
  i;
}
const b = () => {
  let i = 0;
  i; // not a free variable since it is shadowed within this scope
};
