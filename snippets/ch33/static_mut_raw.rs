// `static mut` 在 2024 里只禁止「造引用」，不禁止裸指针。
static mut COUNTER: u32 = 0;

pub fn bump() {
    // SAFETY: 只在单线程里调用；`&raw mut` 不产生引用。
    unsafe {
        let c = &raw mut COUNTER;
        *c += 1;
    }
}

fn main() {
    bump();
    bump();
    // SAFETY: 单线程，且没有别的指针在使用这块内存。
    println!("{}", unsafe { COUNTER });
}
