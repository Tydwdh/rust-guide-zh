// 声明宏的卫生边界：局部变量与标签按定义处解析，其余名字按调用处解析。
// 本机实测（Windows x86-64、Rust 1.98）。
mod defs {
    pub fn helper() -> u32 {
        10
    }
}

// 宏体里调用的是**函数**，所以它按调用处解析：
// 定义处这里没有 helper，调用处必须自己把它引进来（见正文的报错现场）。
macro_rules! call_helper {
    () => {
        helper()
    };
}
use defs::helper;

// 由调用方传进来的标识符，落在调用方的命名空间里：
// 宏造出的 item 名与调用方的其它名字共享同一个值命名空间，撞名就是 E0428。
macro_rules! make_const {
    ($name:ident, $value:expr) => {
        const $name: u32 = $value;
    };
}
make_const!(LIMIT, 1);
make_const!(CEILING, 2);

fn main() {
    println!("call_helper!() = {}", call_helper!());
    println!("LIMIT          = {LIMIT}");
    println!("CEILING        = {CEILING}");

    // 局部变量是卫生的：宏里的 x 和调用处的 x 是两个绑定（第 27 章）
    macro_rules! bump {
        ($v:expr) => {{
            let x = $v;
            x + 1
        }};
    }
    let x = 10;
    println!("bump!(x)       = {}（调用处的 x 仍是 {x}）", bump!(x));
}
