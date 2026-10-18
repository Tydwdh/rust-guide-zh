//! 应用的错误类型：把「所有可能出错的地方」收进一个枚举。

use std::fmt;
use std::io;

/// 待办工具可能遇到的错误。
#[derive(Debug)]
pub enum AppError {
    /// 读写数据文件失败（权限、路径不存在、磁盘……）。
    Io(io::Error),
    /// 数据文件里第 `line` 行不是合法记录。
    BadRecord { line: usize, reason: String },
    /// 命令行参数不合法。
    BadArgs(String),
    /// 找不到指定编号的任务。
    NoSuchTask(u32),
    /// 任务文本含无法写进数据文件的字符。
    UnstorableText,
    /// 编号已经用满（`u32::MAX`），不能再添加任务。
    IdsExhausted,
}

/// 本 crate 统一的 `Result`：错误端默认是 `AppError`，需要时仍可显式指定别的错误类型。
pub type Result<T, E = AppError> = std::result::Result<T, E>;

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Io(e) => write!(f, "读写数据文件失败：{e}"),
            AppError::BadRecord { line, reason } => {
                write!(f, "数据文件第 {line} 行无法解析：{reason}")
            }
            AppError::BadArgs(msg) => write!(f, "参数错误：{msg}"),
            AppError::NoSuchTask(id) => write!(f, "没有编号为 {id} 的任务"),
            AppError::UnstorableText => write!(f, "任务文本不能包含制表符或换行"),
            AppError::IdsExhausted => write!(f, "任务编号已经用满，无法再添加"),
        }
    }
}

impl std::error::Error for AppError {
    // `dyn Error` 是标准库签名规定的（第 13 章展开）。
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AppError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for AppError {
    fn from(e: io::Error) -> Self {
        AppError::Io(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn io_error_keeps_its_source_for_the_cause_chain() {
        let err = AppError::from(io::Error::new(io::ErrorKind::PermissionDenied, "拒绝"));
        assert!(std::error::Error::source(&err).is_some());
        assert!(err.to_string().starts_with("读写数据文件失败"));
    }

    #[test]
    fn no_such_task_message_contains_the_id() {
        assert_eq!(AppError::NoSuchTask(7).to_string(), "没有编号为 7 的任务");
    }

    #[test]
    fn ids_exhausted_message_is_user_facing() {
        assert_eq!(
            AppError::IdsExhausted.to_string(),
            "任务编号已经用满，无法再添加"
        );
    }
}
