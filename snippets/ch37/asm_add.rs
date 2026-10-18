// 用一条内联汇编写完 u64 加法（x86-64）。
use std::arch::asm;

fn add_via_asm(a: u64, b: u64) -> u64 {
    let out: u64;
    // SAFETY: 只动两个通用寄存器，不读写内存、不改栈；
    // 函数返回时寄存器状态由 asm! 的操作数声明接管。
    unsafe {
        asm!(
            "add {0}, {1}",
            inout(reg) a => out,
            in(reg) b,
            options(nomem, nostack, pure),
        );
    }
    out
}

fn main() {
    println!("{}", add_via_asm(2, 3));
    // asm! 里的 add 是机器指令：溢出直接回绕，不受 -C overflow-checks 管。
    println!("{}", add_via_asm(u64::MAX, 1));
}
