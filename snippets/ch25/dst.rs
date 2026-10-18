// DST 与胖指针：本机实测（Windows x86-64、Rust 1.98）。
use std::fmt::Debug;
use std::mem::{align_of, align_of_val, size_of, size_of_val};

#[derive(Debug)]
struct Dot;

#[derive(Debug)]
struct Circle {
    r: f64,
}

fn main() {
    // 胖指针本身的大小：指针 + 元数据，两个机器字。
    println!("&[u64; 3]      {}", size_of::<&[u64; 3]>());
    println!("&[u64]        {}", size_of::<&[u64]>());
    println!("&str          {}", size_of::<&str>());
    println!("Box<[u64; 3]> {}", size_of::<Box<[u64; 3]>>());
    println!("Box<[u64]>    {}", size_of::<Box<[u64]>>());
    println!("Box<str>      {}", size_of::<Box<str>>());
    println!("&dyn Debug    {}", size_of::<&dyn Debug>());
    println!("Box<dyn Debug> {}", size_of::<Box<dyn Debug>>());

    // 被指向对象的大小：编译期不知道，只能问胖指针。
    let arr = [0u64; 3];
    let s: &[u64] = &arr;
    println!(
        "size_of_val(&[u64; 3] 转 &[u64]) = {} align_of_val = {}",
        size_of_val(s),
        align_of_val(s)
    );

    let st: &str = "héllo";
    println!(
        "str 的字节数={} 字符数={} size_of_val={} align_of_val={}",
        st.len(),
        st.chars().count(),
        size_of_val(st),
        align_of_val(st)
    );

    let a: &dyn Debug = &Circle { r: 1.0 };
    let b: &dyn Debug = &Dot;
    println!(
        "同一个 &dyn Debug 指向不同值：size_of_val = {} / {}，align_of_val = {} / {}",
        size_of_val(a),
        size_of_val(b),
        align_of_val(a),
        align_of_val(b)
    );

    let boxed: Box<dyn Debug> = Box::new(Circle { r: 2.0 });
    println!(
        "Box<dyn Debug> 里的值：size_of_val = {}",
        size_of_val(&*boxed)
    );

    // 零大小类型也有对齐，但大小为 0。
    println!(
        "[u16; 0]: size={} align={}",
        size_of::<[u16; 0]>(),
        align_of::<[u16; 0]>()
    );
}
