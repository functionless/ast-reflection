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
        j,
        k,
        l
    ]);
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
}, __filename, ()=>[
        a,
        b
    ]);
// arrow expr capturing a variable with a spread expression
global.__fnl_func(()=>{
    ({
        ...c
    });
    [
        ...d
    ];
}, __filename, ()=>[
        c,
        d
    ]);
// arrow expr capturing variables in property chains
global.__fnl_func(()=>{
    a.prop;
    b.prop[c];
    d.prop?.[e];
}, __filename, ()=>[
        a,
        b,
        c,
        d,
        e
    ]);
// arrow expr capturing variables as call arguments
global.__fnl_func(()=>{
    a("");
    b(c);
    c(d.prop);
    e(f[g]);
}, __filename, ()=>[
        a,
        b,
        c,
        d,
        e,
        f,
        g
    ]);
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
    }, __filename, ()=>[
            i
        ]);
};
// capture a hoisted var
()=>{
    {
        global.__fnl_func(()=>{
            a; // capture hoisted free variable, a
        }, __filename, ()=>[
                a
            ]);
    }
    var a;
};
// arrow expression
{
    const capture_default_reference_to_param_in_closure = global.__fnl_func((a = global.__fnl_func(()=>{
        i; // i#1
    }, __filename, ()=>[
            i
        ]))=>{}, __filename, ()=>[
            i
        ]);
    const dont_capture_let_shadowed_by_param = (i)=>{
        i;
    };
    const dont_capture_default_reference_to_param = (i, a = i)=>{};
    const dont_capture_default_reference_to_param_in_closure = (i, a = global.__fnl_func(()=>{
        i;
    }, __filename, ()=>[
            i
        ]))=>{};
    const arg_destructuring_test = (i, { a , b: c , d: [e] , ...f }, [g, { h , i: j  }, [k], ...l], ...m)=>{
        return global.__fnl_func(()=>{
            i, a, c, e, f, g, h, j, k, l, m;
        }, __filename, ()=>[
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
            ]);
    };
}// function expressions
{
    const capture_default_reference_to_param_in_closure1 = global.__fnl_func(function(a = global.__fnl_func(function() {
        i; // i#1
    }, __filename, ()=>[
            i
        ])) {}, __filename, ()=>[
            i
        ]);
    const dont_capture_let_shadowed_by_param1 = function(i) {
        i;
    };
    const dont_capture_default_reference_to_param1 = function(i, a = i) {};
    const dont_capture_default_reference_to_param_in_closure1 = function(i, a = global.__fnl_func(function() {
        i;
    }, __filename, ()=>[
            i
        ])) {};
    const arg_destructuring_test1 = function(i, { a , b: c , d: [e] , ...f }, [g, { h , i: j  }, [k], ...l], ...m) {
        return global.__fnl_func(function() {
            i, a, c, e, f, g, h, j, k, l, m;
        }, __filename, ()=>[
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
            ]);
    };
}// function declarations
{
    global.__fnl_func(capture_default_reference_to_param_in_closure2, __filename, ()=>[
            i
        ]);
    function capture_default_reference_to_param_in_closure2(a = global.__fnl_func(()=>{
        i; // i#1
    }, __filename, ()=>[
            i
        ])) {}
    ;
    function dont_capture_let_shadowed_by_param2(i) {
        i;
    }
    ;
    function dont_capture_default_reference_to_param2(i, a = i) {}
    ;
    function dont_capture_default_reference_to_param_in_closure2(i, a = global.__fnl_func(()=>{
        i;
    }, __filename, ()=>[
            i
        ])) {}
    ;
    function arg_destructuring_test2(i, { a , b: c , d: [e] , ...f }, [g, { h , i: j  }, [k], ...l], ...m) {
        return global.__fnl_func(()=>{
            i, a, c, e, f, g, h, j, k, l, m;
        }, __filename, ()=>[
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
            ]);
    }
    ;
}
