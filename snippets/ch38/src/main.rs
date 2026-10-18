//! 演示：手写状态机、任务队列、pin 投影各跑一遍。
//!
//! 在 `snippets/ch38/` 下 `cargo run` 即可。

use std::sync::{Arc, Mutex};

use ch38_runtime::{Budgeted, Countdown, Executor, TwoStep, block_on};

fn main() {
    // 一、手写的状态机交给 block_on 驱动
    println!("TwoStep(1, 2, 3) => {}", block_on(TwoStep::new(1, 2, 3)));

    // 二、两个任务交给同一个执行器：它们在队列里交替推进
    let log = Arc::new(Mutex::new(Vec::new()));
    let ex = Executor::new();
    ex.spawn(Countdown::sharing("A", 3, Arc::clone(&log)));
    ex.spawn(Countdown::sharing("B", 2, Arc::clone(&log)));
    let polls = ex.run();
    println!(
        "队列跑空：{} 次 poll，顺序 {}",
        polls,
        log.lock().unwrap().join(" ")
    );

    // 三、pin 投影包出来的安全 API：预算用尽就放弃
    let v = block_on(Budgeted::new(Countdown::new("C", 100), 3));
    println!("Budgeted 用尽预算 => {v:?}");

    // 四、同一条唤醒路径，换成手写 RawWaker 也得到一样的结果
    let log = Arc::new(Mutex::new(Vec::new()));
    let ex = Executor::with_raw_waker();
    ex.spawn(Countdown::sharing("D", 2, Arc::clone(&log)));
    let polls = ex.run();
    println!(
        "RawWaker 版：{} 次 poll，顺序 {}",
        polls,
        log.lock().unwrap().join(" ")
    );
}
