//! 控制事件：把 `SetConsoleCtrlHandler` 那条回调收成一个安全 API。
//!
//! 这一层要处理三件麻烦事：回调没有「上下文参数」（只能走全局槽位）、
//! 回调在系统新起的线程上跑（闭包得 `Send`）、以及 panic **绝不能**穿过
//! `extern "system"` 边界（越过去就是进程 abort）。
#![forbid(unsafe_code)]

use std::fmt;
use std::panic::catch_unwind;
use std::sync::Mutex;

use crate::error::{ProbeError, Result};
use crate::platform;

/// C 侧的回调签名：`BOOL (*)(DWORD)`，Windows 头文件里叫 `PHANDLER_ROUTINE`。
///
/// 它不是 `unsafe fn`：这个函数指针只交给系统去调，我们自己不调它，
/// 也就没有「调用前要满足什么」的契约要写。
pub(crate) type HandlerRoutine = extern "system" fn(u32) -> i32;

/// 处理器闭包的类型：`Box<dyn … + Send>` 这一长串值得一个名字。
type Handler = Box<dyn FnMut(CtrlEvent) -> bool + Send>;

/// 用户闭包存放处。整进程只有一个槽位：系统送来事件时，回调得有个地方找它。
static HANDLER: Mutex<Option<Handler>> = Mutex::new(None);

/// 系统送来的控制事件（C 里的 `DWORD dwCtrlType`）。
///
/// 刻意**没有**做成 `#[repr(u32)]` 枚举：系统可能送来看不懂的码，而
/// 「把任意整数变成枚举」在 Rust 里是 UB（枚举的每个值都必须有效）。
/// 这里只存原始数字，给已知值取个名字。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct CtrlEvent(u32);

impl CtrlEvent {
    /// 按下 Ctrl+C（或别的进程用 `GenerateConsoleCtrlEvent` 发来）。
    pub const CTRL_C: Self = Self(0);
    /// 按下 Ctrl+Break。这个组合键一定会作为信号送达，改不掉。
    pub const CTRL_BREAK: Self = Self(1);
    /// 控制台窗口被关闭。
    pub const CTRL_CLOSE: Self = Self(2);
    /// 用户注销 —— 只有服务进程收得到。
    pub const CTRL_LOGOFF: Self = Self(5);
    /// 系统关机 —— 只有服务进程收得到。
    pub const CTRL_SHUTDOWN: Self = Self(6);

    /// 用系统给的原始码构造一个事件；不认识的码原样保留。
    pub const fn from_raw(raw: u32) -> Self {
        Self(raw)
    }

    /// 原始的 `dwCtrlType` 码。
    pub const fn raw(self) -> u32 {
        self.0
    }
}

impl fmt::Display for CtrlEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self.0 {
            0 => "Ctrl+C",
            1 => "Ctrl+Break",
            2 => "控制台被关闭",
            5 => "用户注销",
            6 => "系统关机",
            unknown => return write!(f, "未知控制事件（码 {unknown}）"),
        };
        f.write_str(name)
    }
}

/// 处理器句柄：只要它还活着，处理器就注册着；一 drop 就注销。
#[must_use = "句柄一被丢掉，处理器就注销了"]
pub struct CtrlGuard {
    /// 现在没有状态要存；但它必须是「不可自己构造」的 —— 唯一的构造点在下面。
    _private: (),
}

impl fmt::Debug for CtrlGuard {
    // 句柄里没有能看的东西，所以只印名字（公开类型实现 `Debug` 是惯例，
    // 但没必要为它编一份假状态）。
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("CtrlGuard")
    }
}

impl Drop for CtrlGuard {
    fn drop(&mut self) {
        // 注销必须传**当初注册的那个函数指针**（传 NULL 只改「忽略 Ctrl+C」这个
        // 进程属性，处理器会一直挂在系统里）。Drop 里不能 panic，失败也没有补救
        // 手段，只能忽略返回值。
        let _ = platform::set_console_ctrl_handler(trampoline, false);

        // 顺序要紧：先让系统摘掉处理器，再清空槽位。反过来会留一个窗口 ——
        // 那期间来的事件看到空槽，回报「没处理」，默认处理器就把进程结束了。
        // 锁中毒也要把数据拿回来，不能 unwrap。
        let mut slot = HANDLER
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *slot = None;
    }
}

/// 注册一个控制事件处理器。
///
/// 系统送来 Ctrl+C、Ctrl+Break、关窗、注销、关机时，`handler` 会在系统新起的
/// 线程上被调用；返回 `true` 表示「这个事件我处理了，别再叫别人」。
///
/// 返回的 [`CtrlGuard`] 一 drop，处理器就注销。
///
/// # 错误
///
/// - [`ProbeError::HandlerInUse`]：本进程已经有一个处理器了（本库只留一个槽位）；
/// - [`ProbeError::Os`]：`SetConsoleCtrlHandler` 自己失败了；
/// - [`ProbeError::Unsupported`]：非 Windows 平台。
///
/// # 闭包的两条约束
///
/// - 它在系统的线程上跑，所以必须是 `Send + 'static`；
/// - **别在处理器里 drop 这个句柄**（也别在处理器里再调本函数）：处理器运行
///   期间槽位的锁是拿在手里的，那会自己等自己。
///
/// ```
/// use sys_probe::{watch_ctrl_events, CtrlEvent};
///
/// // 这个例子在两种平台上都能跑：Windows 上会真的注册，别处会拿到 Unsupported。
/// match watch_ctrl_events(|event| {
///     println!("收到 {event}");
///     event == CtrlEvent::CTRL_C // 只认 Ctrl+C，其余交给下一个处理器
/// }) {
///     Ok(_guard) => println!("处理器已注册"),
///     Err(err) => println!("没注册上：{err}"),
/// }
/// ```
pub fn watch_ctrl_events<F>(handler: F) -> Result<CtrlGuard>
where
    F: FnMut(CtrlEvent) -> bool + Send + 'static,
{
    let mut slot = HANDLER
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if slot.is_some() {
        return Err(ProbeError::HandlerInUse);
    }

    // 先向系统登记，再放闭包：系统那边才是权威，登记失败时槽位还是干净的，
    // 不需要回滚。整段都在同一把锁里 —— 注册期间真有事件到达，回调会先阻塞在
    // 这把锁上，等闭包装好才拿到它，不存在「登记了却没人接」的空档。
    platform::set_console_ctrl_handler(trampoline, true)?;
    *slot = Some(Box::new(handler));

    Ok(CtrlGuard { _private: () })
}

/// 真正交给系统的回调。
///
/// 它不是 `unsafe fn`：签名里没有指针、没有前置条件，调用它不会破坏任何东西。
/// 它要做的事只有一件 —— **别让 panic 出去**。
extern "system" fn trampoline(raw: u32) -> i32 {
    let event = CtrlEvent::from_raw(raw);

    // panic 一旦穿过 `extern "system"` 边界，进程直接 abort（第 26 章）：
    // 系统那边没有「栈展开」这件事，接不住。所以必须就地兜住，
    // 再把「没处理」如实报回去。
    let handled = catch_unwind(|| {
        let mut slot = HANDLER
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        match slot.as_mut() {
            Some(handler) => handler(event),
            None => false,
        }
    })
    .unwrap_or(false);

    if handled { 1 } else { 0 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex as StdMutex, MutexGuard};

    /// 槽位只有一个，碰到它的测试必须串行。
    static TESTS: StdMutex<()> = StdMutex::new(());

    fn serialize() -> MutexGuard<'static, ()> {
        TESTS
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// 测试里直接往槽里塞闭包：不碰操作系统，非 Windows 也能跑回调这一层。
    fn install<F>(handler: F)
    where
        F: FnMut(CtrlEvent) -> bool + Send + 'static,
    {
        let mut slot = HANDLER
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        assert!(slot.is_none(), "上一个用例没把槽位清干净");
        *slot = Some(Box::new(handler));
    }

    /// 用例结束时清空槽位，别影响下一个。
    struct ClearSlot;

    impl Drop for ClearSlot {
        fn drop(&mut self) {
            let mut slot = HANDLER
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            *slot = None;
        }
    }

    #[test]
    fn the_handler_sees_the_raw_code_and_its_answer_is_reported() {
        let _lock = serialize();
        let _clear = ClearSlot;

        let seen = Arc::new(StdMutex::new(Vec::new()));
        let recorder = Arc::clone(&seen);
        install(move |event| {
            recorder.lock().unwrap().push(event.raw());
            event == CtrlEvent::CTRL_C
        });

        assert_eq!(
            trampoline(CtrlEvent::CTRL_C.raw()),
            1,
            "处理器认了，就该回 TRUE"
        );
        assert_eq!(
            trampoline(CtrlEvent::CTRL_BREAK.raw()),
            0,
            "处理器不认，就该回 FALSE"
        );
        assert_eq!(*seen.lock().unwrap(), vec![0, 1]);
    }

    #[test]
    fn with_no_handler_the_event_is_reported_as_unhandled() {
        let _lock = serialize();
        let _clear = ClearSlot;

        assert_eq!(trampoline(CtrlEvent::CTRL_C.raw()), 0);
    }

    #[test]
    fn a_panicking_handler_is_contained_and_the_slot_still_works() {
        let _lock = serialize();
        let _clear = ClearSlot;

        let panicked = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&panicked);
        install(move |_| {
            flag.store(true, Ordering::SeqCst);
            panic!("处理器里的 panic");
        });

        // catch_unwind 接得住 panic，但 panic hook 照样会往 stderr 打一行 ——
        // 测试期间把它静音，免得输出被搅乱（用例自己会检查 panic 确实发生过）。
        let hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let first = trampoline(CtrlEvent::CTRL_C.raw());
        let second = trampoline(CtrlEvent::CTRL_C.raw());
        std::panic::set_hook(hook);

        assert!(
            panicked.load(Ordering::SeqCst),
            "处理器应该真的跑到了 panic"
        );
        assert_eq!(first, 0, "panic 之后按「没处理」回报");
        assert_eq!(second, 0, "第二次调用不会被中毒的锁挡住");

        // 槽位的锁在第一次 panic 时就中毒了。我们要能照样换一个新处理器：
        // 这就是 `unwrap_or_else(into_inner)` 的作用。
        drop(_clear);
        let _clear = ClearSlot;
        install(|_| true);
        assert_eq!(
            trampoline(CtrlEvent::CTRL_C.raw()),
            1,
            "中毒之后换的处理器应该能工作"
        );
    }

    #[test]
    fn unknown_codes_are_kept_verbatim() {
        assert_eq!(CtrlEvent::from_raw(7).raw(), 7);
        assert_eq!(CtrlEvent::from_raw(7).to_string(), "未知控制事件（码 7）");
        assert_eq!(CtrlEvent::CTRL_SHUTDOWN.to_string(), "系统关机");
    }
}
