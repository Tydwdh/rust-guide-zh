// 匹配臂与展开：本机实测（Windows x86-64、Rust 1.98）。
macro_rules! shape {
    // 字面 token 可以直接匹配
    (zero) => {
        println!("1: 字面 token zero")
    };
    (one) => {
        println!("2: 字面 token one")
    };
    // 元变量：$名字:片段说明符
    ($e:expr) => {
        println!("3: 表达式 {}", stringify!($e))
    };
    // 兜底臂：接受任意 token 序列
    ($($tt:tt)*) => {
        println!("4: 其余 token {:?}", stringify!($($tt)*))
    };
}

// 臂按书写顺序尝试，第一处匹配胜出
macro_rules! first {
    ($e:expr) => {
        println!("A: expr 臂赢")
    };
    (1 + 1) => {
        println!("B: 永远轮不到")
    };
}

// 匹配器的外层定界符可以是任意一对
macro_rules! parens {
    (()) => {
        println!("parens: 匹配了")
    };
}

// ident 片段能匹配关键字（`_` 除外）
macro_rules! kind {
    ($i:ident) => {
        println!("ident 匹配: {}", stringify!($i))
    };
}

fn main() {
    shape!(zero);
    shape!(one);
    shape!(1 + 2);
    shape!(a b c);
    first!(1 + 1);
    parens![()];
    parens! {()};
    kind!(fn);
    kind!(struct);
}
