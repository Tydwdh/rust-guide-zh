//! 第 31 章「测试进阶」的演示 crate：单元测试、表驱动、doctest 属性。
//!
//! 四种 doctest 属性（默认、`no_run`、`should_panic`、`compile_fail`）都写在
//! 下面的文档注释里，一次 `cargo test` 全部跑到。

use std::fmt;
use std::str::FromStr;

/// 日志级别。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
}

impl LogLevel {
    /// 级别的规范小写名。
    pub fn as_str(self) -> &'static str {
        match self {
            LogLevel::Error => "error",
            LogLevel::Warn => "warn",
            LogLevel::Info => "info",
            LogLevel::Debug => "debug",
        }
    }
}

/// 解析配置时的错误。
#[derive(Debug, PartialEq, Eq)]
pub struct ConfigError(String);

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for ConfigError {}

/// 从字符串解析日志级别，大小写不敏感、忽略首尾空白。
///
/// 默认属性：编译并运行，断言失败就是测试失败。
///
/// ```
/// use ch31_demo::LogLevel;
///
/// let level: LogLevel = "info".parse().unwrap();
/// assert_eq!(level, LogLevel::Info);
/// assert_eq!(level.as_str(), "info");
/// ```
///
/// 隐藏行（行首 `#`）只在编译时存在，渲染时不显示；这里用它补上 `use`。
///
/// ```
/// # use ch31_demo::LogLevel;
/// assert_eq!("DEBUG".parse::<LogLevel>().unwrap(), LogLevel::Debug);
/// assert_eq!("  Warn ".parse::<LogLevel>().unwrap(), LogLevel::Warn);
/// ```
///
/// 无法识别的级别返回 `Err`，错误信息里带上原输入：
///
/// ```
/// # use ch31_demo::LogLevel;
/// let err = "verbose".parse::<LogLevel>().unwrap_err();
/// assert_eq!(err.to_string(), "无法识别的日志级别：\"verbose\"");
/// ```
///
/// `LogLevel` 不是字符串，必须显式解析——这个块**必须编译失败**：
///
/// ```compile_fail
/// use ch31_demo::LogLevel;
///
/// let level: LogLevel = "info";
/// ```
impl FromStr for LogLevel {
    type Err = ConfigError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "error" => Ok(LogLevel::Error),
            "warn" => Ok(LogLevel::Warn),
            "info" => Ok(LogLevel::Info),
            "debug" => Ok(LogLevel::Debug),
            other => Err(ConfigError(format!("无法识别的日志级别：{other:?}"))),
        }
    }
}

/// 规范单词：去掉首尾空白、转小写，并把内部连续空白压成单个空格。
pub fn normalize_word(s: &str) -> String {
    s.split_whitespace()
        .map(|w| w.to_ascii_lowercase())
        .collect::<Vec<_>>()
        .join(" ")
}

/// 把端口字符串转成 `u16`；不是数字或为 0 时 panic。
///
/// `should_panic`：块会运行，且**必须** panic，否则测试失败。
///
/// ```should_panic
/// ch31_demo::port_or_panic("0");
/// ```
///
/// 合法输入正常返回（对照组）：
///
/// ```
/// assert_eq!(ch31_demo::port_or_panic(" 8080 "), 8080);
/// ```
pub fn port_or_panic(s: &str) -> u16 {
    let Ok(n) = s.trim().parse::<u16>() else {
        panic!("端口不是合法数字：{s:?}");
    };
    if n == 0 {
        panic!("端口不能为 0：{s:?}");
    }
    n
}

/// 从文件读配置——示例需要外部资源，用 `no_run` 只编译不运行。
///
/// ```no_run
/// use ch31_demo::LogLevel;
/// use std::fs;
///
/// let text = fs::read_to_string("config.txt").unwrap();
/// let level: LogLevel = text.trim().parse().unwrap();
/// println!("日志级别 = {}", level.as_str());
/// ```
pub fn read_level_from_file(path: &str) -> Result<LogLevel, Box<dyn std::error::Error>> {
    let text = std::fs::read_to_string(path)?;
    Ok(text.trim().parse()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_levels_case_insensitively() {
        let cases = [
            ("error", LogLevel::Error),
            ("ERROR", LogLevel::Error),
            ("Warn", LogLevel::Warn),
            ("info", LogLevel::Info),
            ("  debug  ", LogLevel::Debug),
        ];
        for (input, expected) in cases {
            let got = input.parse::<LogLevel>();
            assert_eq!(got, Ok(expected), "输入 {input:?}");
        }
    }

    #[test]
    fn rejects_unknown_levels() {
        let cases = ["", "verbose", "info2", "日志"];
        for input in cases {
            let got = input.parse::<LogLevel>();
            assert!(got.is_err(), "输入 {input:?} 本应被拒绝，却得到 {got:?}");
        }
    }

    #[test]
    fn round_trips_through_as_str() {
        let cases = [
            LogLevel::Error,
            LogLevel::Warn,
            LogLevel::Info,
            LogLevel::Debug,
        ];
        for level in cases {
            assert_eq!(level.as_str().parse::<LogLevel>(), Ok(level));
        }
    }

    #[test]
    fn normalizes_words() {
        let cases = [
            ("Hello", "hello"),
            ("  World  ", "world"),
            ("RUST", "rust"),
            ("Two  Words", "two words"),
        ];
        for (input, expected) in cases {
            assert_eq!(normalize_word(input), expected, "输入 {input:?}");
        }
    }

    #[test]
    #[should_panic(expected = "端口不能为 0")]
    fn rejects_zero_port() {
        let _ = port_or_panic("0");
    }
}
