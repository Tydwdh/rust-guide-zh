//! 语法树：节点拥有自己的数据（`String`、`Box`、`Vec`），所以树能活得比源串长。

use std::fmt;

/// 中缀操作符。
///
/// 操作符和它的优先级是同一件事，所以绑定力也挂在这里。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum BinOp {
    /// `+`
    Add,
    /// `-`
    Sub,
    /// `*`
    Mul,
    /// `/`
    Div,
    /// `^`（幂）
    Pow,
}

impl BinOp {
    /// 这个操作符的（左绑定力, 右绑定力）。
    ///
    /// 左绑定力决定「还进不进循环」，右绑定力决定「右操作数解析到哪一档停」。
    /// `+ -` 是 (1, 2)、`* /` 是 (3, 4)：右比左大 1 就是左结合。
    /// `^` 是 (7, 7)：右操作数带着同样的绑定力回来，于是右结合，`2^3^2` = `2^(3^2)`。
    pub(crate) fn binding_power(self) -> (u8, u8) {
        match self {
            BinOp::Add | BinOp::Sub => (1, 2),
            BinOp::Mul | BinOp::Div => (3, 4),
            BinOp::Pow => (7, 7),
        }
    }

    /// 源串里的写法；[`fmt::Display`] 用它拼括号。
    pub(crate) fn symbol(self) -> &'static str {
        match self {
            BinOp::Add => "+",
            BinOp::Sub => "-",
            BinOp::Mul => "*",
            BinOp::Div => "/",
            BinOp::Pow => "^",
        }
    }
}

/// 一个表达式节点。
///
/// 子节点装在 `Box` 里：类型递归包含自己，`Box` 让每个节点的大小固定（第 17 章）。
/// 变量名与函数名是 `String` —— 树要活得比源串长，理由见词法那一步的选择。
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Expr {
    /// 字面量。
    Num(f64),
    /// 变量。
    Var(String),
    /// 一元负号。
    Neg(Box<Expr>),
    /// 中缀运算。
    Binary {
        /// 操作符。
        op: BinOp,
        /// 左操作数。
        lhs: Box<Expr>,
        /// 右操作数。
        rhs: Box<Expr>,
    },
    /// 函数调用。
    Call {
        /// 函数名。
        name: String,
        /// 实参，按书写顺序。
        args: Vec<Expr>,
    },
}

impl fmt::Display for Expr {
    /// 打印**完全括号化**的形式：每个运算都套一层括号。
    ///
    /// 这样打印出来的东西可以直接读回来（`parse(&ast.to_string()) == ast`），
    /// 于是优先级测试只要比较字符串，不必把文法再抄一遍。
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expr::Num(value) => write!(f, "{value}"),
            Expr::Var(name) => f.write_str(name),
            Expr::Neg(inner) => write!(f, "(-{inner})"),
            Expr::Binary { op, lhs, rhs } => write!(f, "({lhs} {} {rhs})", op.symbol()),
            Expr::Call { name, args } => {
                let rendered: Vec<String> = args.iter().map(ToString::to_string).collect();
                write!(f, "{name}({})", rendered.join(", "))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn num(value: f64) -> Box<Expr> {
        Box::new(Expr::Num(value))
    }

    #[test]
    fn display_wraps_every_operation_in_parentheses() {
        let sum = Expr::Binary {
            op: BinOp::Add,
            lhs: num(1.0),
            rhs: Box::new(Expr::Binary {
                op: BinOp::Mul,
                lhs: num(2.0),
                rhs: num(3.0),
            }),
        };
        assert_eq!(sum.to_string(), "(1 + (2 * 3))");
    }

    #[test]
    fn display_puts_the_minus_inside_its_own_parentheses() {
        let negated = Expr::Neg(Box::new(Expr::Binary {
            op: BinOp::Pow,
            lhs: num(2.0),
            rhs: num(2.0),
        }));
        assert_eq!(negated.to_string(), "(-(2 ^ 2))");
    }

    #[test]
    fn display_separates_call_arguments_with_a_comma_and_a_space() {
        let call = Expr::Call {
            name: "pow".to_owned(),
            args: vec![Expr::Num(2.0), Expr::Num(10.0)],
        };
        assert_eq!(call.to_string(), "pow(2, 10)");
    }
}
