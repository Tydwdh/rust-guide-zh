//! 薄 CLI：把命令行上的表达式交给库，把结果或错误打到屏幕上。
//!
//! 这里只干三件事：解析参数、读出源串、打印结果或带位置的错误。
//! 表达式本身的一切（词法、语法、求值）全在 `expr_eval` 库里 —— 这是「库 + 薄壳」的分工。

use std::collections::HashMap;
use std::fmt;
use std::io::{self, Read};
use std::process::ExitCode;

use expr_eval::{Bindings, Error as EvalError, eval_str, parse};

/// CLI 自己的错误：库的错误 + 本进程的 I/O + 用法。
///
/// 库错误和应用错误分开是第 18 章的规矩：库只说「表达式错在哪儿」，
/// 「怎么讲给敲命令的人听、用什么退出码」是应用的事。
#[derive(Debug)]
enum CliError {
    /// 读写标准输入失败。
    Io(io::Error),
    /// 表达式本身有问题（词法 / 语法 / 求值）。
    Eval(EvalError),
    /// 命令行没写好。
    Usage(String),
}

impl From<io::Error> for CliError {
    fn from(err: io::Error) -> Self {
        CliError::Io(err)
    }
}

impl From<EvalError> for CliError {
    fn from(err: EvalError) -> Self {
        CliError::Eval(err)
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CliError::Io(err) => write!(f, "读写标准输入失败：{err}"),
            CliError::Eval(err) => write!(f, "{err}"),
            CliError::Usage(msg) => write!(f, "参数错误：{msg}"),
        }
    }
}

const HELP: &str = "\
eval —— 算一条表达式

用法：
    eval [选项] <表达式>
    eval [选项] -            从标准输入读表达式

选项：
    --ast               只打印语法树，不求值
    --var <名字>=<值>    绑定一个变量，可以写多次
    --help, -h          打印这份说明

表达式：
    数字、变量、+ - * / ^、括号、一元正负号
    函数：abs(x)  sqrt(x)  pow(x, y)  min(x, y)  max(x, y)

退出码：
    0  算出来了
    2  参数或表达式有问题（错误信息写到标准错误）
";

#[derive(Default)]
struct Options {
    /// `--ast`：只打印语法树。
    ast: bool,
    /// `--help`：打印用法。
    help: bool,
    /// `-`：表达式从标准输入读。
    stdin: bool,
    /// `--var` 攒下来的绑定。
    vars: HashMap<String, f64>,
    /// 表达式的各个参数，最后用空格拼起来。
    expr: Vec<String>,
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    run(&args)
}

fn run(args: &[String]) -> ExitCode {
    let opts = match parse_args(args) {
        Ok(opts) => opts,
        Err(err) => return fail(&err, None),
    };
    if opts.help {
        print!("{HELP}");
        return ExitCode::SUCCESS;
    }
    if opts.stdin && !opts.expr.is_empty() {
        return fail(&CliError::Usage("`-` 和表达式只能给一个".to_owned()), None);
    }
    let src = match read_source(&opts) {
        Ok(src) => src,
        Err(err) => return fail(&err, None),
    };
    if src.trim().is_empty() {
        return fail(&CliError::Usage("没给表达式".to_owned()), None);
    }

    let env: Bindings = opts.vars.into();
    let outcome = if opts.ast {
        parse(&src).map(|expr| expr.to_string())
    } else {
        eval_str(&src, &env).map(|value| value.to_string())
    };
    match outcome {
        Ok(text) => {
            println!("{text}");
            ExitCode::SUCCESS
        }
        Err(err) => fail(&CliError::from(err), Some(&src)),
    }
}

/// 选项只认「表达式之前」的位置；第一个不是选项的参数之后全是表达式文本。
fn parse_args(args: &[String]) -> Result<Options, CliError> {
    let mut opts = Options::default();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "-h" | "--help" => {
                opts.help = true;
                return Ok(opts);
            }
            "--ast" => {
                opts.ast = true;
                index += 1;
            }
            "--var" => {
                let Some(spec) = args.get(index + 1) else {
                    return Err(CliError::Usage("`--var` 后面要跟 `名字=值`".to_owned()));
                };
                let Some((name, value)) = spec.split_once('=') else {
                    return Err(CliError::Usage(format!("`{spec}` 不是 `名字=值` 的形式")));
                };
                let value: f64 = value
                    .parse()
                    .map_err(|_| CliError::Usage(format!("`{value}` 不是数字")))?;
                opts.vars.insert(name.to_owned(), value);
                index += 2;
            }
            "-" => {
                opts.stdin = true;
                index += 1;
            }
            _ => {
                opts.expr.extend_from_slice(&args[index..]);
                break;
            }
        }
    }
    Ok(opts)
}

/// 表达式从参数来，或者从标准输入读一整段。
fn read_source(opts: &Options) -> Result<String, CliError> {
    if !opts.stdin {
        return Ok(opts.expr.join(" "));
    }
    let mut buf = String::new();
    io::stdin().read_to_string(&mut buf)?;
    Ok(buf)
}

/// 把错误打到标准错误；能定位的再补上源串和脱字符。退出码一律是 2。
fn fail(err: &CliError, src: Option<&str>) -> ExitCode {
    eprintln!("错误：{err}");
    if let (Some(src), CliError::Eval(eval_err)) = (src, err)
        && let Some(offset) = eval_err.offset()
    {
        eprintln!("\n{src}\n{}^", " ".repeat(columns_of(&src[..offset])));
    }
    ExitCode::from(2)
}

/// 一段前缀在终端里占几列。
///
/// 偏移是**字节**位置，脱字符要的是**列**：这个函数换算它们。ASCII 一列，
/// 宽字符（CJK、全角形、U+3000 表意空格）两列，制表符推进到下一个 8 的倍数。
/// 近似就够用：错误之前的前缀里只可能出现空白 —— 非空白的非 ASCII 字符会在
/// 自己被扫到的那一刻当场报错，偏移正好落在它身上。
fn columns_of(prefix: &str) -> usize {
    let mut column = 0;
    for ch in prefix.chars() {
        column += match ch {
            '\t' => 8 - column % 8,
            '\u{1100}'..='\u{115f}'
            | '\u{2e80}'..='\u{a4cf}'
            | '\u{ac00}'..='\u{d7a3}'
            | '\u{f900}'..='\u{faff}'
            | '\u{fe30}'..='\u{fe6f}'
            | '\u{ff00}'..='\u{ff60}'
            | '\u{ffe0}'..='\u{ffe6}' => 2,
            _ => 1,
        };
    }
    column
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_prefixes_count_one_column_each() {
        assert_eq!(columns_of(""), 0);
        assert_eq!(columns_of("1 + "), 4);
    }

    #[test]
    fn wide_characters_count_two_columns() {
        // U+3000 表意空格：三个字节，但占两列。
        assert_eq!(columns_of("　1 "), 4);
        assert_eq!(columns_of("中文"), 4);
    }

    #[test]
    fn a_tab_advances_to_the_next_tab_stop() {
        assert_eq!(columns_of("\t"), 8);
        assert_eq!(columns_of("a\tb"), 9);
    }
}
