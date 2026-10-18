// 枚举布局与 niche：本机实测（Windows x86-64、Rust 1.98）。
use std::mem::{align_of, size_of};
use std::num::NonZeroU32;

// 三个无字段变体，没写 repr。
enum Plain3 {
    A,
    B,
    C,
}

#[repr(u8)]
enum Tagged8 {
    A,
    B,
    C,
}

#[repr(u32)]
enum Tagged32 {
    A,
    B,
    C,
}

// 两个变体的载荷都是 u64：没有空闲位模式可用，判别式只能另占地方。
enum TwoU64 {
    A(u64),
    B(u64),
}

// repr(C) 的带字段枚举：C 侧看到的是一个 tagged union。
#[repr(C)]
enum CMsg {
    Pair(u8, u32),
    One(u64),
}

fn main() {
    println!(
        "Plain3   size={} align={}   Option<Plain3>  size={}",
        size_of::<Plain3>(),
        align_of::<Plain3>(),
        size_of::<Option<Plain3>>()
    );
    println!(
        "Tagged8  size={} align={}   Option<Tagged8> size={}",
        size_of::<Tagged8>(),
        align_of::<Tagged8>(),
        size_of::<Option<Tagged8>>()
    );
    println!(
        "Tagged32 size={} align={}",
        size_of::<Tagged32>(),
        align_of::<Tagged32>()
    );
    println!(
        "TwoU64   size={} align={}   Option<TwoU64>  size={}",
        size_of::<TwoU64>(),
        align_of::<TwoU64>(),
        size_of::<Option<TwoU64>>()
    );
    println!(
        "CMsg     size={} align={}",
        size_of::<CMsg>(),
        align_of::<CMsg>()
    );

    println!("--- niche 优化 ---");
    println!("&u64                  {}", size_of::<&u64>());
    println!("Option<&u64>          {}", size_of::<Option<&u64>>());
    println!("Box<u64>              {}", size_of::<Box<u64>>());
    println!("Option<Box<u64>>      {}", size_of::<Option<Box<u64>>>());
    println!("u32                   {}", size_of::<u32>());
    println!("Option<u32>           {}", size_of::<Option<u32>>());
    println!("NonZeroU32            {}", size_of::<NonZeroU32>());
    println!("Option<NonZeroU32>    {}", size_of::<Option<NonZeroU32>>());
    println!("bool                  {}", size_of::<bool>());
    println!("Option<bool>          {}", size_of::<Option<bool>>());
    println!("Option<Option<bool>>  {}", size_of::<Option<Option<bool>>>());
    println!("*const u8             {}", size_of::<*const u8>());
    println!("Option<*const u8>     {}", size_of::<Option<*const u8>>());
    println!("Option<fn()>          {}", size_of::<Option<fn()>>());
}
