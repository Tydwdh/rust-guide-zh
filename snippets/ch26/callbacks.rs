//! 进程内演示 FFI 的三块拼板：`extern "C"` 回调、导出符号、panic 边界。
//!
//! 「C 侧」由本文件里的 `drive` / `drive_unwind` 扮演，它们和真正的 C 代码
//! 一样只认函数指针；正文里有一份把同样三个函数交给 gcc 编出来的 C 库的版本。

use std::panic::{catch_unwind, AssertUnwindSafe};

/// 驱动方：接收函数指针，按 C 的约定调用它。
#[unsafe(no_mangle)]
pub extern "C" fn drive(f: extern "C" fn(i32) -> i32, x: i32) -> i32 {
    f(x)
}

/// 回调方自保：panic 关在 `extern "C"` 边界之内。
extern "C" fn checked_div(x: i32) -> i32 {
    match catch_unwind(|| {
        if x == 0 {
            panic!("divide by zero");
        }
        100 / x
    }) {
        Ok(v) => v,
        Err(_) => -1,
    }
}

/// 想让 panic 合法地穿过边界：两侧都写成 `extern "C-unwind"`。
#[unsafe(no_mangle)]
pub extern "C-unwind" fn drive_unwind(f: extern "C-unwind" fn(i32) -> i32, x: i32) -> i32 {
    f(x)
}

extern "C-unwind" fn raw_div(x: i32) -> i32 {
    if x == 0 {
        panic!("divide by zero");
    }
    100 / x
}

extern "C" fn double(x: i32) -> i32 {
    x * 2
}

fn main() {
    println!("drive(double, 21)      = {}", drive(double, 21));
    println!("drive(checked_div, 4)  = {}", drive(checked_div, 4));
    println!("drive(checked_div, 0)  = {}", drive(checked_div, 0));

    let caught = catch_unwind(AssertUnwindSafe(|| drive_unwind(raw_div, 0)));
    println!("外层抓住 drive_unwind(raw_div, 0)：{}", caught.is_err());
    println!("drive_unwind(raw_div, 5) = {}", drive_unwind(raw_div, 5));
}
