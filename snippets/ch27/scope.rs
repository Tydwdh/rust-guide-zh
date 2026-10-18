// #[macro_export] 与作用域：本机实测（Windows x86-64、Rust 1.98）。
// #[macro_export] 把宏放到 crate 根，供路径解析
#[macro_export]
macro_rules! shout {
    ($($t:tt)*) => {
        std::format!("{}!", stringify!($($t)*))
    };
}

// 重导出：给宏一条路径（不带 macro_export 的宏只有文本作用域）
pub(crate) use shout as yell;

mod mac {
    macro_rules! inner {
        () => {
            "mac::inner"
        };
    }

    pub(crate) use inner;
}

mod user {
    pub fn via_use() -> &'static str {
        // 按路径引入后可以直接写 inner!()
        use crate::mac::inner;
        inner!()
    }

    pub fn via_path() -> &'static str {
        crate::mac::inner!()
    }
}

// #[macro_use] 让模块内宏的文本作用域延伸到模块之外
#[macro_use]
mod lifted_mod {
    macro_rules! lifted {
        () => {
            "被 macro_use 提升的宏"
        };
    }
}

// 文本作用域遮蔽路径作用域
macro_rules! shadowed {
    () => {
        "文本作用域的 shadowed"
    };
}

macro_rules! other {
    () => {
        "other"
    };
}

use other as shadowed;

fn main() {
    println!("crate::shout!(\"hi\") -> {}", crate::shout!("hi"));
    println!("yell!(\"alias\")     -> {}", yell!("alias"));
    println!("user::via_use()    -> {}", user::via_use());
    println!("user::via_path()   -> {}", user::via_path());
    println!("lifted!()          -> {}", lifted!());
    println!("shadowed!()        -> {}", shadowed!());
}
