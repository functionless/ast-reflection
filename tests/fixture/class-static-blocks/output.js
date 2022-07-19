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
                [
                    "a",
                    1,
                    a
                ],
                [
                    "block_scoped_arrow_expr",
                    1,
                    block_scoped_arrow_expr
                ],
                [
                    "block_scoped_func_expr",
                    1,
                    block_scoped_func_expr
                ],
                [
                    "c",
                    1,
                    c
                ],
                [
                    "e",
                    1,
                    e
                ],
                [
                    "f",
                    1,
                    f
                ],
                [
                    "g",
                    1,
                    g
                ],
                [
                    "h",
                    1,
                    h
                ],
                [
                    "hoisted_func",
                    1,
                    hoisted_func
                ],
                [
                    "hoisted_var",
                    1,
                    hoisted_var
                ],
                [
                    "i",
                    1,
                    i
                ],
                [
                    "j",
                    1,
                    j
                ],
                [
                    "k",
                    1,
                    k
                ],
                [
                    "l",
                    1,
                    l
                ],
                [
                    "m",
                    1,
                    m
                ]
            ]);
    }
    static{
        global.__fnl_func(// capture module-scoped i
        ()=>{
            i;
        }, __filename, ()=>[
                [
                    "i",
                    1,
                    i
                ]
            ]);
        (i)=>{
            i;
        };
    }
    static{
        let i1 = 0;
        global.__fnl_func(// capture static-block-scoped i that shadows module scoped i
        ()=>i1, __filename, ()=>[
                [
                    "i",
                    3,
                    i1
                ]
            ]);
        (i)=>i;
    }
    static{
        let i2 = 0;
        {
            global.__fnl_func(// capture static-block-scoped i  that shadows module scoped i
            ()=>i2, __filename, ()=>[
                    [
                        "i",
                        5,
                        i2
                    ]
                ]);
            (i)=>i;
        }
    }
    static{
        {
            let i3 = 0;
            global.__fnl_func(// capture block-scoped i
            ()=>i3, __filename, ()=>[
                    [
                        "i",
                        7,
                        i3
                    ]
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
                [
                    "i",
                    1,
                    i
                ]
            ]);
    }
}
class StaticFuncExpr {
    static a = global.__fnl_func(function() {
        i;
    }, __filename, ()=>[
            [
                "i",
                1,
                i
            ]
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
                [
                    "i",
                    1,
                    i
                ]
            ]);
    }
}
class StaticArrowExpr {
    static a = global.__fnl_func(()=>{
        i;
    }, __filename, ()=>[
            [
                "i",
                1,
                i
            ]
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
                [
                    "i",
                    1,
                    i
                ]
            ]);
    }
}
var hoisted_var;
function hoisted_func() {}
