use std::error::Error;
use std::fmt;
use std::fs;
use std::num::ParseIntError;

#[derive(Debug)]
struct MissingKey {
    key: String,
}

impl fmt::Display for MissingKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "缺少配置项 `{}`", self.key)
    }
}

impl Error for MissingKey {}

// 三种错误装进同一个 Box：io::Error、ParseIntError、自定义类型。
fn port_from(path: &str) -> Result<u16, Box<dyn Error>> {
    let text = fs::read_to_string(path)?;
    let port: u16 = text.trim().parse()?;
    if port == 0 {
        return Err(Box::new(MissingKey {
            key: String::from("port"),
        }));
    }
    Ok(port)
}

// 要跨线程传递就换成带 auto trait 的写法。
fn port_from_send(path: &str) -> Result<u16, Box<dyn Error + Send + Sync>> {
    let text = fs::read_to_string(path)?;
    Ok(text.trim().parse()?)
}

fn main() {
    match port_from("no-such-file.toml") {
        Ok(p) => println!("端口 {p}"),
        Err(e) => {
            println!("Display：{e}");
            if let Some(io) = e.downcast_ref::<std::io::Error>() {
                println!("原来是 io::Error，kind = {:?}", io.kind());
            }
        }
    }

    if let Err(e) = port_from_send("no-such-file.toml") {
        println!("Send + Sync 版：{e}");
    }

    // From<E: Error> for Box<dyn Error> 让 `.into()` 也能直接装箱。
    let parsed = "abc".parse::<u16>().unwrap_err();
    let boxed: Box<dyn Error> = parsed.into();
    println!("装箱后：{boxed}");
    println!(
        "还能还原成 ParseIntError 吗？{}",
        boxed.downcast_ref::<ParseIntError>().is_some()
    );
}
