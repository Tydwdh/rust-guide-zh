use std::fmt::Display;
use std::ops::Add;

#[derive(Debug, Clone, Copy, PartialEq)]
struct Meters(f64);

// Add<Rhs = Self> 的默认形态：米 + 米
impl Add for Meters {
    type Output = Meters;
    fn add(self, rhs: Meters) -> Meters {
        Meters(self.0 + rhs.0)
    }
}

// 同一类型换掉 Rhs，再实现一次：米 + 数
impl Add<f64> for Meters {
    type Output = Meters;
    fn add(self, rhs: f64) -> Meters {
        Meters(self.0 + rhs)
    }
}

// 每个类型只有一种「报告类型」→ 用关联类型
trait Report {
    type Detail: Display;
    fn report(&self) -> Self::Detail;
}

impl Report for Meters {
    type Detail = String;
    fn report(&self) -> String {
        format!("{:.2} m", self.0)
    }
}

impl Report for i32 {
    // 关联类型不必是新类型
    type Detail = i32;
    fn report(&self) -> i32 {
        self.abs()
    }
}

// 同一个抽象改用泛型参数：一个类型可以有多种报告方式
trait ReportAs<T> {
    fn report_as(&self) -> T;
}

impl ReportAs<String> for Meters {
    fn report_as(&self) -> String {
        format!("{:.2} m", self.0)
    }
}

impl ReportAs<f64> for Meters {
    fn report_as(&self) -> f64 {
        self.0
    }
}

// 声明处写了 `Detail: Display`，函数体里就能直接格式化
fn show<T: Report>(x: &T) -> String {
    format!("{}", x.report())
}

// 关联类型还能在边界里钉住具体值
fn only_string_detail<T: Report<Detail = String>>(x: &T) -> String {
    x.report()
}

fn main() {
    let a = Meters(1.5);

    println!("{:?}", a + Meters(2.0));
    println!("{:?}", a + 5.0);
    println!("{}", show(&a));
    println!("{}", show(&-3));
    println!("{}", only_string_detail(&a));
    println!("{}", ReportAs::<String>::report_as(&a));
    println!("{}", ReportAs::<f64>::report_as(&a));
}
