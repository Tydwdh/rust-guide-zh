use std::any::Any;
use std::mem::{size_of, size_of_val};

trait Shape {
    fn name(&self) -> &'static str;
    fn area(&self) -> f64;
}

struct Circle {
    radius: f64,
}

impl Shape for Circle {
    fn name(&self) -> &'static str {
        "圆"
    }
    fn area(&self) -> f64 {
        std::f64::consts::PI * self.radius * self.radius
    }
}

struct Square {
    side: f64,
}

impl Shape for Square {
    fn name(&self) -> &'static str {
        "正方形"
    }
    fn area(&self) -> f64 {
        self.side * self.side
    }
}

// 零大小类型：没有字段，值本身不占字节。
struct Dot;

impl Shape for Dot {
    fn name(&self) -> &'static str {
        "点"
    }
    fn area(&self) -> f64 {
        0.0
    }
}

fn main() {
    // 异构集合：三种互不相关的类型，装在同一个 Vec 里。
    let shapes: Vec<Box<dyn Shape>> = vec![
        Box::new(Circle { radius: 1.0 }),
        Box::new(Square { side: 2.0 }),
        Box::new(Dot),
    ];
    for s in &shapes {
        println!("{}：面积 {:.2}", s.name(), s.area());
    }

    println!(
        "&dyn Shape = {} 字节，Box<dyn Shape> = {} 字节",
        size_of::<&dyn Shape>(),
        size_of::<Box<dyn Shape>>()
    );

    // 同一个静态类型 &dyn Shape，指向的对象大小却不同：大小只能问 vtable。
    let a: &dyn Shape = &Circle { radius: 1.0 };
    let b: &dyn Shape = &Dot;
    println!("a 指向的对象 {} 字节，b 指向的对象 {} 字节", size_of_val(a), size_of_val(b));

    // 需要按具体类型做事时：先擦成 dyn Any，再向下转型。
    let any: Box<dyn Any> = Box::new(Circle { radius: 2.0 });
    println!("是 Circle 吗？{}", any.is::<Circle>());
    if let Some(c) = any.downcast_ref::<Circle>() {
        println!("找回 Circle，半径 {}", c.radius);
    }
}
