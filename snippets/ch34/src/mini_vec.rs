//! mini `Vec`：RawVec 式增长、`Drain` 守卫、ZST 惯例。
//!
//! unsafe 契约的写法与第 23 章一致：安全 API 是外壳，每个 `unsafe` 块配一条
//! `// SAFETY:`，说明它凭什么成立。`MiniVec<T>` 自己的不变量只有三条：
//!
//! 1. `buf.ptr` 指向一块按 `Layout::array::<T>(buf.cap)` 分配的缓冲区（`cap == 0` 时是
//!    `NonNull::dangling()`，不指向任何分配）；
//! 2. 前 `len` 个元素已初始化，第 `len` 个及其后是未初始化；
//! 3. `len <= buf.cap`。

use std::alloc::{self, Layout};
use std::marker::PhantomData;
use std::mem;
use std::ops::{Deref, DerefMut};
use std::ptr::{self, NonNull};
use std::slice;

// ANCHOR: rawvec
/// 「一块分配 + 容量」这一半。`ptr` 与 `cap` 成对出现：谁改了缓冲区，谁就得同时改容量。
pub struct RawVec<T> {
    ptr: NonNull<T>,
    cap: usize,
}

impl<T> RawVec<T> {
    fn new() -> Self {
        // ZST 不分配内存：容量直接给 `usize::MAX`，指针用 `NonNull::dangling()`。
        // 它一个值兼两份差事——既表示「还没分配」，也表示「零大小分配」（Nomicon 的惯例）。
        let cap = if mem::size_of::<T>() == 0 { usize::MAX } else { 0 };
        RawVec {
            ptr: NonNull::dangling(),
            cap,
        }
    }

    /// 扩容：容量翻倍起步（`0 -> 1 -> 2 -> 4 -> ...`），顺便把 `isize::MAX` 那条线守住。
    fn grow(&mut self) {
        // ZST 的容量一上来就是 usize::MAX，能走到这里说明真的溢出了。
        if mem::size_of::<T>() == 0 {
            panic!("capacity overflow");
        }
        // `cap <= isize::MAX` 由本函数末尾的检查维持，所以翻倍不会溢出 usize。
        let new_cap = self
            .cap
            .checked_mul(2)
            .unwrap_or_else(|| panic!("capacity overflow"))
            .max(1);
        let new_layout = match Layout::array::<T>(new_cap) {
            // `Layout::array` 的契约已经保证总大小不超过 isize::MAX，所以这条判断按契约
            // 不可达；留着它是为了让「分配不超过 isize::MAX」这条不变量在本地看得见。
            Ok(layout) if layout.size() <= isize::MAX as usize => layout,
            _ => panic!("capacity overflow"),
        };

        let new_ptr = if self.cap == 0 {
            // SAFETY: new_layout 的 size 至少是 size_of::<T>()，且 T 不是 ZST，满足 alloc 的前提。
            unsafe { alloc::alloc(new_layout) }
        } else {
            let old_layout = Layout::array::<T>(self.cap).unwrap();
            // SAFETY: 现有缓冲区就是按 old_layout 分配的（grow 是唯一改动 ptr/cap 的地方），
            // 新大小不超过 isize::MAX 已经检查过。
            unsafe {
                alloc::realloc(self.ptr.as_ptr() as *mut u8, old_layout, new_layout.size())
            }
        };

        // 分配失败时 new_ptr 是空指针，交给 handle_alloc_error 按平台规则收场。
        self.ptr = match NonNull::new(new_ptr as *mut T) {
            Some(p) => p,
            None => alloc::handle_alloc_error(new_layout),
        };
        self.cap = new_cap;
    }
}
// ANCHOR_END: rawvec

impl<T> Drop for RawVec<T> {
    fn drop(&mut self) {
        // ZST 从没分配过（cap 是 usize::MAX、ptr 是 dangling），不能拿去 dealloc。
        if self.cap != 0 && mem::size_of::<T>() != 0 {
            let layout = Layout::array::<T>(self.cap).unwrap();
            // SAFETY: cap != 0 且 T 不是 ZST，说明这块缓冲区确实是 grow 里分配出来的。
            unsafe {
                alloc::dealloc(self.ptr.as_ptr() as *mut u8, layout);
            }
        }
    }
}

// SAFETY: RawVec 独占它持有的元素，所以 T: Send 时它也能送过线程边界。
unsafe impl<T: Send> Send for RawVec<T> {}
// SAFETY: 共享借用 RawVec<T> 拿不到任何元素（访问都走 &mut self），所以 T: Sync 时它也有 Sync。
unsafe impl<T: Sync> Sync for RawVec<T> {}

/// mini 版 `Vec<T>`：`RawVec` 加一个长度。
pub struct MiniVec<T> {
    buf: RawVec<T>,
    len: usize,
}

impl<T> MiniVec<T> {
    pub fn new() -> Self {
        MiniVec {
            buf: RawVec::new(),
            len: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn capacity(&self) -> usize {
        self.buf.cap
    }

    // ANCHOR: push-pop
    pub fn push(&mut self, elem: T) {
        if self.len == self.buf.cap {
            self.buf.grow();
        }
        // SAFETY: len < cap（上面刚保证）。第 len 个槽位要么从没写过，要么被 pop 读过，
        // 两种情况下都用 ptr::write 直接覆写：它不读旧值，不会把未初始化内存当成 T。
        unsafe {
            ptr::write(self.buf.ptr.as_ptr().add(self.len), elem);
        }
        self.len += 1;
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        }
        self.len -= 1;
        // SAFETY: pop 之前第 len 个槽位是已初始化的。读出来之后它逻辑上回到未初始化，
        // 而 len 已经减 1，不会再有人读它。
        Some(unsafe { ptr::read(self.buf.ptr.as_ptr().add(self.len)) })
    }
    // ANCHOR_END: push-pop

    pub fn as_slice(&self) -> &[T] {
        // SAFETY: 不变量 2：[0, len) 一定是已初始化的。切片借用 &self，生命周期由签名给出。
        unsafe { slice::from_raw_parts(self.buf.ptr.as_ptr(), self.len) }
    }

    pub fn as_mut_slice(&mut self) -> &mut [T] {
        // SAFETY: 同 as_slice；&mut self 保证这段内存没有别的借用。
        unsafe { slice::from_raw_parts_mut(self.buf.ptr.as_ptr(), self.len) }
    }

    pub fn insert(&mut self, index: usize, elem: T) {
        assert!(index <= self.len, "insertion index (is {index}) should be <= len");
        if self.len == self.buf.cap {
            self.buf.grow();
        }
        // SAFETY: index <= len <= cap。先把它后面的元素整体后移一格（源与目标区间重叠，
        // 必须用 memmove 语义的 ptr::copy，不能是 ptr::copy_nonoverlapping），再写入空出来的槽位。
        unsafe {
            let p = self.buf.ptr.as_ptr().add(index);
            ptr::copy(p, p.add(1), self.len - index);
            ptr::write(p, elem);
        }
        self.len += 1;
    }

    pub fn remove(&mut self, index: usize) -> T {
        assert!(index < self.len, "removal index (is {index}) should be < len");
        // SAFETY: index < len，所以 p 落在已初始化区间内。读出元素后把它后面的元素前移一格补上
        // 缺口（同样重叠，用 ptr::copy），len 减一。整个过程不出现重复的已初始化槽位。
        unsafe {
            let p = self.buf.ptr.as_ptr().add(index);
            let result = ptr::read(p);
            ptr::copy(p.add(1), p, self.len - index - 1);
            self.len -= 1;
            result
        }
    }

    // ANCHOR: drain
    /// 取走全部元素，但保留缓冲区。返回的守卫借用了 `&mut self`，
    /// 于是「取元素」与「继续用 Vec」在借用检查器眼里不可能同时发生。
    pub fn drain(&mut self) -> Drain<'_, T> {
        // 先把 len 归零，这一步是整个设计的关键：它保证「Drain 被 mem::forget 掉」
        // 最坏也只是泄漏（元素永远不会被 drop 第二次），而不是 UB。
        let len = self.len;
        self.len = 0;
        // SAFETY: buf.ptr 指向 cap >= len 个元素的缓冲区，且前 len 个已初始化（不变量 1、2）；
        // len 刚被设成 0，MiniVec 自己不会再碰这些元素。
        unsafe { Drain::new(self.buf.ptr.as_ptr(), len) }
    }
    // ANCHOR_END: drain
}

impl<T> Drop for MiniVec<T> {
    fn drop(&mut self) {
        // 守卫在这里第二次派上用场：`drain()` 把 len 归零并交出一个 Drain，
        // 它的 Drop 会把剩下的元素逐个读出来 drop；缓冲区随 RawVec 一起释放。
        // 这样元素按从前到后的顺序析构，和 std 的 Vec 一致。
        let _ = self.drain();
    }
}

impl<T> Deref for MiniVec<T> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        self.as_slice()
    }
}

impl<T> DerefMut for MiniVec<T> {
    fn deref_mut(&mut self) -> &mut [T] {
        self.as_mut_slice()
    }
}

impl<T> Default for MiniVec<T> {
    fn default() -> Self {
        Self::new()
    }
}

// ANCHOR: drain-guard
/// `drain()` 的守卫：拿不到这个类型，就碰不到被 `drain()` 借走的那段 `Vec`。
pub struct Drain<'a, T> {
    // 语义上这就是 `&'a mut MiniVec<T>`，所以用 PhantomData 把它写下来。
    vec: PhantomData<&'a mut MiniVec<T>>,
    start: *const T,
    end: *const T,
}

impl<'a, T> Drain<'a, T> {
    /// 从裸指针构造。裸指针没有生命周期，所以这个构造函数自己不安全 ——
    /// 「这块内存归我」这件事只能由调用方担保（见 `# Safety`）。
    ///
    /// # Safety
    ///
    /// 调用者必须保证 `ptr` 指向一块连续、长度至少 `len`、前 `len` 个元素都已初始化的
    /// `T` 缓冲区；并且在返回的 `Drain` 被 drop 之前，这段内存不会被别的路径访问或释放。
    unsafe fn new(ptr: *mut T, len: usize) -> Self {
        let start = ptr as *const T;
        // ZST 上 `add` 是空操作，就用整数搬家当计数器（Nomicon 的写法）。
        let end = if mem::size_of::<T>() == 0 {
            (start as usize + len) as *const T
        } else {
            // SAFETY: 调用方保证 `ptr` 指向至少 len 个元素，start.add(len) 不出界。
            unsafe { start.add(len) }
        };
        Drain {
            vec: PhantomData,
            start,
            end,
        }
    }
}

impl<T> Iterator for Drain<'_, T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        if self.start == self.end {
            return None;
        }
        // SAFETY: start != end，说明 [start, end) 里还有已初始化的元素。
        unsafe {
            // ANCHOR: zst-ptr
            if mem::size_of::<T>() == 0 {
                // ZST：把地址当整数加一。`start.add(1)` 在这里是空操作，计数器会永远追不上终点。
                self.start = (self.start as usize + 1) as *const T;
                Some(ptr::read(NonNull::<T>::dangling().as_ptr()))
            } else {
                let old = self.start;
                self.start = old.add(1);
                Some(ptr::read(old))
            }
            // ANCHOR_END: zst-ptr
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let elem = mem::size_of::<T>();
        let len = (self.end as usize - self.start as usize) / if elem == 0 { 1 } else { elem };
        (len, Some(len))
    }
}

impl<T> Drop for Drain<'_, T> {
    fn drop(&mut self) {
        // 守卫的收尾：把没被取走的元素读出来丢掉。
        // `&mut *self` 走的是上面的 Iterator，等价于 `while self.next().is_some() {}`。
        for _ in &mut *self {}
    }
}
// ANCHOR_END: drain-guard
