//! Windows 实现：整个 crate 唯一和操作系统直接说话的地方。
//!
//! 三条规矩：只声明真正要用到的函数；每个 `unsafe` 块都配一行 `// SAFETY:`
//! 说明（写清这个调用为什么合法）；从本文件出去的每个函数都已经是安全的。

use std::mem::MaybeUninit;

use crate::ctrl::HandlerRoutine;
use crate::error::{ProbeError, Result};
use crate::file_time::FileTime;

// Windows 头文件里的类型 → 这里的类型：
//
//   BOOL       -> i32        （0 = FALSE，非 0 = TRUE）
//   DWORD      -> u32
//   ULONGLONG  -> u64
//   LPFILETIME -> *mut FileTime（我们自己那个 `#[repr(C)]` 结构体）
//   PHANDLER_ROUTINE -> Option<HandlerRoutine>（可空函数指针走空指针优化）
//
// 这些函数都在 Kernel32.dll 里，所以 `#[link]` 告诉链接器去哪里找符号。
// ABI 一律写 `"system"`：它在 x86 上是 `stdcall`，在 x64 上就是 C 的约定 ——
// 这正是 Win32 API 用的那套，也是 Windows 文档里 `WINAPI` 宏的含义。
#[link(name = "kernel32")]
unsafe extern "system" {
    /// `ULONGLONG GetTickCount64()`：自系统启动以来经过的毫秒数。
    fn GetTickCount64() -> u64;

    /// `VOID GetSystemTimeAsFileTime(LPFILETIME)`：把当前 UTC 时间写进指针指向的结构体。
    fn GetSystemTimeAsFileTime(out: *mut FileTime);

    /// `BOOL SetConsoleCtrlHandler(PHANDLER_ROUTINE, BOOL)`：注册/注销控制事件处理器。
    fn SetConsoleCtrlHandler(handler: Option<HandlerRoutine>, add: i32) -> i32;

    /// `DWORD GetLastError()`：取当前线程上一次调用留下的错误码。
    fn GetLastError() -> u32;
}

/// 见 [`crate::uptime_millis`]。
pub(crate) fn get_tick_count_64() -> Result<u64> {
    // SAFETY: 没有参数、没有失败路径，Win32 文档也没有列出任何前置条件；
    // 可以在任意线程调用。
    Ok(unsafe { GetTickCount64() })
}

/// 见 [`crate::system_file_time`]。
pub(crate) fn get_system_time_as_file_time() -> Result<FileTime> {
    let mut out = MaybeUninit::<FileTime>::uninit();

    // SAFETY: `out` 是栈上一块可写、对齐到 4 字节的内存，大小恰好是 FileTime；
    // 参数是 `[out]`，函数保证把它写满（8 字节），所以读之前先初始化这块是合法的。
    unsafe { GetSystemTimeAsFileTime(out.as_mut_ptr()) };

    // SAFETY: 上一行保证已经写满；这个过程不会 panic，也就没有「写了一半就退出」的路径。
    Ok(unsafe { out.assume_init() })
}

/// 注册或注销控制事件处理器。
///
/// 参数是**必填的函数指针**，不是 `Option`：Windows 文档里 `HandlerRoutine` 传 NULL
/// 那一段说的是「让进程忽略 / 恢复正常处理 Ctrl+C」这个属性，**不是**「摘掉处理器」——
/// 注销必须传当初注册的那**同一个**指针。把可空性从这里拿掉，这个坑在签名层面就不成立
/// （原始声明里仍然是 `Option`，因为 C 的 `PHANDLER_ROUTINE` 本来就可空）。
pub(crate) fn set_console_ctrl_handler(handler: HandlerRoutine, add: bool) -> Result<()> {
    // SAFETY: 参数是我们自己实现的、签名与 PHANDLER_ROUTINE 一致的函数指针；
    // 第二个参数是 BOOL（0 / 非 0）。这个调用不碰 Rust 的内存。
    let ok = unsafe { SetConsoleCtrlHandler(Some(handler), if add { 1 } else { 0 }) };

    if ok == 0 {
        // 错误码必须紧接着失败的那次调用去取：中间任何一次 Win32 调用都可能覆盖它。
        // SAFETY: 无参数、无失败路径。
        let code = unsafe { GetLastError() };
        Err(ProbeError::os("SetConsoleCtrlHandler", code))
    } else {
        Ok(())
    }
}
