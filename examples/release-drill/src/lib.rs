//! 把字节数变成人能读的大小字符串，并把这种字符串读回字节数。
//!
//! 这个 crate 只做一件事的两个方向：
//!
//! - [`format()`]：`1536` → `"1.5 KiB"`；
//! - [`parse()`]：`"1.5 KiB"` → `1536`。
//!
//! ```
//! use release_drill::{format, parse};
//!
//! assert_eq!(format(1536), "1.5 KiB");
//! assert_eq!(parse("1.5 KiB").unwrap(), 1536);
//! ```
//!
//! 两套单位在 [`parse()`] 里都能输入：1024 进制的 `KiB` 与 1000 进制的 `kB`；
//! [`format()`] 只输出 1024 进制那一套 —— 写出去只有一种样子，读进来尽量宽容。
//! 具体的档位与舍入规则见各函数的文档。

use std::fmt;

/// [`format`] 使用的单位表：1024 进制的全部档位。
const IEC_UNITS: [&str; 7] = ["B", "KiB", "MiB", "GiB", "TiB", "PiB", "EiB"];

/// [`parse`] 认识的单位后缀（一律小写）与对应的字节数。
const SUFFIXES: &[(&str, u64)] = &[
    // 1024 进制
    ("kib", 1 << 10),
    ("mib", 1 << 20),
    ("gib", 1 << 30),
    ("tib", 1 << 40),
    ("pib", 1 << 50),
    ("eib", 1 << 60),
    // 1000 进制
    ("kb", 1_000),
    ("mb", 1_000_000),
    ("gb", 1_000_000_000),
    ("tb", 1_000_000_000_000),
    ("pb", 1_000_000_000_000_000),
    ("eb", 1_000_000_000_000_000_000),
    // 裸字节
    ("b", 1),
];

/// 把字节数格式化成 1024 进制的大小字符串。
///
/// 档位从 `B` 到 `EiB`，每档除以 1024；尾数小于 10 时保留一位小数，
/// 否则取整（`10 KiB` 比 `10.0 KiB` 好读）。
///
/// ```
/// use release_drill::format;
///
/// assert_eq!(format(0), "0 B");
/// assert_eq!(format(1023), "1023 B");
/// assert_eq!(format(1024), "1.0 KiB");
/// assert_eq!(format(1536), "1.5 KiB");
/// assert_eq!(format(10 * 1024), "10 KiB");
/// assert_eq!(format(1024 * 1024 * 1024), "1.0 GiB");
/// ```
///
/// 内部的中间值用 `f64`：`u64::MAX`（略小于 16 EiB）会打印成 `16 EiB`，
/// 这是「一位小数/取整」规则与浮点精度的合成结果，不是精确值。
#[must_use]
pub fn format(bytes: u64) -> String {
    let mut value = bytes as f64;
    let mut index = 0;
    while value >= 1024.0 && index + 1 < IEC_UNITS.len() {
        value /= 1024.0;
        index += 1;
    }
    if index == 0 {
        format!("{bytes} B")
    } else if value < 10.0 {
        format!("{value:.1} {}", IEC_UNITS[index])
    } else {
        format!("{value:.0} {}", IEC_UNITS[index])
    }
}

/// 把大小字符串解析回字节数。
///
/// 接受的写法：十进制数字（可带小数点）、可选的一个单位后缀、数字与单位之间可有空格；
/// 单位大小写不敏感。没有后缀时按字节算。乘出来不是整数时四舍五入到最近的字节。
///
/// ```
/// use release_drill::parse;
///
/// assert_eq!(parse("2048"), Ok(2048));
/// assert_eq!(parse("1.5 KiB"), Ok(1536));
/// assert_eq!(parse("1.5KiB"), Ok(1536));
/// assert_eq!(parse("1.5MB"), Ok(1_500_000));
/// assert_eq!(parse("1.5 MiB"), Ok(1572864));
///
/// assert!(parse("1 Gi").is_err());
/// ```
///
/// 两个容易踩的点：`kB` 是 1000 字节、`KiB` 是 1024 字节（两套单位都能输入，
/// 但不会互相换算）；数值超过 `u64` 能表示的范围时报 [`ParseError::TooLarge`]，
/// 而不是回绕。
pub fn parse(text: &str) -> Result<u64, ParseError> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(ParseError::Empty);
    }
    // 数字部分 = 前导的 ASCII 数字与小数点；剩下的都归单位部分。
    let split = trimmed
        .find(|c: char| !c.is_ascii_digit() && c != '.')
        .unwrap_or(trimmed.len());
    let (number, rest) = trimmed.split_at(split);
    let value: f64 = number.parse().map_err(|_| ParseError::InvalidNumber {
        text: trimmed.to_owned(),
    })?;

    let unit = rest.trim();
    let multiplier = if unit.is_empty() {
        1
    } else {
        let lowered = unit.to_ascii_lowercase();
        SUFFIXES
            .iter()
            .find(|(suffix, _)| *suffix == lowered)
            .map(|(_, multiplier)| *multiplier)
            .ok_or_else(|| ParseError::UnknownUnit {
                unit: unit.to_owned(),
            })?
    };

    // `value` 可能大到 f64 的上限（几百位数字的输入会解析成 inf），
    // 所以比较放在乘法之后：超出 u64 能表示的范围就报 TooLarge，而不是回绕。
    let bytes = value * multiplier as f64;
    if bytes >= u64::MAX as f64 {
        return Err(ParseError::TooLarge {
            text: trimmed.to_owned(),
        });
    }
    Ok(bytes.round() as u64)
}

/// [`parse`] 可能报出的四类错误。
///
/// 标 `#[non_exhaustive]`：以后再加变体不算破坏性变更，下游 `match` 时留一个 `_` 分支即可。
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParseError {
    /// 输入去掉首尾空白后什么都没有。
    Empty,
    /// 数字部分不是合法的十进制数。
    InvalidNumber {
        /// 触发错误的原始输入（已去掉首尾空白）。
        text: String,
    },
    /// 单位后缀不在认识的表里。
    UnknownUnit {
        /// 认不出来的那个后缀。
        unit: String,
    },
    /// 换算出来的字节数超出 `u64` 能表示的范围。
    TooLarge {
        /// 触发错误的原始输入（已去掉首尾空白）。
        text: String,
    },
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("输入是空的"),
            Self::InvalidNumber { text } => {
                write!(f, "`{text}` 的数字部分不是合法的十进制数")
            }
            Self::UnknownUnit { unit } => write!(f, "不认识的单位 `{unit}`"),
            Self::TooLarge { text } => write!(f, "`{text}` 换算后超出 u64 能表示的范围"),
        }
    }
}

impl std::error::Error for ParseError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes_below_one_kibibyte_keep_the_exact_count() {
        assert_eq!(format(0), "0 B");
        assert_eq!(format(1), "1 B");
        assert_eq!(format(1023), "1023 B");
    }

    #[test]
    fn one_decimal_below_ten_integers_above() {
        assert_eq!(format(1024), "1.0 KiB");
        assert_eq!(format(1536), "1.5 KiB");
        assert_eq!(format(10 * 1024), "10 KiB");
        assert_eq!(format(1536 * 1024), "1.5 MiB");
    }

    #[test]
    fn format_walks_all_the_way_up_to_eib() {
        assert_eq!(format(1 << 50), "1.0 PiB");
        assert_eq!(format(1 << 60), "1.0 EiB");
        // u64 的上限略小于 16 EiB；打印值被 f64 与取整规则抹成了整数。
        assert_eq!(format(u64::MAX), "16 EiB");
    }

    #[test]
    fn parse_accepts_both_unit_families() {
        assert_eq!(parse("1.5 KiB"), Ok(1536));
        assert_eq!(parse("1.5KiB"), Ok(1536));
        assert_eq!(parse("1.5 kB"), Ok(1500));
        assert_eq!(parse("1.5MB"), Ok(1_500_000));
        assert_eq!(parse("  2.5  kb "), Ok(2500));
        assert_eq!(parse("1 eib"), Ok(1 << 60));
    }

    #[test]
    fn parse_treats_a_bare_number_as_bytes() {
        assert_eq!(parse("0"), Ok(0));
        assert_eq!(parse("2048"), Ok(2048));
        assert_eq!(parse("2048 B"), Ok(2048));
    }

    #[test]
    fn parse_rounds_to_the_nearest_byte() {
        assert_eq!(parse("1.1 KiB"), Ok(1126));
        assert_eq!(parse("0.5 KiB"), Ok(512));
    }

    #[test]
    fn format_and_parse_agree_on_whole_kibibytes() {
        for kib in [1u64, 2, 10, 1023, 1024, 4096, 1_048_576] {
            let bytes = kib * 1024;
            assert_eq!(parse(&format(bytes)), Ok(bytes));
        }
    }

    #[test]
    fn empty_input_gets_its_own_error() {
        assert_eq!(parse(""), Err(ParseError::Empty));
        assert_eq!(parse("   "), Err(ParseError::Empty));
    }

    #[test]
    fn bad_numbers_and_bad_units_are_reported_separately() {
        assert_eq!(
            parse("abc"),
            Err(ParseError::InvalidNumber { text: "abc".into() })
        );
        assert_eq!(
            parse("1.2.3 MB"),
            Err(ParseError::InvalidNumber {
                text: "1.2.3 MB".into()
            })
        );
        assert_eq!(
            parse("-1"),
            Err(ParseError::InvalidNumber { text: "-1".into() })
        );
        assert_eq!(
            parse("1 Gi"),
            Err(ParseError::UnknownUnit { unit: "Gi".into() })
        );
    }

    #[test]
    fn jumbo_values_are_rejected_instead_of_wrapping() {
        assert_eq!(
            parse("16 EiB"),
            Err(ParseError::TooLarge {
                text: "16 EiB".into()
            })
        );
        // 几百位数字的输入在 f64 里会溢出成 inf，同样落到 TooLarge。
        let huge = "9".repeat(400);
        assert!(matches!(parse(&huge), Err(ParseError::TooLarge { .. })));
        assert!(matches!(parse("9999 EB"), Err(ParseError::TooLarge { .. })));
    }

    #[test]
    fn error_display_names_the_offending_part() {
        assert_eq!(parse("1 Gi").unwrap_err().to_string(), "不认识的单位 `Gi`");
        assert_eq!(
            parse("abc").unwrap_err().to_string(),
            "`abc` 的数字部分不是合法的十进制数"
        );
    }
}
