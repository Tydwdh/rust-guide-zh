// 字段顺序、填充与 repr：本机实测（Windows x86-64、Rust 1.98）。
use std::mem::{align_of, offset_of, size_of};

// 默认表示：字段声明顺序是 u8、u32、u8、u64。
struct Mixed {
    a: u8,
    b: u32,
    c: u8,
    d: u64,
}

// 同样的字段，顺序由 repr(C) 钉死。
#[repr(C)]
struct CMixed {
    a: u8,
    b: u32,
    c: u8,
    d: u64,
}

// 一个典型的 FFI 头：C 侧写的是 struct { uint8_t tag; uint32_t len; }。
#[repr(C)]
struct CHeader {
    tag: u8,
    len: u32,
}

#[repr(C, align(16))]
struct Aligned16(u64);

#[repr(transparent)]
struct Meters(f64);

fn main() {
    println!(
        "Mixed    size={} align={}",
        size_of::<Mixed>(),
        align_of::<Mixed>()
    );
    println!(
        "  偏移 a={} b={} c={} d={}",
        offset_of!(Mixed, a),
        offset_of!(Mixed, b),
        offset_of!(Mixed, c),
        offset_of!(Mixed, d)
    );

    println!(
        "CMixed   size={} align={}",
        size_of::<CMixed>(),
        align_of::<CMixed>()
    );
    println!(
        "  偏移 a={} b={} c={} d={}",
        offset_of!(CMixed, a),
        offset_of!(CMixed, b),
        offset_of!(CMixed, c),
        offset_of!(CMixed, d)
    );

    println!(
        "CHeader  size={} align={} 偏移 tag={} len={}",
        size_of::<CHeader>(),
        align_of::<CHeader>(),
        offset_of!(CHeader, tag),
        offset_of!(CHeader, len)
    );

    println!(
        "Aligned16 size={} align={}",
        size_of::<Aligned16>(),
        align_of::<Aligned16>()
    );
    println!(
        "Meters   size={} align={}（f64 是 {} / {}）",
        size_of::<Meters>(),
        align_of::<Meters>(),
        size_of::<f64>(),
        align_of::<f64>()
    );
}
