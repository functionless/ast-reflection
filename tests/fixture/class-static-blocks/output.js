const { a , b: c , d: [e] , ...f } = {};
const [g, { h , i: j  }, [k], ...l] = [];
const m = [];
const block_scoped_arrow_expr = ()=>{};
const block_scoped_func_expr = function() {};
let i = 0;
class StaticBlocks {
    static{
        (()=>{
            const c1 = // capture all module-scoped variables
            ()=>{
                i, a, c, e, f, g, h, j, k, l, m, block_scoped_arrow_expr, block_scoped_func_expr, hoisted_var, hoisted_func;
            };
            c1["[[Closure]]"] = [
                __filename,
                ()=>[
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
                    ]
            ];
            return c1;
        })();
    }
    static{
        (()=>{
            const c = // capture module-scoped i
            ()=>{
                i;
            };
            c["[[Closure]]"] = [
                __filename,
                ()=>[
                        i
                    ]
            ];
            return c;
        })();
        (i)=>{
            i;
        };
    }
    static{
        let i1 = 0;
        (()=>{
            const c = // capture static-block-scoped i that shadows module scoped i
            ()=>i1;
            c["[[Closure]]"] = [
                __filename,
                ()=>[
                        i1
                    ]
            ];
            return c;
        })();
        (i)=>i;
    }
    static{
        let i2 = 0;
        {
            (()=>{
                const c = // capture static-block-scoped i  that shadows module scoped i
                ()=>i2;
                c["[[Closure]]"] = [
                    __filename,
                    ()=>[
                            i2
                        ]
                ];
                return c;
            })();
            (i)=>i;
        }
    }
    static{
        {
            let i3 = 0;
            (()=>{
                const c = // capture block-scoped i
                ()=>i3;
                c["[[Closure]]"] = [
                    __filename,
                    ()=>[
                            i3
                        ]
                ];
                return c;
            })();
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
        this.prototype.a["[[Closure]]"] = [
            __filename,
            ()=>[
                    i
                ]
        ];
    }
}
class StaticFuncExpr {
    static a = (()=>{
        const c = function() {
            i;
        };
        c["[[Closure]]"] = [
            __filename,
            ()=>[
                    i
                ]
        ];
        return c;
    })();
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
        this["[[Closure]]"] = [
            __filename,
            ()=>[
                    i
                ]
        ];
    }
}
class StaticArrowExpr {
    static a = (()=>{
        const c = ()=>{
            i;
        };
        c["[[Closure]]"] = [
            __filename,
            ()=>[
                    i
                ]
        ];
        return c;
    })();
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
        this["[[Closure]]"] = [
            __filename,
            ()=>[
                    i
                ]
        ];
    }
}
var hoisted_var;
function hoisted_func() {}
