//! 跑一跑：`cargo run -p task-queue --example demo`
//!
//! 演示四件事：任务的完成顺序不保证、返回值经句柄收回、任务 panic 不带走池、
//! 关闭会把排队的活干完。

use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use task_queue::ThreadPool;

fn main() {
    let pool = ThreadPool::new(3);

    // 一、完成顺序：四个任务交给三个 worker，谁先跑完不保证。
    let (tx, rx) = mpsc::channel();
    for name in ["甲", "乙", "丙", "丁"] {
        let tx = tx.clone();
        pool.spawn(move || {
            thread::sleep(Duration::from_millis(20));
            tx.send(name).unwrap();
        });
    }
    drop(tx);
    let order: Vec<&str> = rx.iter().collect();
    println!("完成顺序：{}", order.join(" "));

    // 二、返回值：句柄自己带着一条通道。
    let handles: Vec<_> = (1..=4).map(|n| pool.submit(move || n * n)).collect();
    let sum: i32 = handles.into_iter().map(|h| h.join().unwrap()).sum();
    println!("平方和：{sum}");

    // 三、panic 的任务只炸自己。
    let bad = pool.submit(|| panic!("这个任务故意炸了"));
    match bad.join() {
        Ok(()) => println!("不该发生"),
        Err(payload) => {
            let message = payload.downcast_ref::<&str>().copied().unwrap_or("（非字符串负载）");
            println!("任务 panic 了：{message}");
        }
    }
    println!("池还活着：{}", pool.submit(|| "是的").join().unwrap());

    // 四、关闭：排队的活跑完，worker 才退。
    let (done_tx, done_rx) = mpsc::channel();
    for n in 0..5 {
        let done = done_tx.clone();
        pool.spawn(move || {
            done.send(n).unwrap();
        });
    }
    drop(done_tx);
    pool.shutdown();
    let done: Vec<i32> = done_rx.iter().collect();
    println!("关闭前排队的任务都跑完了：{done:?}");
}
