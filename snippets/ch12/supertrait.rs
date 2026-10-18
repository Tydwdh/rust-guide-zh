// 父 trait：Circle 的实现者必须先实现 Shape
trait Shape {
    fn area(&self) -> f64;
}

trait Circle: Shape {
    // 默认实现里可以直接调用父 trait 的方法
    fn radius(&self) -> f64 {
        (self.area() / std::f64::consts::PI).sqrt()
    }
}

struct UnitCircle;

// 两个 impl 各写各的：父 trait 不会被「继承」过来
impl Shape for UnitCircle {
    fn area(&self) -> f64 {
        std::f64::consts::PI
    }
}

impl Circle for UnitCircle {}

// 边界只写 Circle，也能调用 Shape 的 area
fn describe<C: Circle>(c: &C) -> String {
    format!("area = {:.4}, radius = {:.4}", c.area(), c.radius())
}

fn main() {
    println!("{}", describe(&UnitCircle));
}
