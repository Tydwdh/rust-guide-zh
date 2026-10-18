//! `MiniArc` / `Mutex` 的行为测试：计数、析构时机、跨线程累加。

use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;

use ch34_snippets::mini_arc::MiniArc;
use ch34_snippets::mini_mutex::Mutex;

static DROPS: AtomicUsize = AtomicUsize::new(0);

struct Counted;

impl Drop for Counted {
    fn drop(&mut self) {
        DROPS.fetch_add(1, Ordering::Relaxed);
    }
}

#[test]
fn clone_bumps_the_count_and_drop_lowers_it() {
    let a = MiniArc::new(String::from("hi"));
    assert_eq!(a.strong_count(), 1);

    let b = MiniArc::clone(&a);
    let c = a.clone();
    assert_eq!(a.strong_count(), 3);
    assert_eq!(&*a, "hi");
    assert!(MiniArc::ptr_eq(&a, &b));

    drop(b);
    assert_eq!(a.strong_count(), 2);
    drop(c);
    assert_eq!(a.strong_count(), 1);
}

#[test]
fn data_is_dropped_exactly_once_when_the_last_arc_goes() {
    DROPS.store(0, Ordering::Relaxed);
    let a = MiniArc::new(Counted);
    let b = a.clone();
    drop(a);
    assert_eq!(DROPS.load(Ordering::Relaxed), 0, "还有一份强引用");
    drop(b);
    assert_eq!(DROPS.load(Ordering::Relaxed), 1, "最后一份触发析构");
}

#[test]
fn lock_is_released_when_the_guard_drops() {
    let m = Mutex::new(5);
    {
        let mut g = m.lock();
        *g += 1;
    }
    // 能再次抢到锁，说明守卫的 Drop 真的解锁了。
    assert_eq!(*m.lock(), 6);
}

#[test]
fn threads_share_one_counter_through_arc_mutex() {
    const THREADS: usize = 4;
    const PER_THREAD: usize = 20_000;

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

    assert_eq!(counter.strong_count(), 1, "线程都退出了，只剩本地这一份");
    assert_eq!(*counter.lock(), THREADS * PER_THREAD);
}

#[test]
fn guard_is_exclusive_while_held() {
    // 一个线程握着锁不放，另一个线程必须等：拿不到锁就不会有第二个守卫。
    let m = MiniArc::new(Mutex::new(Vec::new()));
    let writer = m.clone();
    let handle = thread::spawn(move || {
        let mut g = writer.lock();
        g.push(1);
        thread::sleep(Duration::from_millis(50));
        g.push(2);
    });

    // 主线程不断尝试；一旦拿到锁，看到的一定是两个元素都在（说明写线程已释放锁）。
    loop {
        let g = m.lock();
        if g.len() == 2 {
            assert_eq!(*g, [1, 2]);
            break;
        }
        drop(g);
    }
    handle.join().unwrap();
}

#[test]
fn mut_borrow_through_guard_writes_are_visible_after_a_sync_point() {
    struct Snapshot(usize);

    impl Drop for Snapshot {
        fn drop(&mut self) {
            DROPS.store(self.0, Ordering::Relaxed);
        }
    }

    DROPS.store(0, Ordering::Relaxed);
    let shared = MiniArc::new(Mutex::new(Snapshot(0)));
    let writer = shared.clone();
    thread::spawn(move || {
        writer.lock().0 = 7;
    })
    .join()
    .unwrap();

    drop(shared); // 最后一份强引用在这里析构
    assert_eq!(DROPS.load(Ordering::Relaxed), 7, "析构看到了另一个线程写入的值");
}
