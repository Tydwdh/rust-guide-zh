// Edition 2024 的片段匹配变化：本机实测（Windows x86-64、Rust 1.98，edition 2024）。
// 同一个宏在 2021 版会选另一条臂 —— 正文给了三次编译的实测对比。
macro_rules! which {
    ($e:expr) => {
        "expr 臂"
    };
    (const $e:expr) => {
        "const 臂"
    };
}

macro_rules! under {
    ($e:expr) => {
        "expr 臂"
    };
    (_) => {
        "下划线臂"
    };
}

// expr_2021 保留 2024 之前的行为（在所有 edition 里）
macro_rules! which2021 {
    ($e:expr_2021) => {
        "expr_2021 臂"
    };
    (const $e:expr) => {
        "const 臂"
    };
}

// pat 自 2021 起接受顶层或模式
macro_rules! or_pat {
    ($p:pat) => {
        format!("pat 片段接住: {}", stringify!($p))
    };
}

// 需要紧跟 `|` 时用 pat_param（它不接受顶层 `|`）
macro_rules! any_of {
    ($e:expr, $($p:pat_param)|+) => {
        match $e {
            $($p)|+ => true,
            _ => false,
        }
    };
}

fn main() {
    println!("which!(const {{ 1 + 1 }})    -> {}", which!(const { 1 + 1 }));
    println!("under!(_)                    -> {}", under!(_));
    println!("which2021!(const {{ 1 + 1 }}) -> {}", which2021!(const { 1 + 1 }));
    println!("or_pat!(Some(x) | None)      -> {}", or_pat!(Some(x) | None));
    println!("any_of!(3, 1 | 2 | 3)        -> {}", any_of!(3, 1 | 2 | 3));
    println!("any_of!(9, 1 | 2 | 3)        -> {}", any_of!(9, 1 | 2 | 3));
}
