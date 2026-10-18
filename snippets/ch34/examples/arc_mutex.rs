//! 引用计数 + 互斥锁的演示：计数变化、析构时机、跨线程累加、弱引用对照。
//!
//! 跑法：`cargo run --example arc_mutex`（在 snippets/ch34 下）。

use ch34_snippets::mini_arc::MiniArc;
use ch34_snippets::mini_mutex::Mutex;
use std::sync::{Arc, Weak};
use std::thread;

/// 被共享的数据；析构时打个招呼，好把「什么时候释放」看得见。
struct Tag(&'static str);

impl Drop for Tag {
    fn drop(&mut self) {
        println!("    [drop] {}", self.0);
    }
}

fn main() {
    println!("== 简化版 Arc：计数与释放时机 ==");
    let a = MiniArc::new(Tag("mini Arc 的数据"));
    println!("new       : strong_count = {}", a.strong_count());
    let b = a.clone();
    println!("clone     : strong_count = {}", a.strong_count());
    drop(b);
    println!("drop 一份 : strong_count = {}", a.strong_count());
    drop(a);
    println!("强引用清零后，上面那行 [drop] 就是析构");

    println!();
    println!("== MiniArc<Mutex<T>>：多线程累加 ==");
    const THREADS: usize = 4;
    const PER_THREAD: usize = 50_000;
    let counter = MiniArc::new(Mutex::new(0usize));
    let mut handles = Vec::new();
    for _ in 0..THREADS {
        let mine = counter.clone();
        handles.push(thread::spawn(move || {
            for _ in 0..PER_THREAD {
                *mine.lock() += 1;
            }
        }));
    }
    for h in handles {
        h.join().unwrap();
    }
    println!("{THREADS} 个线程 × {PER_THREAD} 次 = {}", *counter.lock());
    println!("线程退出后 strong_count = {}", counter.strong_count());

    println!();
    println!("== 对照：std 的 Weak 不阻止释放 ==");
    let strong = Arc::new(Tag("std Arc 的数据"));
    let weak: Weak<Tag> = Arc::downgrade(&strong);
    println!(
        "建好 Weak  : strong_count = {}, weak_count = {}",
        Arc::strong_count(&strong),
        weak.weak_count()
    );
    let also_strong = Arc::clone(&strong);
    println!("再 clone 一份: strong_count = {}", Arc::strong_count(&strong));
    drop(also_strong);
    drop(strong);
    println!("强引用清零 : upgrade() 拿得到吗 -> {:?}", weak.upgrade().is_some());
    println!("Weak 还在  : weak_count = {}", weak.weak_count());
}
