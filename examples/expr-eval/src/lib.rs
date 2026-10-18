//! 只用标准库的表达式求值器：字符串进，`f64` 出。
//!
//! 一条表达式走三步：词法分析（[`Lexer`]）→ 语法分析（[`parse`]）→ 求值（[`Expr::eval`]）。
//! 语法有数字、变量、`+ - * / ^`、括号、一元负号，以及 `abs`、`sqrt`、`pow`、`min`、`max` 五个内置函数。
//!
//! # Examples
//!
//! ```
//! use expr_eval::{Bindings, eval_str};
//!
//! let env = Bindings::new().with("x", 3.0);
//! assert_eq!(eval_str("x^2 + 1", &env).unwrap(), 10.0);
//! assert_eq!(eval_str("sqrt(abs(-16))", &env).unwrap(), 4.0);
//! assert!(eval_str("1 / 0", &env).is_err());
//! ```
//!
//! 公开面就下面那几行 `pub use`：模块本身是私有的，想让外界看到什么，只能在这个文件里加。

#![warn(missing_docs)]
#![forbid(unsafe_code)]

mod ast;
mod error;
mod eval;
mod lexer;
mod parser;

pub use ast::{BinOp, Expr};
pub use error::{Error, Result};
pub use eval::{Bindings, Env, call_builtin, eval_str};
pub use lexer::{Lexer, Token, TokenKind, tokenize};
pub use parser::parse;
