const { a , b: c , d: [e] , ...f } = {};
const [g, { h , i: j  }, [k], ...l] = [];
const m = [];
const block_scoped_arrow_expr = ()=>{};
const block_scoped_func_expr = function() {};
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
        static{
            global.__fnl_func(this, __filename, ()=>[
                    [
                        "a",
                        1,
                        a
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
    }
}{
    class A1 {
        // capture all free variables from within the constructor
        constructor(){
            a, c, e, f, g, h, j, k, l, m, block_scoped_arrow_expr, block_scoped_func_expr, hoisted_var, hoisted_func;
        }
        static{
            global.__fnl_func(this, __filename, ()=>[
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
                    ],
                    [
                        "m",
                        1,
                        m
                    ]
                ]);
        }
    }
}{
    // capture free variables in both class body and constructor
    class A2 {
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
        constructor(){
            a, c, e, f, g, h, j, k, l, m;
        }
        static{
            global.__fnl_func(this, __filename, ()=>[
                    [
                        "a",
                        1,
                        a
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
                    ],
                    [
                        "a",
                        1,
                        a
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
    }
}{
    // capture free variables in nested syntax
    class A3 {
        constructor(){
            a[c], e?.[f], g(j), j.k(), l?.[m]();
        }
        static{
            global.__fnl_func(this, __filename, ()=>[
                    [
                        "a",
                        1,
                        a
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
                        "j",
                        1,
                        j
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
    }
}{
    class A4 {
        // should not capture shadowed free variables
        constructor({ a , b: c , d: [e] , hoisted_var , block_scoped_arrow_expr , ...f }, [g, { h , i: j  }, [k], hoisted_func, block_scoped_func_expr, ...l], ...m){
            a, c, e, f, g, h, j, k, l, block_scoped_arrow_expr, block_scoped_func_expr, hoisted_var, hoisted_func;
        }
        static{}
    }
}var hoisted_var;
function hoisted_func() {}
