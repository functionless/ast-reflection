let i = 0; // let i#1 = 0
const a = () => {
  i;
};
function _a() {
  i;
}
const b = () => {
  let i = 0; // let i#2 = 0
  i; // not a free variable since it is shadowed within this scope
};

const c = () => {
  let i = 0; // let i#3 = 0

  const d = () => {
    i;
  };
};

const d = () => {
  {
    () => {
      a; // capture hoisted free variable, a
    };
  }

  var a;
};
