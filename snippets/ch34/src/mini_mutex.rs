//! 简化版 `Mutex`：`UnsafeCell` 装数据 + 一个 `AtomicBool` 当锁。
//!
//! 与 std 的差别：自旋而不是阻塞（stdlib 在竞争时交由操作系统挂起线程），没有锁中毒，
//! 也没处理平台上「解锁必须同线程」这类细节。

use std::cell::UnsafeCell;
use std::hint::spin_loop;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicBool, Ordering};

// ANCHOR: guard
pub struct Mutex<T> {
    locked: AtomicBool,
    // UnsafeCell 是唯一被允许「通过共享引用改数据」的容器；
    // 它是 !Sync，所以下面必须为 Mutex 手工给出 Send/Sync。
    data: UnsafeCell<T>,
}

pub struct MutexGuard<'a, T> {
    lock: &'a Mutex<T>,
}

impl<T> Mutex<T> {
    pub fn new(data: T) -> Self {
        Mutex {
            locked: AtomicBool::new(false),
            data: UnsafeCell::new(data),
        }
    }

    pub fn lock(&self) -> MutexGuard<'_, T> {
        // 抢锁：false -> true。Acquire 保证抢到锁之后，能看见上一个解锁者写进 data 的值。
        while self
            .locked
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            spin_loop();
        }
        MutexGuard { lock: self }
    }
}

impl<T> Deref for MutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        // SAFETY: 持有守卫意味着本线程拿到了锁，别的线程拿不到同一把锁的守卫，
        // 也就没人会经由 &mut 访问这块 data。共享引用的生命周期跟着守卫走。
        unsafe { &*self.lock.data.get() }
    }
}

impl<T> DerefMut for MutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        // SAFETY: 同上；这里还多了一层 &mut self——同一时刻只会有一个守卫能取出 &mut T。
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T> Drop for MutexGuard<'_, T> {
    fn drop(&mut self) {
        // Release 解锁：解锁前对 data 的写，要让下一个抢到锁的线程看得见。
        self.lock.locked.store(false, Ordering::Release);
    }
}
// SAFETY: 把 Mutex<T> 送过线程边界等于把 T 也送过去，所以需要 T: Send；
// 「&Mutex<T> 可共享」意味着两个线程能交替拿到 &mut T，所以 Sync 同样要求 T: Send。
unsafe impl<T: Send> Send for Mutex<T> {}
unsafe impl<T: Send> Sync for Mutex<T> {}

// SAFETY: 守卫的 Deref 会发出 &T，所以「&MutexGuard 可跨线程共享」只有 T: Sync 时才成立。
// 这一条必须显式写：不写的话，守卫的 Sync 会从字段 `&'a Mutex<T>` 自动推导出来，
// 边界退化成 T: Send —— 于是 &MutexGuard<Cell<u32>> 能跨线程共享，两个线程各自拿到
// &Cell<u32> 去写，直接构成数据竞争（Cell 是 Send 但 !Sync）。
unsafe impl<T: Sync> Sync for MutexGuard<'_, T> {}
// ANCHOR_END: guard
