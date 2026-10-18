// &raw const / &raw mut：拿裸指针，但不先造出一个引用。
#[repr(packed)]
struct Packed {
    tag: u8,
    value: u32,
}

fn main() {
    let mut p = Packed { tag: 1, value: 0xDEAD_BEEF };

    // &p.value 会造出未对齐引用（UB），&raw const 不会。
    let value_ptr: *const u32 = &raw const p.value;
    println!("未对齐读 = {:#x}", unsafe { value_ptr.read_unaligned() });

    // 写也同理：&raw mut 拿到的指针可以直接写回。
    let tag_ptr: *mut u8 = &raw mut p.tag;
    unsafe { tag_ptr.write(2) };
    println!("写回后 tag = {}", p.tag);
    println!("标签地址 = {:#x}", tag_ptr.addr());

    // 对一个「看起来是普通类型」的地方，两种写法等价。
    let n = 7u32;
    let a: *const u32 = &raw const n;
    let b: *const u32 = &n;
    assert_eq!(a.addr(), b.addr());
    println!("n = {}", unsafe { *a });
}
