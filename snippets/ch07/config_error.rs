// 一个手写的错误类型：解析端口号。
// 完整程序：用 `rustc --edition 2024 config_error.rs` 编译，再运行生成的程序。
use std::error::Error;
use std::fmt;
use std::num::ParseIntError;

#[derive(Debug)]
enum ConfigError {
    Empty,
    BadNumber(ParseIntError),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::Empty => write!(f, "端口不能为空"),
            // 细节留给 source()，Display 只讲外层发生了什么
            ConfigError::BadNumber(_) => write!(f, "端口不是合法数字"),
        }
    }
}

impl Error for ConfigError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ConfigError::Empty => None,
            ConfigError::BadNumber(e) => Some(e),
        }
    }
}

impl From<ParseIntError> for ConfigError {
    fn from(e: ParseIntError) -> Self {
        ConfigError::BadNumber(e)
    }
}

fn parse_port(s: &str) -> Result<u16, ConfigError> {
    let s = s.trim();
    if s.is_empty() {
        return Err(ConfigError::Empty);
    }
    Ok(s.parse()?) // 这里的 Err 会自动经 From 转成 ConfigError
}

fn main() {
    for input in [" 8080 ", "", "80a0"] {
        match parse_port(input) {
            Ok(port) => println!("{input:?} -> Ok({port})"),
            Err(e) => {
                println!("{input:?} -> Err");
                println!("  Display: {e}");
                println!("  Debug:   {e:?}");
                let mut source = e.source();
                while let Some(s) = source {
                    println!("  source:  {s}");
                    source = s.source();
                }
            }
        }
    }
}
