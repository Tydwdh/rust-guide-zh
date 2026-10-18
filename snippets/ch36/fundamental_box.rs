// fundamental 类型构造器：Box<T> 里的 T 不被「覆盖」，
// 但 Box<本地类型> 本身算本地类型
struct Point(i32);

impl std::ops::Index<usize> for Box<Point> {
    type Output = i32;
    fn index(&self, _: usize) -> &i32 {
        &self.0
    }
}

fn main() {
    let p = Box::new(Point(7));
    println!("p[0] = {}", p[0]);
}
