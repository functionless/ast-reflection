const { a, b: c, d: [e], ...f } = {};
const [g, { h, i: j }, [k], ...l] = [];
const m = [];

const block_scoped_arrow_expr = () => { };

const block_scoped_func_expr = function () { };

// arrow expr capturing nothing
() => {

};

// arrow expr capturing all variables
() => {
  a, c, e, f, g, h, j, k, l, block_scoped_arrow_expr, block_scoped_func_expr, hoisted_var, hoisted_func;
};

// arrow expr where parameters shadow all variables
(a, c, e, f, g, h, j, k, l, block_scoped_arrow_expr, block_scoped_func_expr, hoisted_var, hoisted_func) => {
  a, c, e, f, g, h, j, k, l, block_scoped_arrow_expr, block_scoped_func_expr, hoisted_var, hoisted_func;
}

// arrow expr where destructuring syntax inside parameters shadow all variables
({ a, b: c, d: [e], hoisted_var, block_scoped_arrow_expr, ...f }, [g, { h, i: j }, [k], hoisted_func, block_scoped_func_expr, ...l], ...m) => {
  a, c, e, f, g, h, j, k, l, m, block_scoped_arrow_expr, block_scoped_func_expr, hoisted_var, hoisted_func;
};

// arrow expr capturing variables with an element access expression
() => {
  ({})[a];
  [][b];
};

// arrow expr capturing a variable with a spread expression
() => {
  ({ ...c });
  [...d];
};

// arrow expr capturing variables in property chains
() => {
  a.prop;
  b.prop[c];
  d.prop?.[e];
};

// arrow expr capturing variables as call arguments
() => {
  a("");
  b(c);
  c(d.prop);
  e(f[g]);
};

var hoisted_var;

function hoisted_func() { }

let i;

// shadowed let should not be captured
() => {
  let i = 0;
  i; // not a free variable since it is shadowed within this scope
};

() => {
  let i = 0; // let i#3 = 0

  // capture the shadowed i
  () => {
    i; // i#3
  };
};

// capture a hoisted var
() => {
  {
    () => {
      a; // capture hoisted free variable, a
    };
  }

  var a;
};

// arrow expression
{
  const capture_default_reference_to_param_in_closure = (
    a = () => {
      i; // i#1
    }
  ) => {};
  
  const dont_capture_let_shadowed_by_param = (i) => {
    i;
  };
  
  const dont_capture_default_reference_to_param = (i, a = i) => {};
  
  const dont_capture_default_reference_to_param_in_closure = (
    i,
    a = () => {
      i;
    }
  ) => {};
  
  const arg_destructuring_test = (
    i,
    { a, b: c, d: [e], ...f },
    [g, { h, i: j }, [k], ...l],
    ...m
  ) => {
    return () => {
      i, a, c, e, f, g, h, j, k, l, m;
    };
  };
}

// function expressions
{
  const capture_default_reference_to_param_in_closure = function (
    a = function () {
      i; // i#1
    }
  ) {};
  
  const dont_capture_let_shadowed_by_param = function (i) {
    i;
  };
  
  const dont_capture_default_reference_to_param = function (i, a = i) {};
  
  const dont_capture_default_reference_to_param_in_closure = function (
    i,
    a = function () {
      i;
    }
  ) {};
  
  const arg_destructuring_test = function (
    i,
    { a, b: c, d: [e], ...f },
    [g, { h, i: j }, [k], ...l],
    ...m
  ) {
    return function () {
      i, a, c, e, f, g, h, j, k, l, m;
    };
  };
}

// function declarations
{
  function capture_default_reference_to_param_in_closure (
    a = () => {
      i; // i#1
    }
  ) {};
  
  function dont_capture_let_shadowed_by_param (i) {
    i;
  };
  
  function dont_capture_default_reference_to_param (i, a = i) {};
  
  function dont_capture_default_reference_to_param_in_closure (
    i,
    a = () => {
      i;
    }
  ) {};
  
  function arg_destructuring_test (
    i,
    { a, b: c, d: [e], ...f },
    [g, { h, i: j }, [k], ...l],
    ...m
  ) {
    return () => {
      i, a, c, e, f, g, h, j, k, l, m;
    };
  };
}