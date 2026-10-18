//! 非 Windows 的降级实现：函数名、签名与 Windows 那份逐字一样，
//! 一律返回 [`ProbeError::Unsupported`]。
//!
//! 它存在的意义有两个：让 CI 在 Linux / macOS 上照样编译并跑通全部测试；
//! 以及**用编译器盯着公开 API** —— 两套实现都要满足同一组签名，
//! 任何一边写歪了都在编译期暴露。
//!
//! 注意：本文件一行 `unsafe` 都没有，也不需要。
#![forbid(unsafe_code)]

use crate::ctrl::HandlerRoutine;
use crate::error::{ProbeError, Result};
use crate::file_time::FileTime;

/// Linux 上没有 `GetTickCount64`。
pub(crate) fn get_tick_count_64() -> Result<u64> {
    Err(ProbeError::unsupported("GetTickCount64"))
}

/// Linux 上没有 `GetSystemTimeAsFileTime`。
pub(crate) fn get_system_time_as_file_time() -> Result<FileTime> {
    Err(ProbeError::unsupported("GetSystemTimeAsFileTime"))
}

/// Linux 上没有 `SetConsoleCtrlHandler`。
pub(crate) fn set_console_ctrl_handler(_handler: HandlerRoutine, _add: bool) -> Result<()> {
    Err(ProbeError::unsupported("SetConsoleCtrlHandler"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 降级实现不会真的调用它，只是签名要对得上。
    extern "system" fn never_called(_: u32) -> i32 {
        0
    }

    #[test]
    fn every_entry_point_reports_the_missing_api_by_name() {
        let apis = [
            get_tick_count_64().map(|_| "GetTickCount64"),
            get_system_time_as_file_time().map(|_| "GetSystemTimeAsFileTime"),
            set_console_ctrl_handler(never_called, true).map(|()| "SetConsoleCtrlHandler"),
        ];

        for result in apis {
            match result {
                Err(ProbeError::Unsupported { api }) => {
                    assert!(
                        api.starts_with("Get") || api.starts_with("Set"),
                        "意外的 api：{api}"
                    );
                }
                other => panic!("降级实现只该回 Unsupported，实际是 {other:?}"),
            }
        }
    }
}
