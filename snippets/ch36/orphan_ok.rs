// 本地类型 + 外部 trait：允许（trait 是外部的，但 T0 是本地类型）
struct Point {
    x: i32,
    y: i32,
}

impl std::fmt::Display for Point {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

// 本地类型出现在 trait 的类型参数位置（T1）也算数；
// 而 T 在 T0 里被 Vec 覆盖，所以这条 impl 不会被「未覆盖参数」挡住
struct Crate;

impl<T> From<Crate> for Vec<T> {
    fn from(_: Crate) -> Self {
        Vec::new()
    }
}

fn main() {
    println!("{}", Point { x: 1, y: 2 });
    let v: Vec<u8> = Vec::from(Crate);
    println!("空向量长度 = {}", v.len());
}
