// 片段说明符的实际用法：本机实测（Windows x86-64、Rust 1.98）。
use std::collections::HashMap;

// ident + ty + block：从名字、参数、返回类型拼出一个函数定义
macro_rules! def_fn {
    ($name:ident ( $($arg:ident : $ty:ty),* $(,)? ) -> $ret:ty $body:block) => {
        fn $name($($arg: $ty),*) -> $ret $body
    };
}

def_fn!(add(a: i32, b: i32,) -> i32 { a + b });

// literal：可以匹配带负号的字面量
macro_rules! lit {
    (42) => {
        println!("字面 token 42")
    };
    ($l:literal) => {
        println!("literal 片段: {}", stringify!($l))
    };
}

// path：只匹配一个类型路径
macro_rules! show_path {
    ($p:path) => {
        format!("path 片段: {}", stringify!($p))
    };
}

// meta：属性里逗号分隔的那部分
macro_rules! show_meta {
    (#[$m:meta]) => {
        format!("meta 片段: {}", stringify!($m))
    };
}

// vis：可见性限定符（可以为空）
macro_rules! show_vis {
    ($v:vis struct $n:ident) => {
        format!("vis 片段: {:?}", stringify!($v))
    };
}

// stmt：不带尾部分号的语句
macro_rules! show_stmt {
    ($s:stmt) => {
        format!("stmt 片段: {}", stringify!($s))
    };
}

// tt：一个 token 树（单个 token，或一整对定界符包起来的序列）
macro_rules! show_tt {
    ($t:tt) => {
        format!("tt 片段: {}", stringify!($t))
    };
}

// lifetime：一个生命周期名
macro_rules! show_lifetime {
    ($l:lifetime) => {
        format!("lifetime 片段: {}", stringify!($l))
    };
}

// pat：一个模式（2021 起包括顶层或模式）
macro_rules! show_pat {
    ($p:pat) => {
        format!("pat 片段: {}", stringify!($p))
    };
}

// 综合起来：一个从键值对列表造 HashMap 的宏
macro_rules! map {
    ( $( $k:expr => $v:expr ),* $(,)? ) => {{
        let mut m = HashMap::new();
        $( m.insert($k, $v); )*
        m
    }};
}

fn main() {
    println!("add(1, 2) = {}", add(1, 2));
    lit!(42);
    lit!(-7);
    println!("{}", show_path!(std::mem::size_of));
    println!("{}", show_meta!(#[derive(Debug, Clone)]));
    println!("{}", show_vis!(pub(crate) struct S));
    println!("{}", show_stmt!(let x = 1));
    println!("{}", show_tt!(a));
    println!("{}", show_tt!([1, 2, 3]));
    println!("{}", show_lifetime!('static));
    println!("{}", show_pat!(Some(x) | None));

    let scores = map! {
        "ra" => 90,
        "mut" => 85, // 尾逗号也可以
    };
    println!("map! 造出 {} 条记录，ra = {}", scores.len(), scores["ra"]);
}
