#[derive(Debug, PartialEq)]
struct Celsius(f64);

#[derive(Debug, PartialEq)]
struct Fahrenheit(f64);

// 只实现 From，就同时得到了 Into。
impl From<Celsius> for Fahrenheit {
    fn from(c: Celsius) -> Self {
        Fahrenheit(c.0 * 9.0 / 5.0 + 32.0)
    }
}

// 边界写 Into<Fahrenheit>：任何能转成华氏度的东西都收。
fn report<T: Into<Fahrenheit>>(t: T) -> String {
    let f: Fahrenheit = t.into();
    format!("{:.1} °F", f.0)
}

fn main() {
    // 两种写法都能用：
    let f = Fahrenheit::from(Celsius(25.0));
    let g: Fahrenheit = Celsius(25.0).into();
    assert_eq!(f, g);
    println!("25 °C = {:.1} °F", f.0);

    println!("{}", report(Celsius(0.0)));
    println!("{}", report(Celsius(100.0)));
}
