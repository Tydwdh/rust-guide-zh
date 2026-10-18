// 扩展 trait：trait 定义在本 crate，
// 所以可以给它补在外部类型 Vec<T> 上。
trait VecExt<T> {
    fn second(&self) -> Option<&T>;

    // 故意与 Vec 的固有方法同名
    fn len(&self) -> usize;
}

impl<T> VecExt<T> for Vec<T> {
    fn second(&self) -> Option<&T> {
        self.get(1)
    }

    fn len(&self) -> usize {
        999
    }
}

fn main() {
    let v = vec![10, 20, 30];

    println!("{:?}", v.second());
    // 同名时固有方法优先
    println!("固有 len: {}", v.len());
    // 想调扩展 trait 的，用全限定写法
    println!("扩展 len: {}", VecExt::len(&v));
}
