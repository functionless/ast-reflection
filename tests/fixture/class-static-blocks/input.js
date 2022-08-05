const { a, b: c, d: [e], ...f } = {};
const [g, { h, i: j }, [k], ...l] = [];
const m = [];

const block_scoped_arrow_expr = () => { };

const block_scoped_func_expr = function () { };

let i = 0;

class StaticBlocks {
  static {
    // capture all module-scoped variables
    () => {
      i, a, c, e, f, g, h, j, k, l, m, block_scoped_arrow_expr, block_scoped_func_expr, hoisted_var, hoisted_func
    }
  }

  static {
    // capture module-scoped i
    () => {
      i
    };

    (i) => {
      i
    };
  }

  static {
    let i = 0;

    // capture static-block-scoped i that shadows module scoped i
    () => i;

    (i) => i;
  }

  static {
    let i = 0;

    {
      // capture static-block-scoped i  that shadows module scoped i
      () => i;
  
      (i) => i;
    }
  }

  static {
    {
      let i = 0;
     
      // capture block-scoped i
      () => i;
  
      (i) => i;
    }
  }
}

class StaticFunctionDecls {
  static a() {
    i;
  }

  static b(i) {
    i;
  }

  static c({ i }) {
    i;
  }

  static d({ j: i }) {
    i;
  }

  static e([i]) {
    i;
  }

  static f(...i) {
    i;
  }
}

class StaticFuncExpr {
  static a = function () {
    i;
  }

  static b = function (i) {
    i;
  }

  static c = function ({ i }) {
    i;
  }

  static d = function ({ j: i }) {
    i;
  }

  static e = function ([i]) {
    i;
  }

  static f = function (...i) {
    i;
  }
}

class StaticArrowExpr {
  static a = () => {
    i;
  }

  static b = (i) => {
    i;
  }

  static c = ({ i }) => {
    i;
  }

  static d = ({ j: i }) => {
    i;
  }

  static e = ([i]) => {
    i;
  }

  static f = (...i) => {
    i;
  }
}

var hoisted_var;

function hoisted_func() { }