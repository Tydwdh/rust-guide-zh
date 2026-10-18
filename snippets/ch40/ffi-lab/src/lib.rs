//! cdylib 边界示例：跨边界的每一个类型都必须是 C 认得的东西。
//!
//! 规则只有两条：值类型加 `#[repr(C)]`，复杂状态藏在不透明句柄后面。

/// 内部状态。它的布局是 Rust 私有的，C 侧永远看不到、也不需要看到。
pub struct Counter {
    hits: u64,
}

/// 建一个计数器，返回不透明句柄。
#[unsafe(no_mangle)]
pub extern "C" fn counter_new() -> *mut Counter {
    Box::into_raw(Box::new(Counter { hits: 0 }))
}

/// 累加 `n`，返回加完后的总数。
///
/// # Safety
///
/// `handle` 必须是 `counter_new` 返回、且尚未交给 `counter_free` 的指针。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn counter_add(handle: *mut Counter, n: u64) -> u64 {
    // SAFETY: 由调用方契约保证 handle 指向一个存活的 Counter，且没有别名。
    let counter = unsafe { &mut *handle };
    counter.hits += n;
    counter.hits
}

/// 释放计数器；传空指针是安全的空操作。
///
/// # Safety
///
/// `handle` 必须来自 `counter_new`，且此后不得再被任何一侧使用。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn counter_free(handle: *mut Counter) {
    if !handle.is_null() {
        // SAFETY: 由调用方契约保证该指针是 Box::into_raw 的产物，且只释放一次。
        drop(unsafe { Box::from_raw(handle) });
    }
}

impl Counter {
    /// Rust 侧自己的访问器：它不在 C 的边界上，走的是「Rust ABI」——符号名会被改名并带上哈希。
    pub fn hits(&self) -> u64 {
        self.hits
    }
}

/// 按值跨边界的类型：字段顺序、对齐、大小都按 C 的规则来。
#[repr(C)]
pub struct Summary {
    pub sum: u64,
    pub count: u64,
}

/// 求和并计数，按值返回 `#[repr(C)]` 结构体。
#[unsafe(no_mangle)]
pub extern "C" fn summarize(values: *const u32, len: usize) -> Summary {
    let mut sum = 0u64;
    for i in 0..len {
        // SAFETY: 调用方保证 values 指向 len 个连续、可读的 u32。
        sum += u64::from(unsafe { *values.add(i) });
    }
    Summary {
        sum,
        count: len as u64,
    }
}
