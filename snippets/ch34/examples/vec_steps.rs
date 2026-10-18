//! 打印增长台阶、ZST 容量与关键布局数字。
//!
//! 跑法：`cargo run --example vec_steps`（在 snippets/ch34 下）。

use ch34_snippets::mini_arc::MiniArc;
use ch34_snippets::mini_vec::MiniVec;
use std::mem::size_of;
use std::sync::Arc;

/// 记录「容量发生变化」的那些时刻，得到台阶序列。
fn collect_steps<T>(mut push: impl FnMut(&mut Vec<T>, usize), n: usize) -> Vec<usize> {
    let mut v: Vec<T> = Vec::new();
    let mut steps = Vec::new();
    for i in 0..n {
        push(&mut v, i);
        if steps.last() != Some(&v.capacity()) {
            steps.push(v.capacity());
        }
    }
    steps
}

fn main() {
    println!("== std::vec::Vec 的容量台阶（40 次 push，去重后）==");
    println!(
        "Vec<u8>       : {:?}",
        collect_steps(|v: &mut Vec<u8>, i| v.push(i as u8), 40)
    );
    println!(
        "Vec<u64>      : {:?}",
        collect_steps(|v: &mut Vec<u64>, i| v.push(i as u64), 40)
    );
    println!(
        "Vec<[u64;300]>: {:?}",
        collect_steps(|v: &mut Vec<[u64; 300]>, _| v.push([0; 300]), 40)
    );

    println!();
    println!("== MiniVec 的容量台阶（同一次数）==");
    let mut v: MiniVec<u32> = MiniVec::new();
    let mut steps = Vec::new();
    for i in 0..40u32 {
        v.push(i);
        if steps.last() != Some(&v.capacity()) {
            steps.push(v.capacity());
        }
    }
    println!("MiniVec<u32>  : {steps:?}");

    println!();
    println!("== ZST ==");
    println!("Vec::<()>::new().capacity() = {}", Vec::<()>::new().capacity());
    let mut z: Vec<()> = Vec::new();
    for _ in 0..10 {
        z.push(());
    }
    println!("std    push 10 次: cap = {}, len = {}", z.capacity(), z.len());
    let mut mz: MiniVec<()> = MiniVec::new();
    for _ in 0..10 {
        mz.push(());
    }
    println!("MiniVec push 10 次: cap = {}, len = {}", mz.capacity(), mz.len());

    println!();
    println!("== 布局（第 25 章的量法）==");
    println!("size_of::<Vec<u8>>()                     = {}", size_of::<Vec<u8>>());
    println!("size_of::<Option<Vec<u8>>>()             = {}", size_of::<Option<Vec<u8>>>());
    println!("size_of::<MiniVec<u8>>()                 = {}", size_of::<MiniVec<u8>>());
    println!("size_of::<Option<MiniVec<u8>>>()         = {}", size_of::<Option<MiniVec<u8>>>());
    println!("size_of::<MiniVec<()>>()                 = {}", size_of::<MiniVec<()>>());
    println!("size_of::<Arc<u128>>()                   = {}", size_of::<Arc<u128>>());
    println!("size_of::<Option<Arc<u128>>>()           = {}", size_of::<Option<Arc<u128>>>());
    println!("size_of::<MiniArc<u128>>()               = {}", size_of::<MiniArc<u128>>());
    println!("size_of::<Option<MiniArc<u128>>>()       = {}", size_of::<Option<MiniArc<u128>>>());

    println!();
    println!("== 字节转储：MiniVec<u8> 的 3 个机器字（第 25 章的读法）==");
    let mut b: MiniVec<u8> = MiniVec::new();
    b.push(0xAA);
    b.push(0xBB);
    b.push(0xCC);
    // SAFETY: MiniVec<u8> 就占 3 个机器字；这里只是把它按 usize 读出来观察布局，
    // 读完就不再持有这些引用了。字段顺序由 repr(Rust) 决定，不是接口。
    let words = unsafe { std::slice::from_raw_parts((&b as *const MiniVec<u8>).cast::<usize>(), 3) };
    println!("API 读数    : cap = {}, len = {}", b.capacity(), b.len());
    println!("三个机器字  : word0 = 首元素地址吗 -> {}", words[0] == b.as_ptr() as usize);
    println!("              word1 = {}, word2 = {}", words[1], words[2]);
    let buf = &b[..];
    println!(
        "缓冲区字节  : {:02X?}（只读 len 个：容量是 4，第 4 个字节还没初始化）",
        buf
    );

    println!();
    println!("== drain 只借出元素，不还缓冲区 ==");
    let mut d: MiniVec<i32> = MiniVec::new();
    for i in 0..5 {
        d.push(i);
    }
    let cap_before = d.capacity();
    println!("drain 之前: len = {}, capacity = {}", d.len(), cap_before);
    let taken: Vec<i32> = d.drain().collect();
    println!("drain 出来: {taken:?}");
    println!("drain 之后: len = {}, capacity = {}（缓冲区照旧）", d.len(), d.capacity());
    d.push(99);
    println!("缓冲区还能用: {:?}", d.as_slice());
}
