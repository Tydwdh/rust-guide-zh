//! 错误设计：把一个「封装系统 API 的库」能遇到的三类问题分开。
//!
//! - 系统调用本身失败 —— [`ProbeError::Os`]，带 Win32 错误码；
//! - 这个平台没有这个 API —— [`ProbeError::Unsupported`]；
//! - 值换算不出来 —— [`ProbeError::BeforeUnixEpoch`]。
//!
//! 另外还有一个「用法不对」的错误：[`ProbeError::HandlerInUse`]。
#![forbid(unsafe_code)]

use std::fmt;
use std::io;

/// crate 统一的 `Result` 别名：错误类型默认就是 [`ProbeError`]。
pub type Result<T, E = ProbeError> = std::result::Result<T, E>;

/// 这个库可能报出的全部错误。
///
/// 枚举标了 `#[non_exhaustive]`：以后加变体不算破坏性改动，但调用方的
/// `match` 必须留一个 `_` 分支。
#[non_exhaustive]
#[derive(Debug)]
pub enum ProbeError {
    /// 当前平台上没有这个系统 API。
    ///
    /// 非 Windows 构建里，本 crate 的每个函数都返回这个变体。
    Unsupported {
        /// 缺的是哪个 API（Windows 头文件里的原名，如 `GetTickCount64`）。
        api: &'static str,
    },
    /// 系统调用失败了。
    Os {
        /// 失败的是哪个 API。
        api: &'static str,
        /// `GetLastError` 给出的 Win32 错误码。
        code: u32,
        /// 同一个错误码对应的系统错误对象，留给 [`std::error::Error::source`]。
        source: io::Error,
    },
    /// 时间戳早于 Unix 纪元（1970-01-01 UTC），换不成 Unix 毫秒。
    BeforeUnixEpoch {
        /// 原始的 100 纳秒计数。
        ticks: u64,
    },
    /// 本进程已经注册过一个控制事件处理器了。
    ///
    /// Windows 允许一个进程注册多个处理器，但本库只留一个槽位：
    /// 一个槽位才能让「谁处理这个事件」这件事有唯一答案。
    HandlerInUse,
}

// 这两个构造函数各服务一套平台实现：Windows 构建里 `unsupported` 用不上，
// 非 Windows 构建里 `os` 用不上。两个都留着，不让实现跟着 cfg 走。
#[allow(dead_code)]
impl ProbeError {
    /// 构造「平台不支持」。
    pub(crate) fn unsupported(api: &'static str) -> Self {
        Self::Unsupported { api }
    }

    /// 构造「系统调用失败」：错误码与它的系统消息一起存下来。
    pub(crate) fn os(api: &'static str, code: u32) -> Self {
        Self::Os {
            api,
            code,
            source: io::Error::from_raw_os_error(code as i32),
        }
    }
}

impl fmt::Display for ProbeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsupported { api } => write!(f, "{api} 在当前平台上不可用"),
            // 只印错误码，不印系统消息：系统消息随系统语言变，错误码不变。
            Self::Os { api, code, .. } => write!(f, "{api} 调用失败（Win32 错误码 {code}）"),
            Self::BeforeUnixEpoch { ticks } => write!(
                f,
                "时间戳 {ticks} 早于 Unix 纪元（1970-01-01 UTC），换算不出 Unix 毫秒"
            ),
            Self::HandlerInUse => write!(f, "本进程已经注册过一个控制事件处理器"),
        }
    }
}

impl std::error::Error for ProbeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Os { source, .. } => Some(source),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_says_which_api_is_missing() {
        let err = ProbeError::unsupported("GetTickCount64");
        assert_eq!(err.to_string(), "GetTickCount64 在当前平台上不可用");
    }

    #[test]
    fn os_error_keeps_the_code_in_the_message_and_in_the_source_chain() {
        let err = ProbeError::os("SetConsoleCtrlHandler", 6);
        assert_eq!(
            err.to_string(),
            "SetConsoleCtrlHandler 调用失败（Win32 错误码 6）"
        );

        let source = std::error::Error::source(&err).expect("Os 变体应该有原因链");
        assert_eq!(
            source.downcast_ref::<io::Error>().unwrap().raw_os_error(),
            Some(6)
        );
    }

    #[test]
    fn other_variants_have_no_source() {
        assert!(std::error::Error::source(&ProbeError::HandlerInUse).is_none());
        assert!(std::error::Error::source(&ProbeError::BeforeUnixEpoch { ticks: 0 }).is_none());
    }
}
