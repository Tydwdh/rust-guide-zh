use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Barrier};
use std::thread;

const ROUNDS: usize = 2_000_000;

struct Ctx {
    x: AtomicUsize,
    y: AtomicUsize,
    r: [AtomicUsize; 2],
    hits: AtomicUsize,
    start: Barrier,
    middle: Barrier,
    end: Barrier,
}

fn run(sord: Ordering, lord: Ordering, name: &str) {
    let ctx = Arc::new(Ctx {
        x: AtomicUsize::new(0),
        y: AtomicUsize::new(0),
        r: [AtomicUsize::new(0), AtomicUsize::new(0)],
        hits: AtomicUsize::new(0),
        start: Barrier::new(2),
        middle: Barrier::new(2),
        end: Barrier::new(2),
    });
    let mut handles = Vec::new();
    for t in 0..2usize {
        let ctx = Arc::clone(&ctx);
        handles.push(thread::spawn(move || {
            for _ in 0..ROUNDS {
                ctx.start.wait();
                let got = if t == 0 {
                    ctx.x.store(1, sord);
                    ctx.y.load(lord)
                } else {
                    ctx.y.store(1, sord);
                    ctx.x.load(lord)
                };
                ctx.r[t].store(got, Ordering::Relaxed);
                ctx.middle.wait();
                if t == 0 {
                    let both = ctx.r[0].load(Ordering::Relaxed) == 0
                        && ctx.r[1].load(Ordering::Relaxed) == 0;
                    if both {
                        ctx.hits.fetch_add(1, Ordering::Relaxed);
                    }
                    ctx.x.store(0, Ordering::Relaxed);
                    ctx.y.store(0, Ordering::Relaxed);
                }
                ctx.end.wait();
            }
        }));
    }
    for h in handles {
        h.join().unwrap();
    }
    println!(
        "{name:<16} 两者都读到 0 的轮数：{} / {ROUNDS}",
        ctx.hits.load(Ordering::Relaxed)
    );
}

fn main() {
    run(Ordering::Relaxed, Ordering::Relaxed, "Relaxed");
    run(Ordering::Release, Ordering::Acquire, "Release/Acquire");
    run(Ordering::SeqCst, Ordering::SeqCst, "SeqCst");
}
