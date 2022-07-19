const { a , b: c , d: [e] , ...f } = {};
const [g, { h , i: j  }, [k], ...l] = [];
const m = [];
const block_scoped_arrow_expr = ()=>{};
const block_scoped_func_expr = function() {};
// arrow expr capturing nothing
()=>{};
// arrow expr capturing all variables
(()=>{
    const c1 = ()=>{
        a, c, e, f, g, h, j, k, l, block_scoped_arrow_expr, block_scoped_func_expr, hoisted_var, hoisted_func;
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
                j,
                k,
                l
            ]
    ];
    return c1;
})();
// arrow expr where parameters shadow all variables
(a, c, e, f, g, h, j, k, l, block_scoped_arrow_expr, block_scoped_func_expr, hoisted_var, hoisted_func)=>{
    a, c, e, f, g, h, j, k, l, block_scoped_arrow_expr, block_scoped_func_expr, hoisted_var, hoisted_func;
};
// arrow expr where destructuring syntax inside parameters shadow all variables
({ a , b: c , d: [e] , hoisted_var , block_scoped_arrow_expr , ...f }, [g, { h , i: j  }, [k], hoisted_func, block_scoped_func_expr, ...l], ...m)=>{
    a, c, e, f, g, h, j, k, l, m, block_scoped_arrow_expr, block_scoped_func_expr, hoisted_var, hoisted_func;
};
// arrow expr capturing variables with an element access expression
(()=>{
    const c = ()=>{
        ({})[a];
        [][b];
    };
    c["[[Closure]]"] = [
        __filename,
        ()=>[
                a,
                b
            ]
    ];
    return c;
})();
// arrow expr capturing a variable with a spread expression
(()=>{
    const c1 = ()=>{
        ({
            ...c
        });
        [
            ...d
        ];
    };
    c1["[[Closure]]"] = [
        __filename,
        ()=>[
                c,
                d
            ]
    ];
    return c1;
})();
// arrow expr capturing variables in property chains
(()=>{
    const c1 = ()=>{
        a.prop;
        b.prop[c];
        d.prop?.[e];
    };
    c1["[[Closure]]"] = [
        __filename,
        ()=>[
                a,
                b,
                c,
                d,
                e
            ]
    ];
    return c1;
})();
// arrow expr capturing variables as call arguments
(()=>{
    const c1 = ()=>{
        a("");
        b(c);
        c(d.prop);
        e(f[g]);
    };
    c1["[[Closure]]"] = [
        __filename,
        ()=>[
                a,
                b,
                c,
                d,
                e,
                f,
                g
            ]
    ];
    return c1;
})();
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
    (()=>{
        const c = // capture the shadowed i
        ()=>{
            i; // i#3
        };
        c["[[Closure]]"] = [
            __filename,
            ()=>[
                    i
                ]
        ];
        return c;
    })();
};
// capture a hoisted var
()=>{
    {
        (()=>{
            const c = ()=>{
                a; // capture hoisted free variable, a
            };
            c["[[Closure]]"] = [
                __filename,
                ()=>[
                        a
                    ]
            ];
            return c;
        })();
    }
    var a;
};
// arrow expression
{
    const capture_default_reference_to_param_in_closure = (()=>{
        const c = (a = (()=>{
            const c = ()=>{
                i; // i#1
            };
            c["[[Closure]]"] = [
                __filename,
                ()=>[
                        i
                    ]
            ];
            return c;
        })())=>{};
        c["[[Closure]]"] = [
            __filename,
            ()=>[
                    i
                ]
        ];
        return c;
    })();
    const dont_capture_let_shadowed_by_param = (i)=>{
        i;
    };
    const dont_capture_default_reference_to_param = (i, a = i)=>{};
    const dont_capture_default_reference_to_param_in_closure = (i, a = (()=>{
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
    })())=>{};
    const arg_destructuring_test = (i, { a , b: c , d: [e] , ...f }, [g, { h , i: j  }, [k], ...l], ...m)=>{
        return (()=>{
            const c1 = ()=>{
                i, a, c, e, f, g, h, j, k, l, m;
            };
            c1["[[Closure]]"] = [
                __filename,
                ()=>[
                        a,
                        c,
                        e,
                        f,
                        g,
                        h,
                        i,
                        j,
                        k,
                        l,
                        m
                    ]
            ];
            return c1;
        })();
    };
}// function expressions
{
    const capture_default_reference_to_param_in_closure1 = (()=>{
        const c = function(a = (()=>{
            const c = function() {
                i; // i#1
            };
            c["[[Closure]]"] = [
                __filename,
                ()=>[
                        i
                    ]
            ];
            return c;
        })()) {};
        c["[[Closure]]"] = [
            __filename,
            ()=>[
                    i
                ]
        ];
        return c;
    })();
    const dont_capture_let_shadowed_by_param1 = function(i) {
        i;
    };
    const dont_capture_default_reference_to_param1 = function(i, a = i) {};
    const dont_capture_default_reference_to_param_in_closure1 = function(i, a = (()=>{
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
    })()) {};
    const arg_destructuring_test1 = function(i, { a , b: c , d: [e] , ...f }, [g, { h , i: j  }, [k], ...l], ...m) {
        return (()=>{
            const c1 = function() {
                i, a, c, e, f, g, h, j, k, l, m;
            };
            c1["[[Closure]]"] = [
                __filename,
                ()=>[
                        a,
                        c,
                        e,
                        f,
                        g,
                        h,
                        i,
                        j,
                        k,
                        l,
                        m
                    ]
            ];
            return c1;
        })();
    };
}// function declarations
{
    capture_default_reference_to_param_in_closure2["[[Closure]]"] = [
        __filename,
        ()=>[
                i
            ]
    ];
    function capture_default_reference_to_param_in_closure2(a = (()=>{
        const c = ()=>{
            i; // i#1
        };
        c["[[Closure]]"] = [
            __filename,
            ()=>[
                    i
                ]
        ];
        return c;
    })()) {}
    ;
    function dont_capture_let_shadowed_by_param2(i) {
        i;
    }
    ;
    function dont_capture_default_reference_to_param2(i, a = i) {}
    ;
    function dont_capture_default_reference_to_param_in_closure2(i, a = (()=>{
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
    })()) {}
    ;
    function arg_destructuring_test2(i, { a , b: c , d: [e] , ...f }, [g, { h , i: j  }, [k], ...l], ...m) {
        return (()=>{
            const c1 = ()=>{
                i, a, c, e, f, g, h, j, k, l, m;
            };
            c1["[[Closure]]"] = [
                __filename,
                ()=>[
                        a,
                        c,
                        e,
                        f,
                        g,
                        h,
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
    ;
}
