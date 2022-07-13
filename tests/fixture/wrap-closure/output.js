let i = 0;

const a = global.wrapClosure(()=>{
    i;
}, ()=>({
        __filename,
        free: [
            {
                name: i
            }
        ]
    }));