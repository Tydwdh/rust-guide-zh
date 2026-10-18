//! panic 策略对照实验：同一份源码，`panic = "unwind"` 与 `panic = "abort"` 各编一次。
//!
//! 用法：
//!   cargo run --release                     走 catch_unwind 那条路
//!   cargo run --release -- uncaught         不加捕获，让 panic 一路走到底
//!   cargo run --profile release-abort       把上面两条换成 abort 档

use std::panic::catch_unwind;

/// 一个只为在析构时留下痕迹的哨兵，用来观察 Drop 有没有跑。
struct Guard(&'static str);

impl Drop for Guard {
    fn drop(&mut self) {
        println!("  drop {}", self.0);
    }
}

fn deepest() {
    let _g = Guard("deepest");
    panic!("模拟生产事故");
}

fn middle() {
    let _g = Guard("middle");
    deepest();
}

fn main() {
    // 默认（无参数）走捕获那条路，这样直接 `cargo run` 也能正常退出。
    if std::env::args().nth(1).as_deref() == Some("uncaught") {
        let _g = Guard("main");
        middle();
    } else {
        let result = catch_unwind(|| {
            let _g = Guard("main");
            middle();
        });
        println!("catch_unwind 拦住了：{}", result.is_err());
    }
}
