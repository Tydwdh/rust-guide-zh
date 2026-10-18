// 非消费式 builder：配置方法借 `&mut self` 并返回 `&mut Self` 以便链式调用，
// 终端方法只要 `&self` —— 因为「构造」并不需要拿走配置本身。

#[derive(Debug)]
pub struct ConnBuilder {
    host: String, // 必填项：构造函数就给
    port: u16,
    retries: u8,
}

impl ConnBuilder {
    /// 构造函数只接收「没有它就不成其为一个配置」的数据。
    pub fn new(host: impl Into<String>) -> Self {
        ConnBuilder { host: host.into(), port: 8080, retries: 3 }
    }

    /// 可选项：返回 `&mut Self`，于是可以一行串下去。
    pub fn port(&mut self, port: u16) -> &mut Self {
        self.port = port;
        self
    }

    /// 可选项。
    pub fn retries(&mut self, retries: u8) -> &mut Self {
        self.retries = retries;
        self
    }

    /// 终端方法：只借用配置，不改它，所以签名是 `&self`。
    pub fn connect(&self) -> String {
        format!("{}:{} (重试 {})", self.host, self.port, self.retries)
    }
}

fn main() {
    // 一行式：临时值活到整条语句结束
    println!("{}", ConnBuilder::new("example.com").port(9000).connect());

    // 分步式：条件配置写起来和普通代码一样自然
    let use_tls_port = true;
    let mut b = ConnBuilder::new("example.com");
    if use_tls_port {
        b.port(443);
    }
    b.retries(0);
    println!("{}", b.connect());
}
