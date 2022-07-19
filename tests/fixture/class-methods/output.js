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
        this.prototype.capture["[[Closure]]"] = [
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
    }
}
var hoisted_var;
function hoisted_func() {}
