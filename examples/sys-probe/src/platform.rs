//! 平台层：同一个函数名，两套实现，签名一模一样。
//!
//! 上层代码（[`crate::ctrl`]、`lib.rs`）只认这里的签名，一个 `cfg` 都不用写；
//! 「哪些平台有什么」这件事全部收在这两个文件里。
//!
//! 两套实现里的每一个函数都返回 [`crate::Result`]：Windows 实现里可能失败的是
//! 「系统调用报错」，降级实现里失败的是「这个平台根本没有这个 API」。

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub(crate) use windows::{
    get_system_time_as_file_time, get_tick_count_64, set_console_ctrl_handler,
};

#[cfg(not(windows))]
mod unsupported;
#[cfg(not(windows))]
pub(crate) use unsupported::{
    get_system_time_as_file_time, get_tick_count_64, set_console_ctrl_handler,
};
