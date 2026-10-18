// typestate：把「配置到哪一步了」编码进类型参数，
// 于是「忘了设必填项」不再是运行时错误，而是编译错误。

use std::marker::PhantomData;

// 还没给 host。
pub struct Missing;
// 已经给过 host。
pub struct Present;

pub struct ConnBuilder<State> {
    host: Option<String>,
    port: u16,
    retries: u8,
    // 状态只是个标记，不占空间
    _state: PhantomData<State>,
}

impl ConnBuilder<Missing> {
    pub fn new() -> Self {
        ConnBuilder { host: None, port: 8080, retries: 3, _state: PhantomData }
    }

    /// 设置必填项：类型参数从 `Missing` 走到 `Present`。
    pub fn host(self, host: impl Into<String>) -> ConnBuilder<Present> {
        ConnBuilder {
            host: Some(host.into()),
            port: self.port,
            retries: self.retries,
            _state: PhantomData,
        }
    }
}

// 两个状态都能用的可选项。
impl<State> ConnBuilder<State> {
    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    pub fn retries(mut self, retries: u8) -> Self {
        self.retries = retries;
        self
    }
}

// `build` 只挂在 `Present` 上：没设 host 就调不到它。
impl ConnBuilder<Present> {
    pub fn build(&self) -> String {
        format!("{}:{} (重试 {})", self.host.as_deref().unwrap_or(""), self.port, self.retries)
    }
}

impl Default for ConnBuilder<Missing> {
    fn default() -> Self {
        Self::new()
    }
}

fn main() {
    println!("{}", ConnBuilder::new().host("example.com").build());

    // 可选项可以放在必填项之前或之后，顺序自由
    println!("{}", ConnBuilder::new().port(9000).host("example.com").retries(0).build());

    // 下面这行编译不过：`build` 不在 `ConnBuilder<Missing>` 上
    // ConnBuilder::new().build();
}
