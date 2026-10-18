//! 第 35 章「动手看看」用的探针：看编译器为引用形参贴上了哪些 LLVM 属性。
//! 编译方式（不经过 mdBook）：
//! `rustc --edition 2024 -O --emit=llvm-ir snippets/ch35/ir_probe.rs`
//! （在仓库根目录执行；`ir_probe.ll` 落在当前目录。本文件自带 `crate_type`。）

#![crate_type = "lib"]

#[unsafe(no_mangle)]
pub extern "C" fn compute(input: &u32, output: &mut u32) {
    if *input > 10 {
        *output = 1;
    }
    if *input > 5 {
        *output *= 2;
    }
}
