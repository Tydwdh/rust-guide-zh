// naked 函数：整个函数体就是一段汇编，编译器不加序言与尾声。
// 注意 ABI 必须是 "C"/"sysv64" 这类外部 ABI，不能用 Rust ABI。
#[unsafe(naked)]
#[unsafe(no_mangle)]
pub extern "C" fn add(a: u32, b: u32) -> u32 {
    // win64 调用约定：第 1、2 个参数在 ECX、EDX，返回值放 EAX。
    core::arch::naked_asm!("lea eax, [rcx + rdx]", "ret");
}

fn main() {
    println!("{}", add(2, 3));
}
