// 手写 micro-bench：std::time::Instant + std::hint::black_box。
// 本机实测：Windows x86-64、Rust 1.98；release 与 dev 两档各跑一次（见正文）。
use std::hint::black_box;
use std::sync::Arc;
use std::time::Instant;

const ROUNDS: u32 = 20;

/// 跑 ROUNDS 次，取最快的一次，再除以每次 f() 里做掉的 ops 次操作。
/// 取最小值而不是平均值：调度抖动只会让某一轮变慢，不会让它变快。
fn bench(ops: u64, mut f: impl FnMut() -> u64) -> f64 {
    black_box(f()); // 预热：让分配、页错误、分支预测先热起来
    let mut best = u128::MAX;
    for _ in 0..ROUNDS {
        let t = Instant::now();
        let out = f();
        let dt = t.elapsed().as_nanos();
        black_box(out); // 结果没人消费，整段调用可能被优化掉
        best = best.min(dt);
    }
    best as f64 / ops as f64
}

fn sum_index(xs: &[u64]) -> u64 {
    let mut s = 0;
    for i in 0..xs.len() {
        s += xs[i];
    }
    s
}

fn main() {
    let n = black_box(1_000_000u64);
    let data: Vec<u64> = (0..n).map(|i| i % 7).collect();

    println!("每次操作的纳秒数（20 轮取最小值；耗时会随机器浮动）");

    println!(
        "{:>8.2} ns/op  索引循环求和",
        bench(n, || sum_index(black_box(&data)))
    );
    println!(
        "{:>8.2} ns/op  迭代器求和",
        bench(n, || data.iter().sum::<u64>())
    );

    println!(
        "{:>8.2} ns/op  push 无容量预留",
        bench(n, || {
            let mut v = Vec::new();
            for i in 0..n {
                v.push(i);
            }
            v.len() as u64
        })
    );
    println!(
        "{:>8.2} ns/op  push 有 with_capacity",
        bench(n, || {
            let mut v = Vec::with_capacity(n as usize);
            for i in 0..n {
                v.push(i);
            }
            v.len() as u64
        })
    );

    let clones = black_box(200_000u64);
    let small: Vec<u64> = vec![0; 16];
    let arc = Arc::new(small.clone());
    println!(
        "{:>8.2} ns/op  Vec<u64> clone（16 个元素）",
        bench(clones, || {
            let mut acc = 0u64;
            for _ in 0..clones {
                acc = acc.wrapping_add(black_box(small.clone())[15]);
            }
            acc
        })
    );
    println!(
        "{:>8.2} ns/op  Arc<Vec<u64>> clone",
        bench(clones, || {
            let mut acc = 0u64;
            for _ in 0..clones {
                acc = acc.wrapping_add(black_box(Arc::clone(&arc)).len() as u64);
            }
            acc
        })
    );

    // 反面教材：结果不消费，纯计算循环会被整个删掉。
    let m = black_box(10_000_000u64);
    let t = Instant::now();
    let mut dead = 0u64;
    for i in 0..m {
        dead = dead.wrapping_add(i);
    }
    let dt_dead = t.elapsed().as_nanos();
    let t = Instant::now();
    let mut live = 0u64;
    for i in 0..m {
        live = black_box(live.wrapping_add(i));
    }
    let dt_live = t.elapsed().as_nanos();
    println!("{m} 次加法：结果不消费 {dt_dead} ns；每次结果走 black_box {dt_live} ns");
    black_box((dead, live));
}
