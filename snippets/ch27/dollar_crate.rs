// $crate：展开式里的路径按「定义宏的 crate」解析。本机实测（Windows x86-64、Rust 1.98）。
pub mod inner {
    #[macro_export]
    macro_rules! call_foo {
        () => {
            $crate::inner::foo()
        };
    }

    pub fn foo() -> &'static str {
        "crate 里的 foo"
    }
}

// 调用处也有一个同名函数：宏展开不会误取它
fn foo() -> &'static str {
    "调用处的 foo"
}

fn main() {
    println!("直接调用 foo()     -> {}", foo());
    println!("call_foo!()        -> {}", call_foo!());
    println!("crate::call_foo!() -> {}", crate::call_foo!());
}
