//! 词法分析：把源串切成记号流。
//!
//! 记号里的标识符**借用**源串（`Ident(&'a str)`）：整趟词法分析一次内存都不分配，
//! 切出来的切片直接指向输入。真正的复制发生在语法分析那一步 —— 树要活得比源串长。

use std::fmt;

use crate::error::{Error, Result};

/// 记号的种类。
///
/// `Num` 在词法阶段就解析成 `f64`：这样语法树里只有一种数字表示。
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum TokenKind<'a> {
    /// 数字字面量。
    Num(f64),
    /// 标识符：变量名或函数名；借用源串里的那一段。
    Ident(&'a str),
    /// `+`
    Plus,
    /// `-`
    Minus,
    /// `*`
    Star,
    /// `/`
    Slash,
    /// `^`
    Caret,
    /// `(`
    LParen,
    /// `)`
    RParen,
    /// `,`
    Comma,
}

impl fmt::Display for TokenKind<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenKind::Num(value) => write!(f, "{value}"),
            TokenKind::Ident(name) => f.write_str(name),
            TokenKind::Plus => f.write_str("+"),
            TokenKind::Minus => f.write_str("-"),
            TokenKind::Star => f.write_str("*"),
            TokenKind::Slash => f.write_str("/"),
            TokenKind::Caret => f.write_str("^"),
            TokenKind::LParen => f.write_str("("),
            TokenKind::RParen => f.write_str(")"),
            TokenKind::Comma => f.write_str(","),
        }
    }
}

/// 一个记号：种类 + 起点的字节偏移。
///
/// 偏移留着报错用（[`Error::offset`]）。`Token` 是个定长小值，派生 `Copy` 之后
/// 「看一眼再决定」就不必和借用检查器较劲。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Token<'a> {
    /// 记号种类。
    pub kind: TokenKind<'a>,
    /// 记号第一个字节在源串里的位置。
    pub offset: usize,
}

/// 词法分析器：按需产出记号，不预先切完整个输入。
pub struct Lexer<'a> {
    src: &'a str,
    pos: usize,
}

impl<'a> Lexer<'a> {
    /// 为源串建一个词法分析器。
    #[must_use]
    pub fn new(src: &'a str) -> Self {
        Lexer { src, pos: 0 }
    }

    /// 跳过空白（含换行与中文全角空格 —— `char::is_whitespace` 说了算）。
    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.src[self.pos..].chars().next() {
            if ch.is_whitespace() {
                self.pos += ch.len_utf8();
            } else {
                break;
            }
        }
    }

    /// 扫描一个数字字面量。
    ///
    /// 规则：`.5` 与 `1.` 都收；`e`/`E` 后面必须真跟着（可带符号的）数字，
    /// 否则整段（如 `2e`）算作坏的数字字面量 —— 免得 `2e` 被静默切成 `2` 和变量 `e`。
    fn number(&mut self, offset: usize) -> Result<Token<'a>> {
        let bytes = self.src.as_bytes();
        let mut end = offset;
        while end < bytes.len() && bytes[end].is_ascii_digit() {
            end += 1;
        }
        if end < bytes.len() && bytes[end] == b'.' {
            end += 1;
            while end < bytes.len() && bytes[end].is_ascii_digit() {
                end += 1;
            }
        }
        if end < bytes.len() && (bytes[end] == b'e' || bytes[end] == b'E') {
            let mut exponent_end = end + 1;
            if exponent_end < bytes.len()
                && (bytes[exponent_end] == b'+' || bytes[exponent_end] == b'-')
            {
                exponent_end += 1;
            }
            let digits_start = exponent_end;
            while exponent_end < bytes.len() && bytes[exponent_end].is_ascii_digit() {
                exponent_end += 1;
            }
            if exponent_end > digits_start {
                end = exponent_end;
            } else {
                let text = self.src[offset..exponent_end].to_owned();
                self.pos = exponent_end;
                return Err(Error::BadNumber { offset, text });
            }
        }
        let text = &self.src[offset..end];
        self.pos = end;
        text.parse::<f64>()
            .map(|value| Token {
                kind: TokenKind::Num(value),
                offset,
            })
            .map_err(|_| Error::BadNumber {
                offset,
                text: text.to_owned(),
            })
    }

    /// 扫描一个标识符：`[A-Za-z_][A-Za-z0-9_]*`，切片直接借用源串。
    fn ident(&mut self, offset: usize) -> Token<'a> {
        let bytes = self.src.as_bytes();
        let mut end = offset;
        while end < bytes.len() && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_') {
            end += 1;
        }
        self.pos = end;
        Token {
            kind: TokenKind::Ident(&self.src[offset..end]),
            offset,
        }
    }

    /// 单个字符的运算符与括号（`self.pos` 已经越过它）。
    fn punct(&self, offset: usize, ch: char) -> Result<Token<'a>> {
        let kind = match ch {
            '+' => TokenKind::Plus,
            '-' => TokenKind::Minus,
            '*' => TokenKind::Star,
            '/' => TokenKind::Slash,
            '^' => TokenKind::Caret,
            '(' => TokenKind::LParen,
            ')' => TokenKind::RParen,
            ',' => TokenKind::Comma,
            other => return Err(Error::UnexpectedChar { offset, ch: other }),
        };
        Ok(Token { kind, offset })
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Result<Token<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        self.skip_whitespace();
        let offset = self.pos;
        let rest = &self.src[offset..];
        match rest.chars().next()? {
            '0'..='9' => Some(self.number(offset)),
            '.' if rest.chars().nth(1).is_some_and(|c| c.is_ascii_digit()) => {
                Some(self.number(offset))
            }
            ch if ch.is_ascii_alphabetic() || ch == '_' => Some(Ok(self.ident(offset))),
            ch => {
                self.pos += ch.len_utf8();
                Some(self.punct(offset, ch))
            }
        }
    }
}

/// 一次切完整个源串；遇到第一个词法错误就返回。
///
/// # Examples
///
/// ```
/// use expr_eval::{TokenKind, tokenize};
///
/// let tokens = tokenize("1 + x").unwrap();
/// let kinds: Vec<TokenKind<'_>> = tokens.iter().map(|t| t.kind).collect();
/// assert_eq!(
///     kinds,
///     vec![TokenKind::Num(1.0), TokenKind::Plus, TokenKind::Ident("x")]
/// );
/// ```
pub fn tokenize(src: &str) -> Result<Vec<Token<'_>>> {
    Lexer::new(src).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(src: &str) -> Vec<TokenKind<'_>> {
        tokenize(src).unwrap().into_iter().map(|t| t.kind).collect()
    }

    #[test]
    fn whitespace_is_skipped_and_operators_come_out_one_by_one() {
        assert_eq!(
            kinds("1 + 2*3"),
            vec![
                TokenKind::Num(1.0),
                TokenKind::Plus,
                TokenKind::Num(2.0),
                TokenKind::Star,
                TokenKind::Num(3.0),
            ]
        );
    }

    #[test]
    fn identifiers_borrow_from_the_source_instead_of_copying() {
        let src = String::from("alpha + beta");
        let tokens = tokenize(&src).unwrap();
        let TokenKind::Ident(name) = tokens[0].kind else {
            panic!("第一个记号应该是标识符");
        };
        // 切片指向的就是 src 自己的那块内存 —— 词法阶段一次分配都没有。
        assert!(std::ptr::eq(name.as_ptr(), src.as_ptr()));
        assert_eq!(name, "alpha");
    }

    #[test]
    fn numbers_cover_leading_dot_trailing_dot_and_exponents() {
        assert_eq!(kinds(".5"), vec![TokenKind::Num(0.5)]);
        assert_eq!(kinds("1."), vec![TokenKind::Num(1.0)]);
        assert_eq!(kinds("2e3"), vec![TokenKind::Num(2000.0)]);
        assert_eq!(kinds("1E-2"), vec![TokenKind::Num(0.01)]);
    }

    #[test]
    fn an_exponent_marker_without_digits_is_a_bad_number() {
        assert_eq!(
            tokenize("2e").unwrap_err().to_string(),
            "偏移 0 处的数字 `2e` 拼不出 f64"
        );
        assert_eq!(
            tokenize("1e+").unwrap_err().to_string(),
            "偏移 0 处的数字 `1e+` 拼不出 f64"
        );
    }

    #[test]
    fn positions_are_byte_offsets_of_each_token() {
        let tokens = tokenize("12 + x").unwrap();
        let offsets: Vec<usize> = tokens.iter().map(|t| t.offset).collect();
        assert_eq!(offsets, vec![0, 3, 5]);
    }

    #[test]
    fn full_width_spaces_are_whitespace_and_offsets_stay_byte_based() {
        // U+3000 是三个字节：第二个记号的偏移因此是 3 而不是 1。
        let tokens = tokenize("　1　+　2").unwrap();
        let kinds: Vec<TokenKind<'_>> = tokens.iter().map(|t| t.kind).collect();
        assert_eq!(
            kinds,
            vec![TokenKind::Num(1.0), TokenKind::Plus, TokenKind::Num(2.0)]
        );
        let offsets: Vec<usize> = tokens.iter().map(|t| t.offset).collect();
        assert_eq!(offsets, vec![3, 7, 11]);
    }

    #[test]
    fn an_unknown_char_is_reported_with_its_position() {
        let err = tokenize("1 $ 2").unwrap_err();
        assert_eq!(err.to_string(), "偏移 2 处有不认识的字符 `$`");
        assert_eq!(err.offset(), Some(2));
    }

    #[test]
    fn a_bare_dot_is_not_a_number() {
        assert_eq!(
            tokenize(".").unwrap_err().to_string(),
            "偏移 0 处有不认识的字符 `.`"
        );
    }
}
