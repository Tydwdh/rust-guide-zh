// stable 上表达「N + 1」的常见退路：用宏把每个具体长度展开成一组 impl
trait Push<T> {
    type Bigger;
    fn push(self, value: T) -> Self::Bigger;
}

macro_rules! impl_push {
    ($($n:literal => $m:literal),* $(,)?) => {
        $(
            impl<T> Push<T> for [T; $n] {
                type Bigger = [T; $m];
                fn push(self, value: T) -> [T; $m] {
                    let mut tail = Some(value);
                    let mut head = self.into_iter();
                    std::array::from_fn(|i| {
                        if i < $n {
                            head.next().unwrap()
                        } else {
                            tail.take().unwrap()
                        }
                    })
                }
            }
        )*
    };
}

// 想要多少长度就展开多少：这是 generic_const_exprs 之前的标准做法
impl_push!(1 => 2, 2 => 3, 3 => 4);

fn main() {
    let a: [i32; 2] = [1, 2];
    let b: [i32; 3] = a.push(3);
    println!("{:?}", b);
}
