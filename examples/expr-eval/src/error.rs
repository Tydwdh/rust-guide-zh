//! 错误类型：把「可能出错的地方」全收进一个公开的枚举。
//!
//! 词法、语法、求值三档错误共用一个类型：调用者拿到的永远是 [`Error`]，
//! 想知道是哪一档就 `match`；想知道错在哪儿用 [`Error::offset`]。

use std::fmt;

/// 本 crate 的错误类型。
///
/// 标了 `#[non_exhaustive]`：以后加变体不算破坏性变更（第 18 章）。
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// 词法阶段：源串里有不认识的字符。
    UnexpectedChar {
        /// 那个字符在源串里的字节偏移。
        offset: usize,
        /// 字符本身。
        ch: char,
    },
    /// 词法阶段：一串数字字面量拼不出 `f64`。
    BadNumber {
        /// 字面量第一个字节的偏移。
        offset: usize,
        /// 字面量原文。
        text: String,
    },
    /// 语法阶段：这里本该是某个东西，实际是别的。
    UnexpectedToken {
        /// 出问题的记号第一个字节的偏移。
        offset: usize,
        /// 本该是什么（给用户看的短句）。
        expected: &'static str,
        /// 实际看到了什么（已经带好反引号，或者是一句中文）。
        found: String,
    },
    /// 语法阶段：括号或前缀号套得太深。
    TooDeep {
        /// 允许的最大嵌套层数。
        limit: usize,
    },
    /// 求值阶段：变量没有绑定。
    UnknownName {
        /// 变量名。
        name: String,
    },
    /// 求值阶段：这个名字不是函数。
    UnknownFunction {
        /// 函数名。
        name: String,
    },
    /// 求值阶段：函数参数个数不对。
    WrongArity {
        /// 函数名。
        name: String,
        /// 需要几个参数。
        expected: usize,
        /// 实际给了几个。
        got: usize,
    },
    /// 求值阶段：结果不是有限数（除零、负数开方、上溢都会这样）。
    NotFinite {
        /// 出问题的运算的可读形式，例如 `1 / 0`、`sqrt(-1)`。
        expr: String,
    },
}

/// 本 crate 统一的 `Result`：错误端默认是 [`Error`]，需要时仍可显式指定别的错误类型。
pub type Result<T, E = Error> = std::result::Result<T, E>;

impl Error {
    /// 错误在源串里的字节偏移；和位置无关的错误返回 `None`。
    ///
    /// 调用者不必 `match` 每个变体就能画出「哪一列出错了」的脱字符。
    #[must_use]
    pub fn offset(&self) -> Option<usize> {
        match self {
            Error::UnexpectedChar { offset, .. }
            | Error::BadNumber { offset, .. }
            | Error::UnexpectedToken { offset, .. } => Some(*offset),
            Error::TooDeep { .. }
            | Error::UnknownName { .. }
            | Error::UnknownFunction { .. }
            | Error::WrongArity { .. }
            | Error::NotFinite { .. } => None,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::UnexpectedChar { offset, ch } => {
                write!(f, "偏移 {offset} 处有不认识的字符 `{ch}`")
            }
            Error::BadNumber { offset, text } => {
                write!(f, "偏移 {offset} 处的数字 `{text}` 拼不出 f64")
            }
            Error::UnexpectedToken {
                offset,
                expected,
                found,
            } => write!(f, "偏移 {offset} 处应该是{expected}，实际看到 {found}"),
            Error::TooDeep { limit } => write!(f, "表达式嵌套太深（最多 {limit} 层）"),
            Error::UnknownName { name } => write!(f, "变量 `{name}` 没有绑定"),
            Error::UnknownFunction { name } => write!(f, "没有名为 `{name}` 的函数"),
            Error::WrongArity {
                name,
                expected,
                got,
            } => write!(f, "函数 `{name}` 需要 {expected} 个参数，实际给了 {got} 个"),
            Error::NotFinite { expr } => {
                write!(f, "{expr} 的结果不是有限数（除零、负数开方、上溢都会这样）")
            }
        }
    }
}

impl std::error::Error for Error {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unexpected_char_reports_the_offset_and_the_char() {
        let err = Error::UnexpectedChar { offset: 3, ch: '@' };
        assert_eq!(err.to_string(), "偏移 3 处有不认识的字符 `@`");
        assert_eq!(err.offset(), Some(3));
    }

    #[test]
    fn wrong_arity_mentions_both_numbers() {
        let err = Error::WrongArity {
            name: "pow".to_owned(),
            expected: 2,
            got: 1,
        };
        assert_eq!(err.to_string(), "函数 `pow` 需要 2 个参数，实际给了 1 个");
        assert_eq!(err.offset(), None);
    }

    #[test]
    fn not_finite_quotes_the_failing_operation() {
        let err = Error::NotFinite {
            expr: "1 / 0".to_owned(),
        };
        assert!(err.to_string().starts_with("1 / 0 的结果不是有限数"));
    }
}
