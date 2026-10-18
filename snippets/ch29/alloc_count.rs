// 数分配次数：自定义全局分配器。次数是确定的，不受机器快慢影响。
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

struct Counting;

static ALLOCS: AtomicUsize = AtomicUsize::new(0);
static BYTES: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        BYTES.fetch_add(layout.size(), Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static A: Counting = Counting;

/// 打印 f() 期间发生的分配次数与总字节数（realloc 默认实现会走 alloc + dealloc）。
fn measure(name: &str, f: impl FnOnce()) {
    let a0 = ALLOCS.load(Ordering::Relaxed);
    let b0 = BYTES.load(Ordering::Relaxed);
    f();
    let a = ALLOCS.load(Ordering::Relaxed) - a0;
    let b = BYTES.load(Ordering::Relaxed) - b0;
    println!("{a:>3} 次分配 / {b:>7} 字节   {name}");
}

fn main() {
    measure("Vec::new + push 1000 个 u32", || {
        let mut v = Vec::new();
        for i in 0..1000u32 {
            v.push(i);
        }
        std::hint::black_box(v);
    });
    measure("Vec::with_capacity(1000) + push", || {
        let mut v = Vec::with_capacity(1000);
        for i in 0..1000u32 {
            v.push(i);
        }
        std::hint::black_box(v);
    });
    measure("String::new + push 1000 个字符", || {
        let mut s = String::new();
        for _ in 0..1000 {
            s.push('x');
        }
        std::hint::black_box(s);
    });
    measure("format! 一次", || {
        let s = format!("{}-{:?}-{}", 1, [2, 3], 4);
        std::hint::black_box(s);
    });

    let v = vec![0u64; 1000];
    measure("Vec<u64> clone（1000 个元素）", || {
        std::hint::black_box(v.clone());
    });
    let arc = Arc::new(vec![0u64; 1000]);
    measure("Arc<Vec<u64>> clone", || {
        std::hint::black_box(Arc::clone(&arc));
    });
    let s = String::from("hello, world");
    measure("Box<str> from &str（含拷贝）", || {
        std::hint::black_box(Box::<str>::from(s.as_str()));
    });
}
