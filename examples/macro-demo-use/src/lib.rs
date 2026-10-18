//! 第 28 章过程宏的**消费端** crate。
//!
//! `macro-demo` 是 proc-macro crate，只能导出宏、不能导出 trait，
//! 所以 `Describe` trait 与全部测试住在这一侧。serde 把 `serde`（trait）与
//! `serde_derive`（宏）拆成两个 crate，是同一条约束的结果。

pub use macro_demo::{Describe, repeat, sum};

/// `#[derive(Describe)]` 生成的 impl 所实现的 trait。
pub trait Describe {
    /// 描述自己，形如 `Point { x = 1, y = 2 }`。
    fn describe(&self) -> String;
}

/// 消费者只写 `#[derive(Describe)]`，方法由宏展开出来。
#[derive(Describe)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

/// 空的花括号字段表也认，输出就是结构体名。
#[derive(Describe)]
pub struct Empty {}

/// 字段类型不限，能 `Debug` 就行。
#[derive(Describe)]
pub struct Payload {
    pub name: String,
    pub sizes: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn describe_lists_fields_in_declaration_order() {
        let point = Point { x: 1, y: 2 };
        assert_eq!(point.describe(), "Point { x = 1, y = 2 }");
    }

    #[test]
    fn describe_handles_empty_struct() {
        assert_eq!(Empty {}.describe(), "Empty");
    }

    #[test]
    fn describe_works_behind_a_generic_bound() {
        fn render<T: Describe>(value: &T) -> String {
            value.describe()
        }
        let payload = Payload {
            name: "盒".to_owned(),
            sizes: vec![1, 2],
        };
        assert_eq!(render(&payload), r#"Payload { name = "盒", sizes = [1, 2] }"#);
    }

    #[repeat(3)]
    const SCALE: i32 = 2;

    #[repeat(2)]
    fn greet() -> &'static str {
        "hi"
    }

    #[test]
    fn repeat_copies_the_item_and_keeps_the_original() {
        assert_eq!([SCALE_0, SCALE_1, SCALE_2], [2, 2, 2]);
        assert_eq!(SCALE, 2);
        assert_eq!((greet_0(), greet_1(), greet()), ("hi", "hi", "hi"));
    }

    const TOTAL: i64 = sum!(10, 20, 30);

    #[test]
    fn sum_is_evaluated_during_expansion() {
        assert_eq!(sum!(1, 2, 3), 6);
        assert_eq!(TOTAL, 60);
        let typed: i32 = sum!(1, 2); // 无后缀字面量：类型由上下文推断
        assert_eq!(typed, 3);
    }
}
