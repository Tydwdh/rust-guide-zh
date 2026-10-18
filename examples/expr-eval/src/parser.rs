//! 语法分析：递归下降 + Pratt，产出拥有数据的 [`Expr`]。
//!
//! 表达式文法只有一层递归（`expr` 里套 `expr`），优先级交给一张表，不在文法里编码。

use std::iter::Peekable;

use crate::ast::{BinOp, Expr};
use crate::error::{Error, Result};
use crate::lexer::{Lexer, Token, TokenKind};

/// 前缀负号的绑定力。
///
/// 它比 `*` 松（3 < 5）、比 `^` 紧（5 < 7）：`-2 * 3` 是 `(-2) * 3`，
/// `-2 ^ 2` 是 `-(2 ^ 2)` —— 跟纸面上的习惯一致。
const PREFIX_BP: u8 = 5;

/// 解析器允许的最大嵌套层数。
///
/// 递归下降每进一层括号或前缀号，就往调用栈上多压几帧；不设上限时
/// `((((1))))` 这类输入会把栈撑爆（debug 构建下大约 400 层就够，
/// 结果是进程 abort，连 `Result` 都来不及给）。128 层对计算器远远够用。
const MAX_DEPTH: usize = 128;

/// 把源串解析成一棵表达式树；除了空白，输入必须全部用掉。
///
/// # Examples
///
/// ```
/// use expr_eval::parse;
///
/// let expr = parse("1 + 2 * 3").unwrap();
/// assert_eq!(expr.to_string(), "(1 + (2 * 3))");
/// assert!(parse("1 +").is_err());
/// ```
pub fn parse(src: &str) -> Result<Expr> {
    let mut parser = Parser::new(src);
    let expr = parser.expr_bp(0)?;
    if let Some(tok) = parser.peek_token()? {
        return Err(Error::UnexpectedToken {
            offset: tok.offset,
            expected: "输入的末尾",
            found: describe(tok.kind),
        });
    }
    Ok(expr)
}

/// 操作符表：记号 → 中缀操作符。表里没有的记号就不是中缀操作符。
fn infix_op(kind: TokenKind<'_>) -> Option<BinOp> {
    match kind {
        TokenKind::Plus => Some(BinOp::Add),
        TokenKind::Minus => Some(BinOp::Sub),
        TokenKind::Star => Some(BinOp::Mul),
        TokenKind::Slash => Some(BinOp::Div),
        TokenKind::Caret => Some(BinOp::Pow),
        _ => None,
    }
}

/// 报错时怎么称呼一个记号。
fn describe(kind: TokenKind<'_>) -> String {
    format!("`{kind}`")
}

struct Parser<'a> {
    lexer: Peekable<Lexer<'a>>,
    /// 源串长度：给「输入提前结束」这类错误定位。
    len: usize,
    /// 当前嵌套深度，`MAX_DEPTH` 是上限。
    depth: usize,
}

impl<'a> Parser<'a> {
    fn new(src: &'a str) -> Self {
        Parser {
            lexer: Lexer::new(src).peekable(),
            len: src.len(),
            depth: 0,
        }
    }

    /// 取下一个记号；输入结束是 `None`，词法错误是 `Err`。
    fn next_token(&mut self) -> Result<Option<Token<'a>>> {
        self.lexer.next().transpose()
    }

    /// 看一眼下一个记号，不消耗它。
    fn peek_token(&mut self) -> Result<Option<Token<'a>>> {
        match self.lexer.peek() {
            Some(Ok(tok)) => Ok(Some(*tok)),
            // 词法错误留到调用者真正取它的时候再返回。
            Some(Err(_)) => self.lexer.next().transpose(),
            None => Ok(None),
        }
    }

    /// 「输入提前结束」的错误。
    fn eof(&self, expected: &'static str) -> Error {
        Error::UnexpectedToken {
            offset: self.len,
            expected,
            found: "输入的末尾".to_owned(),
        }
    }

    /// 带深度记账的一层表达式：进来先查上限，出错也照样把深度还回去。
    fn expr_bp(&mut self, min_bp: u8) -> Result<Expr> {
        if self.depth >= MAX_DEPTH {
            return Err(Error::TooDeep { limit: MAX_DEPTH });
        }
        self.depth += 1;
        let parsed = self.expr_level(min_bp);
        self.depth -= 1;
        parsed
    }

    /// Pratt 主循环：先取一个前缀表达式，再按绑定力把中缀操作符接上去。
    fn expr_level(&mut self, min_bp: u8) -> Result<Expr> {
        let mut lhs = self.prefix()?;
        while let Some(tok) = self.peek_token()? {
            let Some(op) = infix_op(tok.kind) else { break };
            let (left_bp, right_bp) = op.binding_power();
            // 绑定力不够紧的操作符留给外层 —— 这就是「优先级」的全部实现。
            if left_bp < min_bp {
                break;
            }
            // 吃掉这个操作符：上面刚 peek 过，一定还在。
            let _ = self.next_token()?;
            let rhs = self.expr_bp(right_bp)?;
            lhs = Expr::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }

    /// 前缀位置：字面量、变量、函数调用、括号，或一元正负号。
    fn prefix(&mut self) -> Result<Expr> {
        let Some(tok) = self.next_token()? else {
            return Err(self.eof("一个值"));
        };
        match tok.kind {
            TokenKind::Num(value) => Ok(Expr::Num(value)),
            TokenKind::Ident(name) => {
                // 标识符后面紧跟 `(` 就是调用，否则是变量。
                if matches!(self.peek_token()?, Some(t) if matches!(t.kind, TokenKind::LParen)) {
                    let _ = self.next_token()?;
                    let args = self.call_args()?;
                    Ok(Expr::Call {
                        name: name.to_owned(),
                        args,
                    })
                } else {
                    Ok(Expr::Var(name.to_owned()))
                }
            }
            TokenKind::Minus => Ok(Expr::Neg(Box::new(self.expr_bp(PREFIX_BP)?))),
            // 一元加号不改值，直接把操作数交给上层。
            TokenKind::Plus => self.expr_bp(PREFIX_BP),
            TokenKind::LParen => {
                let inner = self.expr_bp(0)?;
                self.expect_rparen()?;
                Ok(inner)
            }
            other => Err(Error::UnexpectedToken {
                offset: tok.offset,
                expected: "一个值",
                found: describe(other),
            }),
        }
    }

    /// 实参列表：进来时 `(` 已经吃掉，返回时 `)` 已经吃掉。
    fn call_args(&mut self) -> Result<Vec<Expr>> {
        let mut args = Vec::new();
        match self.peek_token()? {
            // `f()` 也收：参数个数对不对是求值阶段的事。
            Some(tok) if matches!(tok.kind, TokenKind::RParen) => {
                let _ = self.next_token()?;
                return Ok(args);
            }
            Some(_) => {}
            None => return Err(self.eof("参数或 `)`")),
        }
        loop {
            args.push(self.expr_bp(0)?);
            let Some(tok) = self.peek_token()? else {
                return Err(self.eof("`,` 或 `)`"));
            };
            match tok.kind {
                TokenKind::Comma => {
                    let _ = self.next_token()?;
                }
                TokenKind::RParen => {
                    let _ = self.next_token()?;
                    return Ok(args);
                }
                other => {
                    return Err(Error::UnexpectedToken {
                        offset: tok.offset,
                        expected: "`,` 或 `)`",
                        found: describe(other),
                    });
                }
            }
        }
    }

    /// 吃掉一个 `)`，或者在它该在的地方报错。
    fn expect_rparen(&mut self) -> Result<()> {
        match self.peek_token()? {
            Some(tok) if matches!(tok.kind, TokenKind::RParen) => {
                let _ = self.next_token()?;
                Ok(())
            }
            Some(tok) => Err(Error::UnexpectedToken {
                offset: tok.offset,
                expected: "`)`",
                found: describe(tok.kind),
            }),
            None => Err(self.eof("`)`")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(src: &str) -> String {
        parse(src).unwrap().to_string()
    }

    #[test]
    fn multiplication_binds_tighter_than_addition() {
        assert_eq!(parsed("1 + 2 * 3"), "(1 + (2 * 3))");
        assert_eq!(parsed("1 * 2 + 3"), "((1 * 2) + 3)");
    }

    #[test]
    fn same_precedence_operators_are_left_associative() {
        assert_eq!(parsed("1 - 2 - 3"), "((1 - 2) - 3)");
        assert_eq!(parsed("8 / 4 / 2"), "((8 / 4) / 2)");
    }

    #[test]
    fn power_is_right_associative() {
        assert_eq!(parsed("2 ^ 3 ^ 2"), "(2 ^ (3 ^ 2))");
    }

    #[test]
    fn unary_minus_sits_between_multiplication_and_power() {
        assert_eq!(parsed("-2 * 3"), "((-2) * 3)");
        assert_eq!(parsed("-2 ^ 2"), "(-(2 ^ 2))");
        assert_eq!(parsed("2 ^ -3 ^ 2"), "(2 ^ (-(3 ^ 2)))");
    }

    #[test]
    fn a_unary_plus_leaves_the_value_alone() {
        assert_eq!(parsed("+-1"), "(-1)");
    }

    #[test]
    fn parentheses_override_the_table() {
        assert_eq!(parsed("(1 + 2) * 3"), "((1 + 2) * 3)");
        assert_eq!(parsed("2 ^ (3 ^ 2)"), "(2 ^ (3 ^ 2))");
    }

    #[test]
    fn calls_take_zero_or_more_comma_separated_arguments() {
        assert_eq!(parsed("pi()"), "pi()");
        assert_eq!(parsed("pow(2, 10)"), "pow(2, 10)");
        assert_eq!(parsed("max(1 + 2, 3 * 4)"), "max((1 + 2), (3 * 4))");
        assert_eq!(parsed("sqrt(abs(-16))"), "sqrt(abs((-16)))");
    }

    #[test]
    fn display_is_readable_again_verbatim() {
        for src in [
            "1 + 2 * 3",
            "-2 ^ 2",
            "2 ^ -3 ^ 2",
            "(1 + 2) * (3 - 4)",
            "max(1, min(2, 3))",
            "pow(x, 2) / -y",
        ] {
            let once = parse(src).unwrap();
            let twice = parse(&once.to_string()).unwrap();
            assert_eq!(once, twice, "源码 {src}");
        }
    }

    #[test]
    fn a_missing_operand_points_at_the_end_of_the_input() {
        let err = parse("1 +").unwrap_err();
        assert_eq!(
            err.to_string(),
            "偏移 3 处应该是一个值，实际看到 输入的末尾"
        );
    }

    #[test]
    fn an_unclosed_parenthesis_is_reported_at_the_end() {
        let err = parse("(1 + 2").unwrap_err();
        assert_eq!(err.to_string(), "偏移 6 处应该是`)`，实际看到 输入的末尾");
    }

    #[test]
    fn trailing_input_is_rejected_instead_of_ignored() {
        let err = parse("1 2").unwrap_err();
        assert_eq!(err.to_string(), "偏移 2 处应该是输入的末尾，实际看到 `2`");
    }

    #[test]
    fn a_missing_comma_in_an_argument_list_is_reported() {
        let err = parse("pow(2 3)").unwrap_err();
        assert_eq!(err.to_string(), "偏移 6 处应该是`,` 或 `)`，实际看到 `3`");
    }

    #[test]
    fn a_leading_operator_that_cannot_start_a_value_is_reported() {
        let err = parse("* 2").unwrap_err();
        assert_eq!(err.to_string(), "偏移 0 处应该是一个值，实际看到 `*`");
    }

    #[test]
    fn nesting_within_the_limit_still_parses() {
        let half = MAX_DEPTH / 2;
        let src = format!("{}1{}", "(".repeat(half), ")".repeat(half));
        assert_eq!(parse(&src).unwrap().to_string(), "1");
    }

    #[test]
    fn nesting_past_the_limit_is_an_error_instead_of_a_crash() {
        let src = format!(
            "{}1{}",
            "(".repeat(MAX_DEPTH + 1),
            ")".repeat(MAX_DEPTH + 1)
        );
        let err = parse(&src).unwrap_err();
        assert_eq!(err.to_string(), "表达式嵌套太深（最多 128 层）");
        assert_eq!(err.offset(), None);
    }

    #[test]
    fn long_left_associative_chains_stay_iterative() {
        // 2000 项、1999 个 `+`：左结合链走的是循环，不会把深度顶到上限。
        let src = vec!["1"; 2000].join(" + ");
        let rendered = parse(&src).unwrap().to_string();
        assert_eq!(rendered.matches(" + ").count(), 1999);
    }
}
