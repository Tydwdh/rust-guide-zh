//! 把 Windows 的系统 API 包成安全函数。
//!
//! 三个 API，三种难度：
//!
//! | 公开函数 | 封装的是 | 难在哪 |
//! |---|---|---|
//! | [`uptime_millis`] | `GetTickCount64` | 最简单的样子：无参数、无失败 |
//! | [`system_file_time`] | `GetSystemTimeAsFileTime` | 出参结构体：`#[repr(C)]` 布局与裸指针 |
//! | [`watch_ctrl_events`] | `SetConsoleCtrlHandler` | 回调：全局状态、线程、panic 边界 |
//!
//! 公开面上只有安全函数、一个 `unsafe fn`（[`FileTime::read_raw`]，契约写在它的
//! `# Safety` 段里）和一个不可自己构造的句柄。`unsafe` 只落在两个文件里：
//! `src/platform/windows.rs`（调系统 API）与 `src/file_time.rs`（那一次 `ptr.read()`）；
//! `error.rs`、`ctrl.rs`、`platform/unsupported.rs` 用 `#![forbid(unsafe_code)]` 钉死。
//! 平台的差异只收在 `src/platform.rs`（windows / not(windows) 两处判断）——
//! 别的平台有一份签名完全相同、只回 [`ProbeError::Unsupported`] 的降级实现。
//!
//! ```
//! # fn main() -> Result<(), sys_probe::ProbeError> {
//! // Windows 上给出真实值；其它平台上返回 Unsupported。
//! match sys_probe::unix_time_millis() {
//!     Ok(ms) => println!("现在是 Unix 毫秒 {ms}"),
//!     Err(err) => println!("这台机器上拿不到：{err}"),
//! }
//! # Ok(())
//! # }
//! ```
#![deny(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]

mod ctrl;
mod error;
mod file_time;
mod platform;

pub use ctrl::{CtrlEvent, CtrlGuard, watch_ctrl_events};
pub use error::{ProbeError, Result};
pub use file_time::FileTime;

/// 自系统启动以来经过的毫秒数（Windows 的 `GetTickCount64`）。
///
/// 分辨率受系统时钟限制，通常在 10 ms 到 16 ms 之间；它没有被
/// `GetSystemTimeAdjustment` 调整过，所以只适合量「过了多久」，
/// 不适合当高精度计时器。
///
/// ```no_run
/// # fn main() -> Result<(), sys_probe::ProbeError> {
/// let uptime = sys_probe::uptime_millis()?;
/// println!("开机 {uptime} ms");
/// # Ok(())
/// # }
/// ```
///
/// 标 `no_run` 是因为结果随平台而异：Windows 上拿到真实数值，别的平台上
/// 这里是 [`ProbeError::Unsupported`]，两边都不该当断言跑。
pub fn uptime_millis() -> Result<u64> {
    platform::get_tick_count_64()
}

/// 当前 UTC 时间，用 Windows 的 [`FileTime`] 表示。
///
/// `GetSystemTimeAsFileTime` 是「取当前时间并写进出参」的典型形状：
/// 返回 `VOID`，失败与否则靠参数说话 —— 它没有失败路径。
///
/// ```no_run
/// # fn main() -> Result<(), sys_probe::ProbeError> {
/// let ft = sys_probe::system_file_time()?;
/// println!("{} 个 100 纳秒", ft.ticks());
/// # Ok(())
/// # }
/// ```
pub fn system_file_time() -> Result<FileTime> {
    platform::get_system_time_as_file_time()
}

/// 当前 UTC 时间距 Unix 纪元（1970-01-01）的毫秒数。
///
/// ```no_run
/// # fn main() -> Result<(), sys_probe::ProbeError> {
/// let ms = sys_probe::unix_time_millis()?;
/// assert!(ms > 1_600_000_000_000, "这个断言假设我们在 2020 年之后");
/// # Ok(())
/// # }
/// ```
pub fn unix_time_millis() -> Result<u64> {
    system_file_time()?.unix_millis()
}
