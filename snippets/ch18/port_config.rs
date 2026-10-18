// 按端口读配置的小库：演示「错误类型是公开 API 的一部分」。
//
// 上半个文件是库作者要写的东西（类型、构造函数、`From`、`source()`），
// 下半个文件是调用者拿它做什么（匹配、遍历错误链、交给应用层呈现）。

use std::error::Error;
use std::fmt;
use std::io;
use std::num::ParseIntError;

// 库对外公开的错误类型。
//
// `#[non_exhaustive]`：外部 crate 必须带通配分支匹配，
// 于是将来加变体不算破坏性变更。
#[non_exhaustive]
#[derive(Debug)]
pub enum PortError {
    /// 配置里没有端口这一项。
    Missing,
    /// 端口不是数字：原文与底层解析错误都留着。
    BadNumber { raw: String, cause: ParseIntError },
    /// 读配置文件失败。
    Io(io::Error),
}

impl PortError {
    /// 显式构造函数。
    ///
    /// 这里故意不写 `From<ParseIntError>`：光凭一个 `ParseIntError`
    /// 说不清是哪个字段、原文是什么，那样的转换是有损的。
    pub fn bad_number(raw: impl Into<String>, cause: ParseIntError) -> Self {
        PortError::BadNumber { raw: raw.into(), cause }
    }
}

impl fmt::Display for PortError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // 只写「外层发生了什么」；底层的细节留给 source()。
            PortError::Missing => write!(f, "配置里缺少端口"),
            PortError::BadNumber { raw, .. } => write!(f, "端口 {raw:?} 不是数字"),
            PortError::Io(_) => write!(f, "读取配置文件失败"),
        }
    }
}

impl Error for PortError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            PortError::Missing => None,
            PortError::BadNumber { cause, .. } => Some(cause),
            PortError::Io(e) => Some(e),
        }
    }
}

impl From<io::Error> for PortError {
    /// 无歧义：`io::Error` 在这个库里只可能来自「读文件」。
    fn from(e: io::Error) -> Self {
        PortError::Io(e)
    }
}

#[derive(Debug)]
pub struct Config {
    pub port: u16,
}

// 从 `port = 8080` 这样的一行文本里读出端口。
pub fn parse_config(text: &str) -> Result<Config, PortError> {
    let line = text
        .lines()
        .find(|l| l.trim_start().starts_with("port"))
        .ok_or(PortError::Missing)?;
    let raw = line
        .split_once('=')
        .map(|(_, v)| v.trim())
        .ok_or(PortError::Missing)?;
    // 不是 `?` + From，而是显式构造函数：把原文一起带上。
    let port: u16 = raw
        .parse()
        .map_err(|cause| PortError::bad_number(raw, cause))?;
    Ok(Config { port })
}

// 从文件读配置：这里 `?` 走的是 `From<io::Error>`。
pub fn load_config(path: &str) -> Result<Config, PortError> {
    let text = std::fs::read_to_string(path)?;
    parse_config(&text)
}

// 顺着 `source()` 逐层打印错误链 —— 这是应用层才该做的事。
fn print_chain(err: &dyn Error) {
    let mut depth = 0;
    let mut cur: Option<&dyn Error> = Some(err);
    while let Some(e) = cur {
        let indent = "  ".repeat(depth);
        if depth == 0 {
            println!("{indent}{e}");
        } else {
            println!("{indent}└─ {e}");
        }
        cur = e.source();
        depth += 1;
    }
}

// 编译期检查：公开错误类型必须能跨线程搬运。
fn assert_send_sync_static<T: Send + Sync + 'static>() {}

fn main() {
    assert_send_sync_static::<PortError>();

    for text in ["", "port = 80a0", "port = 8080"] {
        match parse_config(text) {
            Ok(cfg) => println!("{text:?} -> Ok(port = {})", cfg.port),
            Err(e) => {
                println!("{text:?} -> Err");
                print_chain(&e);
            }
        }
    }

    // 库不打印、不 exit：把错误交回给上层，由应用决定怎么呈现。
    match load_config("不存在的文件.toml") {
        Ok(_) => unreachable!(),
        Err(e) => {
            let boxed: Box<dyn Error> = Box::new(e);
            println!("上层拿到 Box<dyn Error>：{boxed}");
            println!("底层原因还在：source().is_some() = {}", boxed.source().is_some());
        }
    }
}
