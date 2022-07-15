const { a , b: c , d: [e] , ...f } = {};
const [g, { h , i: j  }, [k], ...l] = [];
const m = [];
const block_scoped_arrow_expr = ()=>{};
const block_scoped_func_expr = function() {};
class A {
    capture() {
        a, c, e, f, g, h, j, k, l, block_scoped_arrow_expr, block_scoped_func_expr, hoisted_var, hoisted_func;
    }
    shadowed({ a , b: c , d: [e] , hoisted_var , block_scoped_arrow_expr , ...f }, [g, { h , i: j  }, [k], hoisted_func, block_scoped_func_expr, ...l], ...m) {
        a, c, e, f, g, h, j, k, l, block_scoped_arrow_expr, block_scoped_func_expr, hoisted_var, hoisted_func;
    }
    static{
        global.__fnl_func(this.prototype.capture, ()=>({
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
    }
}
var hoisted_var;
function hoisted_func() {}
