//! 简化版 `Arc`：引用计数与数据放在同一块分配里，`Release`/`Acquire` 成对。
//!
//! 与 std 的差别（都是有意的简化）：没有 `Weak`（下文用 std 的 `Arc`/`Weak` 演示弱引用），
//! 计数溢出用 `assert!` 而不是 `abort`，分配器固定为全局分配器。

use std::marker::PhantomData;
use std::ops::Deref;
use std::ptr::NonNull;
use std::sync::atomic::{self, AtomicUsize, Ordering};

/// 计数与数据放在一起：一次分配、一次释放。
struct ArcInner<T> {
    rc: AtomicUsize,
    data: T,
}

// ANCHOR: layout
pub struct MiniArc<T> {
    ptr: NonNull<ArcInner<T>>,
    // `NonNull` 解决可变性与「永不为空」；`PhantomData<ArcInner<T>>` 解决所有权与 drop check：
    // 告诉编译器「我拥有一个 ArcInner<T>」，因为最后一个 MiniArc 会把它 drop 掉。
    phantom: PhantomData<ArcInner<T>>,
}

impl<T> MiniArc<T> {
    pub fn new(data: T) -> Self {
        let boxed = Box::new(ArcInner {
            rc: AtomicUsize::new(1), // 第一个引用就是「我」
            data,
        });
        MiniArc {
            // SAFETY（对 unwrap 而言）：Box::into_raw 不可能给出空指针。
            ptr: NonNull::new(Box::into_raw(boxed)).unwrap(),
            phantom: PhantomData,
        }
    }

    /// 当前强引用数。调用者手里至少有一个 `MiniArc`，所以这块分配一定还活着，
    /// 读计数用 `Relaxed` 就够（std 的 `Arc::strong_count` 也是这个口径）。
    pub fn strong_count(&self) -> usize {
        self.inner().rc.load(Ordering::Relaxed)
    }

    pub fn ptr_eq(this: &Self, other: &Self) -> bool {
        this.ptr == other.ptr
    }

    fn inner(&self) -> &ArcInner<T> {
        // SAFETY: 只要有一个 MiniArc 活着（这里就是 &self），这块分配就没被释放过。
        unsafe { self.ptr.as_ref() }
    }
}

// SAFETY: 跨线程共享的是 &T，所以需要 T: Sync；MiniArc 自己可以被送走（从而可能带着 T 走），
// 所以还需要 T: Send。少任何一个都会让 Arc<Rc<u32>> 这类东西跨线程，直接失去安全性保证。
unsafe impl<T: Send + Sync> Send for MiniArc<T> {}
unsafe impl<T: Send + Sync> Sync for MiniArc<T> {}

impl<T> Deref for MiniArc<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.inner().data
    }
}
// ANCHOR_END: layout

// ANCHOR: clone
impl<T> Clone for MiniArc<T> {
    fn clone(&self) -> Self {
        // Relaxed 就够：克隆只改计数，不碰 data，所以不需要与任何别的操作建立先后关系。
        let old = self.inner().rc.fetch_add(1, Ordering::Relaxed);
        // mem::forget 掉一堆 MiniArc 也能让计数涨到上限，std 在这里直接 abort；
        // 这里换成 assert，行为看得见。
        assert!(old < isize::MAX as usize, "Arc counter overflow");
        MiniArc {
            ptr: self.ptr,
            phantom: PhantomData,
        }
    }
}
// ANCHOR_END: clone

// ANCHOR: drop
impl<T> Drop for MiniArc<T> {
    fn drop(&mut self) {
        // Release + 只有减到 0 的那一次才往下走：别人对 data 的写，必须在我删除它之前可见。
        if self.inner().rc.fetch_sub(1, Ordering::Release) != 1 {
            return;
        }
        // Acquire fence 与上面那次 Release 配对：拿到锁写进 Mutex 的值（第 A 个线程），
        // 在这里的析构（第 B 个线程）里一定看得见——这正是「先 happened-before，再删除」。
        atomic::fence(Ordering::Acquire);
        // SAFETY: 计数从 1 减到 0，说明再也没有别的 MiniArc 指向它了，
        // Box::from_raw 把所有权收回来交给 Box 的 Drop 释放。
        unsafe { drop(Box::from_raw(self.ptr.as_ptr())) };
    }
}
// ANCHOR_END: drop
