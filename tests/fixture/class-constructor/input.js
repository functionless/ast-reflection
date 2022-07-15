const { a, b: c, d: [e], ...f } = {};
const [g, { h, i: j }, [k], ...l] = [];
const m = [];

const block_scoped_arrow_expr = () => { };

const block_scoped_func_expr = function () { };

{
  // should capture free variables in class body
  class A {
    a = a;
    c = c;
    e = e;
    f = f;
    g = g;
    h = h;
    j = j;
    k = k;
    l = l;
    m = m;
  }
}

{
  class A {
    // capture all free variables from within the constructor
    constructor() {
      a, c, e, f, g, h, j, k, l, m, block_scoped_arrow_expr, block_scoped_func_expr, hoisted_var, hoisted_func
    }
  }
}

{
  // capture free variables in both class body and constructor
  class A {
    a = a;
    c = c;
    e = e;
    f = f;
    g = g;
    h = h;
    j = j;
    k = k;
    l = l;
    m = m;

    constructor() {
      a, c, e, f, g, h, j, k, l, m;
    }
  }
}

{
  // capture free variables in nested syntax
  class A {
    constructor() {
      a[c], e?.[f], g(j), j.k(), l?.[m]()
    }
  }
}


{
  class A {
    // should not capture shadowed free variables
    constructor({ a, b: c, d: [e], hoisted_var, block_scoped_arrow_expr, ...f }, [g, { h, i: j }, [k], hoisted_func, block_scoped_func_expr, ...l], ...m) {
      a, c, e, f, g, h, j, k, l, block_scoped_arrow_expr, block_scoped_func_expr, hoisted_var, hoisted_func
    }
  }
}


var hoisted_var;

function hoisted_func() { }