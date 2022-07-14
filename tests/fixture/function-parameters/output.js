let i;
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
                    6,
                    i
                ]
            ]
        })))=>{};
    const arg_destructuring_test = (i, { a , b: c , d: [e] , ...f }, [g, { h , i: j  }, [k], ...l], ...m)=>{
        return global.__fnl_func(()=>{
            i;
            a;
            // b;
            c;
            // d;
            e;
            f;
            g;
            h;
            // i;
            j;
            k;
            l;
            m;
        }, ()=>({
                __filename,
                free: [
                    [
                        "a",
                        7,
                        a
                    ],
                    [
                        "c",
                        7,
                        c
                    ],
                    [
                        "e",
                        7,
                        e
                    ],
                    [
                        "f",
                        7,
                        f
                    ],
                    [
                        "g",
                        7,
                        g
                    ],
                    [
                        "h",
                        7,
                        h
                    ],
                    [
                        "i",
                        7,
                        i
                    ],
                    [
                        "j",
                        7,
                        j
                    ],
                    [
                        "k",
                        7,
                        k
                    ],
                    [
                        "l",
                        7,
                        l
                    ],
                    [
                        "m",
                        7,
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
            i;
            a;
            // b;
            c;
            // d;
            e;
            f;
            g;
            h;
            // i;
            j;
            k;
            l;
            m;
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
                    18,
                    i
                ]
            ]
        }))) {}
    ;
    function arg_destructuring_test2(i, { a , b: c , d: [e] , ...f }, [g, { h , i: j  }, [k], ...l], ...m) {
        return global.__fnl_func(()=>{
            i;
            a;
            // b;
            c;
            // d;
            e;
            f;
            g;
            h;
            // i;
            j;
            k;
            l;
            m;
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
    }
    ;
}
