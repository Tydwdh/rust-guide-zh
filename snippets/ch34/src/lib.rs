//! 第 34 章「手写集合与智能指针」的完整代码。
//!
//! 三个模块对应正文三块内容：
//!
//! - [`mini_vec`]：RawVec 式增长、`Drain` 守卫、ZST 惯例；
//! - [`mini_arc`]：引用计数与数据同块分配，`Release`/`Acquire` 配对；
//! - [`mini_mutex`]：`UnsafeCell` 装数据 + `AtomicBool` 自旋锁。
//!
//! 与第 23 章同一套纪律：安全 API 是外壳，每个 `unsafe` 块配一条 `// SAFETY:`。
//! 本 crate 没有**公开的** `unsafe fn`：唯一一个是私有的 `mini_vec::Drain::new`，
//! 它带 `# Safety` 段，唯一的调用点写了 `// SAFETY:`。
#![deny(unsafe_op_in_unsafe_fn)]

pub mod mini_arc;
pub mod mini_mutex;
pub mod mini_vec;
