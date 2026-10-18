// 想「先写通用实现，再为某个具体类型特化」在 stable 上做不到，
// 退路之一：把类型判断推迟到运行期（要求 T: 'static）
trait Describe {
    fn describe(&self) -> String;
}

impl<T: std::any::Any> Describe for T {
    fn describe(&self) -> String {
        match (self as &dyn std::any::Any).downcast_ref::<String>() {
            Some(s) => format!("String，{} 字节", s.len()),
            None => String::from("其他类型"),
        }
    }
}

fn main() {
    println!("{}", 42.describe());
    println!("{}", String::from("hey").describe());
}
