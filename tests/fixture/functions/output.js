const { a , b: c , d: [e] , ...f } = {};
const [g, { h , i: j  }, [k], ...l] = [];
const m = [];
const block_scoped_arrow_expr = ()=>{};
const block_scoped_func_expr = function() {};
// arrow expr capturing nothing
()=>{};
// arrow expr capturing all variables
global.__fnl_func(()=>{
    a, c, e, f, g, h, j, k, l, block_scoped_arrow_expr, block_scoped_func_expr, hoisted_var, hoisted_func;
}, ()=>({
        __filename,
        free: [
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
            ]
        ]
    }));
// arrow expr where parameters shadow all variables
(a, c, e, f, g, h, j, k, l, block_scoped_arrow_expr, block_scoped_func_expr, hoisted_var, hoisted_func)=>{
    a, c, e, f, g, h, j, k, l, block_scoped_arrow_expr, block_scoped_func_expr, hoisted_var, hoisted_func;
};
// arrow expr where destructuring syntax inside parameters shadow all variables
({ a , b: c , d: [e] , hoisted_var , block_scoped_arrow_expr , ...f }, [g, { h , i: j  }, [k], hoisted_func, block_scoped_func_expr, ...l], ...m)=>{
    a, c, e, f, g, h, j, k, l, m, block_scoped_arrow_expr, block_scoped_func_expr, hoisted_var, hoisted_func;
};
// arrow expr capturing variables with an element access expression
global.__fnl_func(()=>{
    ({})[a];
    [][b];
}, ()=>({
        __filename,
        free: [
            [
                "a",
                1,
                a
            ],
            [
                "b",
                4,
                b
            ]
        ]
    }));
// arrow expr capturing a variable with a spread expression
global.__fnl_func(()=>{
    ({
        ...c
    });
    [
        ...d
    ];
}, ()=>({
        __filename,
        free: [
            [
                "c",
                1,
                c
            ],
            [
                "d",
                4,
                d
            ]
        ]
    }));
// arrow expr capturing variables in property chains
global.__fnl_func(()=>{
    a.prop;
    b.prop[c];
    d.prop?.[e];
}, ()=>({
        __filename,
        free: [
            [
                "a",
                1,
                a
            ],
            [
                "b",
                4,
                b
            ],
            [
                "c",
                1,
                c
            ],
            [
                "d",
                4,
                d
            ],
            [
                "e",
                1,
                e
            ]
        ]
    }));
// arrow expr capturing variables as call arguments
global.__fnl_func(()=>{
    a("");
    b(c);
    c(d.prop);
    e(f[g]);
}, ()=>({
        __filename,
        free: [
            [
                "a",
                1,
                a
            ],
            [
                "b",
                4,
                b
            ],
            [
                "c",
                1,
                c
            ],
            [
                "d",
                4,
                d
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
            ]
        ]
    }));
var hoisted_var;
function hoisted_func() {}
let i;
// shadowed let should not be captured
()=>{
    let i = 0;
    i; // not a free variable since it is shadowed within this scope
};
()=>{
    let i = 0; // let i#3 = 0
    global.__fnl_func(// capture the shadowed i
    ()=>{
        i; // i#3
    }, ()=>({
            __filename,
            free: [
                [
                    "i",
                    6,
                    i
                ]
            ]
        }));
};
// capture a hoisted var
()=>{
    {
        global.__fnl_func(()=>{
            a; // capture hoisted free variable, a
        }, ()=>({
                __filename,
                free: [
                    [
                        "a",
                        7,
                        a
                    ]
                ]
            }));
    }
    var a;
};
// arrow expression
{
    const capture_default_reference_to_param_in_closure = (a = global.__fnl_func(()=>{
        i; // i#1
    }, ()=>({
            __filename,
            free: [
                [
                    "i",
                    1,
                    i
                ]
            ]
        })))=>{};
    const dont_capture_let_shadowed_by_param = (i)=>{
        i;
    };
    const dont_capture_default_reference_to_param = (i, a = i)=>{};
    const dont_capture_default_reference_to_param_in_closure = (i, a = global.__fnl_func(()=>{
        i;
    }, ()=>({
            __filename,
            free: [
                [
                    "i",
                    12,
                    i
                ]
            ]
        })))=>{};
    const arg_destructuring_test = (i, { a , b: c , d: [e] , ...f }, [g, { h , i: j  }, [k], ...l], ...m)=>{
        return global.__fnl_func(()=>{
            i, a, c, e, f, g, h, j, k, l, m;
        }, ()=>({
                __filename,
                free: [
                    [
                        "a",
                        13,
                        a
                    ],
                    [
                        "c",
                        13,
                        c
                    ],
                    [
                        "e",
                        13,
                        e
                    ],
                    [
                        "f",
                        13,
                        f
                    ],
                    [
                        "g",
                        13,
                        g
                    ],
                    [
                        "h",
                        13,
                        h
                    ],
                    [
                        "i",
                        13,
                        i
                    ],
                    [
                        "j",
                        13,
                        j
                    ],
                    [
                        "k",
                        13,
                        k
                    ],
                    [
                        "l",
                        13,
                        l
                    ],
                    [
                        "m",
                        13,
                        m
                    ]
                ]
            }));
    };
}// function expressions
{
    const capture_default_reference_to_param_in_closure1 = global.__fnl_func(function(a = function() {
        i; // i#1
    }) {}, ()=>({
            __filename,
            free: [
                [
                    "i",
                    1,
                    i
                ]
            ]
        }));
    const dont_capture_let_shadowed_by_param1 = function(i) {
        i;
    };
    const dont_capture_default_reference_to_param1 = function(i, a = i) {};
    const dont_capture_default_reference_to_param_in_closure1 = function(i, a = function() {
        i;
    }) {};
    const arg_destructuring_test1 = global.__fnl_func(function(i, { a , b: c , d: [e] , ...f }, [g, { h , i: j  }, [k], ...l], ...m) {
        return global.__fnl_func(function() {
            i, a, c, e, f, g, h, j, k, l, m;
        }, ()=>({
                __filename,
                free: [
                    [
                        "a",
                        19,
                        a
                    ],
                    [
                        "c",
                        19,
                        c
                    ],
                    [
                        "e",
                        19,
                        e
                    ],
                    [
                        "f",
                        19,
                        f
                    ],
                    [
                        "g",
                        19,
                        g
                    ],
                    [
                        "h",
                        19,
                        h
                    ],
                    [
                        "i",
                        19,
                        i
                    ],
                    [
                        "j",
                        19,
                        j
                    ],
                    [
                        "k",
                        19,
                        k
                    ],
                    [
                        "l",
                        19,
                        l
                    ],
                    [
                        "m",
                        19,
                        m
                    ]
                ]
            }));
    }, ()=>({
            __filename,
            free: [
                [
                    "b",
                    0,
                    b
                ],
                [
                    "d",
                    0,
                    d
                ]
            ]
        }));
}// function declarations
{
    global.__fnl_func(capture_default_reference_to_param_in_closure2, ()=>({
            __filename,
            free: [
                [
                    "i",
                    1,
                    i
                ]
            ]
        }));
    global.__fnl_func(arg_destructuring_test2, ()=>({
            __filename,
            free: [
                [
                    "b",
                    0,
                    b
                ],
                [
                    "d",
                    0,
                    d
                ]
            ]
        }));
    function capture_default_reference_to_param_in_closure2(a = global.__fnl_func(()=>{
        i; // i#1
    }, ()=>({
            __filename,
            free: [
                [
                    "i",
                    1,
                    i
                ]
            ]
        }))) {}
    ;
    function dont_capture_let_shadowed_by_param2(i) {
        i;
    }
    ;
    function dont_capture_default_reference_to_param2(i, a = i) {}
    ;
    function dont_capture_default_reference_to_param_in_closure2(i, a = global.__fnl_func(()=>{
        i;
    }, ()=>({
            __filename,
            free: [
                [
                    "i",
                    24,
                    i
                ]
            ]
        }))) {}
    ;
    function arg_destructuring_test2(i, { a , b: c , d: [e] , ...f }, [g, { h , i: j  }, [k], ...l], ...m) {
        return global.__fnl_func(()=>{
            i, a, c, e, f, g, h, j, k, l, m;
        }, ()=>({
                __filename,
                free: [
                    [
                        "a",
                        25,
                        a
                    ],
                    [
                        "c",
                        25,
                        c
                    ],
                    [
                        "e",
                        25,
                        e
                    ],
                    [
                        "f",
                        25,
                        f
                    ],
                    [
                        "g",
                        25,
                        g
                    ],
                    [
                        "h",
                        25,
                        h
                    ],
                    [
                        "i",
                        25,
                        i
                    ],
                    [
                        "j",
                        25,
                        j
                    ],
                    [
                        "k",
                        25,
                        k
                    ],
                    [
                        "l",
                        25,
                        l
                    ],
                    [
                        "m",
                        25,
                        m
                    ]
                ]
            }));
    }
    ;
}
