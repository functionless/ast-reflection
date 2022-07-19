const { a , b: c , d: [e] , ...f } = {};
const [g, { h , i: j  }, [k], ...l] = [];
const m = [];
const block_scoped_arrow_expr = ()=>{};
const block_scoped_func_expr = function() {};
let i = 0;
class StaticBlocks {
    static{
        global.__fnl_func(// capture all module-scoped variables
        ()=>{
            i, a, c, e, f, g, h, j, k, l, m, block_scoped_arrow_expr, block_scoped_func_expr, hoisted_var, hoisted_func;
        }, __filename, ()=>[
                a,
                block_scoped_arrow_expr,
                block_scoped_func_expr,
                c,
                e,
                f,
                g,
                h,
                hoisted_func,
                hoisted_var,
                i,
                j,
                k,
                l,
                m
            ]);
    }
    static{
        global.__fnl_func(// capture module-scoped i
        ()=>{
            i;
        }, __filename, ()=>[
                i
            ]);
        (i)=>{
            i;
        };
    }
    static{
        let i1 = 0;
        global.__fnl_func(// capture static-block-scoped i that shadows module scoped i
        ()=>i1, __filename, ()=>[
                i1
            ]);
        (i)=>i;
    }
    static{
        let i2 = 0;
        {
            global.__fnl_func(// capture static-block-scoped i  that shadows module scoped i
            ()=>i2, __filename, ()=>[
                    i2
                ]);
            (i)=>i;
        }
    }
    static{
        {
            let i3 = 0;
            global.__fnl_func(// capture block-scoped i
            ()=>i3, __filename, ()=>[
                    i3
                ]);
            (i)=>i;
        }
    }
    static{}
}
class StaticFunctionDecls {
    static a() {
        i;
    }
    static b(i) {
        i;
    }
    static c({ i  }) {
        i;
    }
    static d({ j: i  }) {
        i;
    }
    static e([i]) {
        i;
    }
    static f(...i) {
        i;
    }
    static{
        global.__fnl_func(this.prototype.a, __filename, ()=>[
                i
            ]);
    }
}
class StaticFuncExpr {
    static a = global.__fnl_func(function() {
        i;
    }, __filename, ()=>[
            i
        ]);
    static b = function(i) {
        i;
    };
    static c = function({ i  }) {
        i;
    };
    static d = function({ j: i  }) {
        i;
    };
    static e = function([i]) {
        i;
    };
    static f = function(...i) {
        i;
    };
    static{
        global.__fnl_func(this, __filename, ()=>[
                i
            ]);
    }
}
class StaticArrowExpr {
    static a = global.__fnl_func(()=>{
        i;
    }, __filename, ()=>[
            i
        ]);
    static b = (i)=>{
        i;
    };
    static c = ({ i  })=>{
        i;
    };
    static d = ({ j: i  })=>{
        i;
    };
    static e = ([i])=>{
        i;
    };
    static f = (...i)=>{
        i;
    };
    static{
        global.__fnl_func(this, __filename, ()=>[
                i
            ]);
    }
}
var hoisted_var;
function hoisted_func() {}
