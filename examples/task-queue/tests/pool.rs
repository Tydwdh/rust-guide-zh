//! 集成测试：只用公开 API，把池当黑盒用。
//!
//! 并发用例的稳定性靠三件事：把「先后」写成同步事实（闸门、交会通道），
//! 而不是靠 `sleep` 猜时间；肯定断言给一个宽到不可能误伤的上界；否定断言
//! 由上一步的协议保证（真要坏，也只是晚一点被判定为失败）。

use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use task_queue::{ThreadPool, Timeout};

/// 否定断言的等待上限：它只在「本来就不该发生」时被用到。
const SHORT: Duration = Duration::from_millis(200);

/// 肯定断言的上限：正常机器上远够用，超了就是真坏了。
const LONG: Duration = Duration::from_secs(5);

#[test]
fn every_submitted_task_returns_its_value() {
    let pool = ThreadPool::new(4);
    let handles: Vec<_> = (0..50).map(|n| (n, pool.submit(move || n * n))).collect();
    // 句柄按提交顺序排在 Vec 里，与任务跑完的先后无关。
    for (n, handle) in handles {
        assert_eq!(handle.join().unwrap(), n * n);
    }
}

#[test]
fn tasks_can_be_submitted_from_several_threads() {
    let pool = ThreadPool::new(4);
    let total: i32 = thread::scope(|s| {
        let pool = &pool;
        let submitters: Vec<_> = (0..4)
            .map(|batch| {
                s.spawn(move || {
                    (0..25)
                        .map(|i| pool.submit(move || batch * 100 + i))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        submitters
            .into_iter()
            .flat_map(|handle| handle.join().unwrap())
            .map(|handle| handle.join().unwrap())
            .sum()
    });
    let expected: i32 = (0..4).flat_map(|b| (0..25).map(move |i| b * 100 + i)).sum();
    assert_eq!(total, expected);
}

#[test]
fn every_worker_can_run_a_task_at_the_same_time() {
    let pool = ThreadPool::new(3);
    let (started_tx, started_rx) = mpsc::channel();
    // 每个任务配一条只属于自己的「放行」通道：三个任务都得先报到，
    // 主线程才放它们走 —— 串行执行的池会在第二个报到上卡住并超时。
    let mut gates = Vec::new();
    for _ in 0..3 {
        let started = started_tx.clone();
        let (gate_tx, gate_rx) = mpsc::channel::<()>();
        gates.push(gate_tx);
        pool.spawn(move || {
            started.send(()).unwrap();
            gate_rx.recv().unwrap();
        });
    }
    for _ in 0..3 {
        started_rx
            .recv_timeout(LONG)
            .expect("三个任务应当同时在跑");
    }
    for gate in gates {
        gate.send(()).unwrap();
    }
    pool.shutdown();
}

#[test]
fn a_panicking_task_does_not_take_the_pool_with_it() {
    let pool = ThreadPool::new(2);
    let payload = pool
        .submit(|| panic!("任务里故意炸的"))
        .join()
        .unwrap_err();
    assert_eq!(payload.downcast_ref::<&str>(), Some(&"任务里故意炸的"));

    // worker 还活着，后面的任务照跑。
    assert_eq!(pool.submit(|| "后面的任务").join().unwrap(), "后面的任务");
    assert_eq!(pool.worker_count(), 2);
}

#[test]
fn join_timeout_gives_up_without_cancelling_the_task() {
    let pool = ThreadPool::new(1);
    let (gate_tx, gate_rx) = mpsc::channel::<()>();
    let (ran_tx, ran_rx) = mpsc::channel::<()>();
    let handle = pool.submit(move || {
        gate_rx.recv().unwrap();
        ran_tx.send(()).unwrap();
    });

    let waited = Duration::from_millis(50);
    let timeout = handle.join_timeout(waited).unwrap_err();
    assert_eq!(timeout, Timeout { waited });

    // 超时不是取消：任务还在等 gate，放行之后它会把剩下的活干完。
    gate_tx.send(()).unwrap();
    ran_rx.recv_timeout(LONG).expect("任务放行后应当跑完");
    pool.shutdown();
}

#[test]
fn one_worker_runs_tasks_in_submission_order() {
    let pool = ThreadPool::new(1);
    let (order_tx, order_rx) = mpsc::channel();
    for n in 0..10 {
        let order = order_tx.clone();
        pool.spawn(move || order.send(n).unwrap());
    }
    drop(order_tx);
    let order: Vec<i32> = order_rx.iter().collect();
    assert_eq!(order, (0..10).collect::<Vec<_>>());
}

#[test]
fn dropping_the_pool_finishes_queued_tasks() {
    const JOBS: usize = 20;
    let pool = ThreadPool::new(1);
    let (done_tx, done_rx) = mpsc::channel();
    for n in 0..JOBS {
        let done = done_tx.clone();
        pool.spawn(move || {
            done.send(n).unwrap();
        });
    }
    drop(done_tx);
    drop(pool); // 关队列 + join worker：排队的活先跑完，再收工。
    let finished: Vec<usize> = done_rx.iter().collect();
    assert_eq!(finished, (0..JOBS).collect::<Vec<_>>());
}

#[test]
fn results_arrive_even_after_the_pool_is_dropped() {
    let pool = ThreadPool::new(2);
    let handle = pool.submit(|| "跑完了");
    drop(pool);
    assert_eq!(handle.join().unwrap(), "跑完了");
}

#[test]
fn a_rendezvous_queue_blocks_the_submitter_until_a_worker_is_free() {
    let pool = ThreadPool::bounded(1, 0);
    let (gate_tx, gate_rx) = mpsc::channel::<()>();
    let (started_tx, started_rx) = mpsc::channel::<()>();

    // 唯一的 worker 被这个任务占住，直到 gate 放行。
    pool.spawn(move || {
        started_tx.send(()).unwrap();
        gate_rx.recv().unwrap();
    });
    started_rx.recv_timeout(LONG).expect("第一个任务没有跑起来");

    let (submitted_tx, submitted_rx) = mpsc::channel::<()>();
    let (done_tx, done_rx) = mpsc::channel::<()>();
    {
        let pool = &pool;
        thread::scope(|s| {
            s.spawn(move || {
                // 容量 0：没有空闲 worker 来取，这一次提交会阻塞在这里。
                pool.spawn(move || {
                    done_tx.send(()).unwrap();
                });
                submitted_tx.send(()).unwrap();
            });
            // worker 还在忙，所以这次提交没有、也不该完成。
            assert!(submitted_rx.recv_timeout(SHORT).is_err());

            gate_tx.send(()).unwrap(); // 放行：worker 空出来，提交随即完成。
        });
    }
    done_rx.recv_timeout(LONG).expect("第二个任务终究要跑");
    assert!(submitted_rx.recv_timeout(LONG).is_ok());
}

#[test]
fn a_handle_can_be_joined_from_another_thread() {
    let pool = ThreadPool::new(1);
    let handle = pool.submit(|| 21 * 2);
    let value = thread::scope(|s| s.spawn(move || handle.join().unwrap()).join().unwrap());
    assert_eq!(value, 42);
}

#[test]
fn a_bounded_queue_finishes_tasks_that_were_already_submitted() {
    let pool = ThreadPool::bounded(2, 4);
    let handles: Vec<_> = (0..10).map(|n| (n, pool.submit(move || n * 3))).collect();
    drop(pool); // 缓冲有限，但已经排进去的活照样跑完。
    for (n, handle) in handles {
        assert_eq!(handle.join().unwrap(), n * 3);
    }
}

#[test]
fn a_full_bounded_queue_blocks_the_submitter_until_a_worker_is_free() {
    let pool = ThreadPool::bounded(1, 1);
    let (gate_tx, gate_rx) = mpsc::channel::<()>();
    let (started_tx, started_rx) = mpsc::channel::<()>();

    // 唯一的 worker 被这个任务占住，缓冲空着。
    pool.spawn(move || {
        started_tx.send(()).unwrap();
        gate_rx.recv().unwrap();
    });
    started_rx.recv_timeout(LONG).expect("第一个任务没有跑起来");

    // 缓冲只有 1 格：这一次提交立刻成功，把格子占满。
    let (queued_tx, queued_rx) = mpsc::channel::<()>();
    pool.spawn(move || {
        queued_tx.send(()).unwrap();
    });

    // 再提交就得等：worker 在忙，缓冲也满了。
    let (submitted_tx, submitted_rx) = mpsc::channel::<()>();
    let pool = &pool;
    thread::scope(|s| {
        s.spawn(move || {
            pool.spawn(|| {});
            submitted_tx.send(()).unwrap();
        });
        assert!(
            submitted_rx.recv_timeout(SHORT).is_err(),
            "缓冲满了，提交不该完成"
        );

        gate_tx.send(()).unwrap(); // 放行：worker 跑完手上的活，格子空出来。
    });
    assert!(submitted_rx.recv_timeout(LONG).is_ok());
    queued_rx.recv_timeout(LONG).expect("缓冲里的任务应当跑完");
}

#[test]
fn several_threads_can_submit_to_a_small_bounded_queue() {
    let pool = ThreadPool::bounded(3, 2);
    let total: i32 = thread::scope(|s| {
        let pool = &pool;
        let submitters: Vec<_> = (0..3)
            .map(|batch| {
                s.spawn(move || {
                    (0..20)
                        .map(|i| pool.submit(move || batch * 100 + i))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        submitters
            .into_iter()
            .flat_map(|handle| handle.join().unwrap())
            .map(|handle| handle.join().unwrap())
            .sum()
    });
    let expected: i32 = (0..3).flat_map(|b| (0..20).map(move |i| b * 100 + i)).sum();
    assert_eq!(total, expected);
}
