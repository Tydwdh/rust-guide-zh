//! 求值：遍历语法树，把每个节点变成一个 `f64`。

use std::collections::HashMap;

use crate::ast::{BinOp, Expr};
use crate::error::{Error, Result};
use crate::parser::parse;

/// 求值环境：变量从哪儿来、函数怎么调。
///
/// 这是本 crate 留给外面的扩展点：实现它就能让表达式看到自己的数据。
/// 两个方法都接收 `&str` 而不是 `String` —— 查询不该逼调用者分配内存。
pub trait Env {
    /// 查一个变量的值；没绑定时返回 `None`。
    fn var(&self, name: &str) -> Option<f64>;

    /// 调一个函数；默认实现只认 [`call_builtin`] 里那几个。
    ///
    /// 自己实现时，认不出的名字记得退回 [`call_builtin`]，
    /// 否则内置函数会跟着一起失效。
    fn call(&self, name: &str, args: &[f64]) -> Result<f64> {
        call_builtin(name, args)
    }
}

/// 一张变量表：变量存在 `HashMap` 里，函数走内置的那五个。
#[derive(Debug, Clone, Default)]
pub struct Bindings {
    vars: HashMap<String, f64>,
}

impl Bindings {
    /// 建一张空表。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 绑一个变量，返回更新后的表，方便连着写。
    ///
    /// 返回值标了 `#[must_use]`：写成 `Bindings::new().with("x", 1.0);` 的话，
    /// 丢掉的是整张表，编译器会当场警告（第 18 章）。
    #[must_use = "with 返回更新后的绑定表，丢掉它等于没绑定"]
    pub fn with(mut self, name: impl Into<String>, value: f64) -> Self {
        self.insert(name, value);
        self
    }

    /// 绑一个变量；已经有了就覆盖。
    pub fn insert(&mut self, name: impl Into<String>, value: f64) {
        self.vars.insert(name.into(), value);
    }
}

impl From<HashMap<String, f64>> for Bindings {
    fn from(vars: HashMap<String, f64>) -> Self {
        Bindings { vars }
    }
}

impl Env for Bindings {
    fn var(&self, name: &str) -> Option<f64> {
        self.vars.get(name).copied()
    }
}

/// 调用一个内置函数。
///
/// `abs`/`sqrt` 收一个参数，`pow`/`min`/`max` 收两个；名字没听过是
/// [`Error::UnknownFunction`]，个数不对是 [`Error::WrongArity`]。
///
/// # Examples
///
/// ```
/// use expr_eval::call_builtin;
///
/// assert_eq!(call_builtin("max", &[1.0, 2.0]).unwrap(), 2.0);
/// assert!(call_builtin("pow", &[2.0]).is_err());
/// ```
pub fn call_builtin(name: &str, args: &[f64]) -> Result<f64> {
    match (name, args) {
        ("abs", [x]) => Ok(x.abs()),
        ("sqrt", [x]) => finite(format!("sqrt({x})"), x.sqrt()),
        ("pow", [x, y]) => finite(format!("pow({x}, {y})"), x.powf(*y)),
        ("min", [x, y]) => Ok(x.min(*y)),
        ("max", [x, y]) => Ok(x.max(*y)),
        ("abs" | "sqrt", _) => Err(Error::WrongArity {
            name: name.to_owned(),
            expected: 1,
            got: args.len(),
        }),
        ("pow" | "min" | "max", _) => Err(Error::WrongArity {
            name: name.to_owned(),
            expected: 2,
            got: args.len(),
        }),
        _ => Err(Error::UnknownFunction {
            name: name.to_owned(),
        }),
    }
}

/// 结果不是有限数就报错：`NaN` 和 `±inf` 都不是能给用户看的答案。
fn finite(expr: String, value: f64) -> Result<f64> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(Error::NotFinite { expr })
    }
}

impl BinOp {
    /// 算一下两个操作数；结果不是有限数就报 [`Error::NotFinite`]。
    fn apply(self, lhs: f64, rhs: f64) -> Result<f64> {
        let value = match self {
            BinOp::Add => lhs + rhs,
            BinOp::Sub => lhs - rhs,
            BinOp::Mul => lhs * rhs,
            BinOp::Div => lhs / rhs,
            BinOp::Pow => lhs.powf(rhs),
        };
        finite(format!("{lhs} {} {rhs}", self.symbol()), value)
    }
}

impl Expr {
    /// 在 `env` 里求出这个表达式的值。
    ///
    /// 每遇到一个变量就去 [`Env::var`] 问一次，每遇到一个调用就去 [`Env::call`] 问一次；
    /// 求值本身不改变任何东西，`&self` 和 `&dyn Env` 都是共享借用。
    ///
    /// # Examples
    ///
    /// ```
    /// use expr_eval::{Bindings, parse};
    ///
    /// let env = Bindings::new().with("x", 4.0);
    /// assert_eq!(parse("sqrt(x)").unwrap().eval(&env).unwrap(), 2.0);
    /// ```
    pub fn eval(&self, env: &dyn Env) -> Result<f64> {
        match self {
            Expr::Num(value) => Ok(*value),
            Expr::Var(name) => env
                .var(name)
                .ok_or_else(|| Error::UnknownName { name: name.clone() }),
            Expr::Neg(inner) => Ok(-inner.eval(env)?),
            Expr::Binary { op, lhs, rhs } => op.apply(lhs.eval(env)?, rhs.eval(env)?),
            Expr::Call { name, args } => {
                let mut values = Vec::with_capacity(args.len());
                for arg in args {
                    values.push(arg.eval(env)?);
                }
                env.call(name, &values)
            }
        }
    }
}

/// 解析并求值一条表达式：[`parse`] 加 [`Expr::eval`] 的便捷组合。
///
/// # Examples
///
/// ```
/// use expr_eval::{Bindings, eval_str};
///
/// let env = Bindings::new();
/// assert_eq!(eval_str("2 * (3 + 4)", &env).unwrap(), 14.0);
/// ```
pub fn eval_str(src: &str, env: &dyn Env) -> Result<f64> {
    parse(src)?.eval(env)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eval(src: &str) -> Result<f64> {
        eval_str(src, &Bindings::new())
    }

    #[test]
    fn arithmetic_follows_the_usual_rules() {
        assert_eq!(eval("1 + 2 * 3").unwrap(), 7.0);
        assert_eq!(eval("(1 + 2) * 3").unwrap(), 9.0);
        assert_eq!(eval("2 ^ 10").unwrap(), 1024.0);
        assert_eq!(eval("2 ^ 3 ^ 2").unwrap(), 512.0);
        assert_eq!(eval("-2 ^ 2").unwrap(), -4.0);
        assert_eq!(eval("0.1 + 0.2").unwrap(), 0.30000000000000004);
    }

    #[test]
    fn variables_come_from_the_environment() {
        let env = Bindings::new().with("x", 3.0).with("y", 4.0);
        assert_eq!(eval_str("x^2 + y^2", &env).unwrap(), 25.0);
    }

    #[test]
    fn an_unbound_variable_is_an_error_not_a_zero() {
        let err = eval("x + 1").unwrap_err();
        assert_eq!(err.to_string(), "变量 `x` 没有绑定");
        assert_eq!(err.offset(), None);
    }

    #[test]
    fn builtin_functions_are_available_by_default() {
        assert_eq!(eval("abs(-3) + sqrt(16)").unwrap(), 7.0);
        assert_eq!(eval("pow(2, 8)").unwrap(), 256.0);
        assert_eq!(eval("max(1, min(2, 3))").unwrap(), 2.0);
    }

    #[test]
    fn calling_a_name_that_is_not_a_function_is_an_error() {
        let err = eval("nope(1)").unwrap_err();
        assert_eq!(err.to_string(), "没有名为 `nope` 的函数");
    }

    #[test]
    fn the_wrong_number_of_arguments_is_an_error() {
        let err = eval("sqrt(1, 2)").unwrap_err();
        assert_eq!(err.to_string(), "函数 `sqrt` 需要 1 个参数，实际给了 2 个");
        let err = eval("pow(2)").unwrap_err();
        assert_eq!(err.to_string(), "函数 `pow` 需要 2 个参数，实际给了 1 个");
    }

    #[test]
    fn division_by_zero_and_other_non_finite_results_are_errors() {
        assert_eq!(
            eval("1 / 0").unwrap_err().to_string(),
            "1 / 0 的结果不是有限数（除零、负数开方、上溢都会这样）"
        );
        assert!(eval("0 / 0").is_err());
        assert!(eval("sqrt(-1)").is_err());
        assert!(eval("2 ^ 1024").is_err());
    }

    #[test]
    fn a_custom_environment_can_override_call_and_fall_back_to_the_builtins() {
        struct Doubling;

        impl Env for Doubling {
            fn var(&self, name: &str) -> Option<f64> {
                (name == "answer").then_some(42.0)
            }

            fn call(&self, name: &str, args: &[f64]) -> Result<f64> {
                match name {
                    "double" => Ok(args.first().copied().unwrap_or(0.0) * 2.0),
                    other => call_builtin(other, args),
                }
            }
        }

        assert_eq!(eval_str("double(answer)", &Doubling).unwrap(), 84.0);
        assert_eq!(eval_str("abs(-1)", &Doubling).unwrap(), 1.0);
    }

    #[test]
    fn a_hash_map_of_floats_is_already_an_environment() {
        let mut vars = HashMap::new();
        vars.insert("x".to_owned(), 5.0);
        let env: Bindings = vars.into();
        assert_eq!(eval_str("x * 2", &env).unwrap(), 10.0);
    }
}
