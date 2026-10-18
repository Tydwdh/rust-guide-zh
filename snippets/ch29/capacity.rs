// Vec / String 的容量增长点：只取决于标准库实现，不看机器快慢。
use std::mem::size_of;

fn growth<T: Default>(label: &str) {
    let mut v: Vec<T> = Vec::new();
    let mut last = 0;
    print!("{label:<10} size_of={:<3} 容量台阶：", size_of::<T>());
    for _ in 0..12 {
        if v.capacity() != last {
            print!("{}@len{} ", v.capacity(), v.len());
            last = v.capacity();
        }
        v.push(T::default()); // 只关心容量，不看元素值
    }
    println!("(len={} capacity={})", v.len(), v.capacity());
}

fn main() {
    growth::<u8>("Vec<u8>");
    growth::<u16>("Vec<u16>");
    growth::<u32>("Vec<u32>");
    growth::<u64>("Vec<u64>");

    let mut v: Vec<u32> = Vec::with_capacity(100);
    for i in 0..50u32 {
        v.push(i);
    }
    println!(
        "with_capacity(100) 后 push 50 个：len={} capacity={}",
        v.len(),
        v.capacity()
    );
    v.shrink_to_fit();
    println!("shrink_to_fit 之后：len={} capacity={}", v.len(), v.capacity());

    let mut s = String::new();
    let mut last = 0;
    print!("String 容量台阶（每次 push 1 字节）：");
    for _ in 0..16 {
        if s.capacity() != last {
            print!("{}@len{} ", s.capacity(), s.len());
            last = s.capacity();
        }
        s.push('x');
    }
    println!("(len={} capacity={})", s.len(), s.capacity());
}
