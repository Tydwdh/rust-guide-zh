// 一个函数，服务所有满足 PartialOrd 的类型。
fn largest<T: PartialOrd>(list: &[T]) -> &T {
    let mut largest = &list[0];
    for item in list {
        if item > largest {
            largest = item;
        }
    }
    largest
}

fn main() {
    let numbers = vec![34, 50, 25, 100, 65];
    println!("整数最大值: {}", largest(&numbers));

    let chars = vec!['y', 'm', 'a', 'q'];
    println!("字符最大值: {}", largest(&chars));

    let words = vec!["pear", "apple", "zebra"];
    println!("字典序最大: {}", largest(&words));
}
