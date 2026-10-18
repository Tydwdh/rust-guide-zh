use std::fmt;

// derive 让编译器按字段逐个生成实现。
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
struct Version {
    major: u32,
    minor: u32,
}

// Display 不能 derive：怎么显示是人的决定。
impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

fn main() {
    let a = Version { major: 1, minor: 2 };
    let b = Version { major: 1, minor: 10 };

    // PartialOrd 带来比较运算符；derive 按字段声明顺序逐字段比较。
    println!("{a} < {b}: {}", a < b);

    // PartialEq 带来 == 与 assert_eq!。
    assert_eq!(a, Version { major: 1, minor: 2 });

    // Copy：赋值是复制，a 仍然可用。
    let c = a;
    println!("{a} 和 {c} 都还在");

    // Debug：{:?} 面向开发者，{:#?} 带缩进。
    println!("{a:?} / {a:#?}");
}
