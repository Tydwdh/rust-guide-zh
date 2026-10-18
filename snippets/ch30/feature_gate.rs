// features 的编译期与运行期两面。
//
// 本文件可以用 rustc 直接编译、运行：不带任何 feature 时，cfg 为假的分支整块
// 不参与编译。真实项目里 feature 由 Cargo 打开，它做的就是给 rustc 传
// `--cfg feature="bmp"`（见正文）。

use std::fmt::Write as _;

/// 只有打开 bmp feature 时才存在的模块。
#[cfg(feature = "bmp")]
pub mod bmp {
    pub fn decode() -> &'static str {
        "BMP"
    }
}

/// 同一个函数两套实现，靠 cfg 在编译期二选一。
#[cfg(feature = "bmp")]
pub fn backend() -> &'static str {
    "bmp"
}

#[cfg(not(feature = "bmp"))]
pub fn backend() -> &'static str {
    "cpu"
}

/// `cfg!` 是编译期定值、运行期可读的布尔量。
pub fn enabled() -> String {
    let mut out = String::new();
    for (name, on) in [("bmp", cfg!(feature = "bmp")), ("png", cfg!(feature = "png"))] {
        if on {
            let _ = write!(out, "{name} ");
        }
    }
    if out.is_empty() {
        out.push_str("(无)");
    }
    out.trim_end().to_string()
}

fn main() {
    println!("backend = {}", backend());
    println!("enabled = {}", enabled());
}
