//! 命令行解析：把 `argv` 变成 `Command`。

use std::path::PathBuf;

use crate::error::AppError;
use crate::store::Filter;

/// 默认的数据文件。
pub const DEFAULT_FILE: &str = "todo.txt";

/// 解析好的整条命令。
#[derive(Debug)]
pub struct Options {
    pub file: PathBuf,
    pub command: Command,
}

/// 用户要求执行的动作。
#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    Help,
    Add { text: String },
    List { filter: Filter },
    Done { id: u32 },
    Remove { id: u32 },
}

/// 解析去掉程序名之后的参数。
pub fn parse(args: &[String]) -> Result<Options, AppError> {
    let mut file = PathBuf::from(DEFAULT_FILE);
    let mut rest = args;
    // 只认「命令之前」的全局选项；命令之后的参数原样交给子命令。
    while let Some((first, tail)) = rest.split_first() {
        match first.as_str() {
            "--file" | "-f" => match tail.split_first() {
                Some((path, more)) => {
                    file = PathBuf::from(path);
                    rest = more;
                }
                None => return Err(AppError::BadArgs("`--file` 后面要跟一个路径".to_owned())),
            },
            _ => break,
        }
    }
    Ok(Options {
        file,
        command: parse_command(rest)?,
    })
}

fn parse_command(args: &[String]) -> Result<Command, AppError> {
    let Some((name, rest)) = args.split_first() else {
        return Err(AppError::BadArgs(
            "没有命令；跑 `todo-cli help` 看用法".to_owned(),
        ));
    };
    match name.as_str() {
        "help" | "--help" | "-h" => Ok(Command::Help),
        "add" => {
            // 把后面的参数拼成一段文本：`todo-cli add 买 牛奶` 就够了，不必加引号。
            let text = rest.join(" ");
            if text.trim().is_empty() {
                Err(AppError::BadArgs("`add` 后面要跟任务文本".to_owned()))
            } else {
                Ok(Command::Add { text })
            }
        }
        "list" => Ok(Command::List {
            filter: parse_filter(rest)?,
        }),
        "done" => Ok(Command::Done {
            id: parse_id(rest, "done")?,
        }),
        "rm" => Ok(Command::Remove {
            id: parse_id(rest, "rm")?,
        }),
        other => Err(AppError::BadArgs(format!(
            "不认识的命令 `{other}`；跑 `todo-cli help` 看用法"
        ))),
    }
}

/// `list` 的筛选开关：最多一个。
fn parse_filter(args: &[String]) -> Result<Filter, AppError> {
    match args {
        [] => Ok(Filter::All),
        [flag] => match flag.as_str() {
            "--all" | "-a" => Ok(Filter::All),
            "--pending" | "-p" => Ok(Filter::Pending),
            "--done" | "-d" => Ok(Filter::Done),
            other => Err(AppError::BadArgs(format!("`list` 不认识 `{other}`"))),
        },
        _ => Err(AppError::BadArgs(
            "`list` 最多只接受一个筛选开关".to_owned(),
        )),
    }
}

/// 取一个数字编号。
fn parse_id(args: &[String], cmd: &str) -> Result<u32, AppError> {
    match args {
        [id] => id
            .parse()
            .map_err(|_| AppError::BadArgs(format!("`{cmd}` 需要一个数字编号，收到 `{id}`"))),
        [] => Err(AppError::BadArgs(format!("`{cmd}` 后面要跟任务编号"))),
        _ => Err(AppError::BadArgs(format!("`{cmd}` 只接受一个编号"))),
    }
}

/// 用法说明；`--help`、`-h` 与 `help` 都打印它。
pub fn usage() -> &'static str {
    "\
todo-cli —— 只用标准库的待办清单

用法：
    todo-cli [--file <路径>] <命令> [参数]

命令：
    add <文本>      添加一条待办（文本可以带空格，不必加引号）
    list [筛选]     列出任务，默认 --all
                      --all / -a       全部
                      --pending / -p   只看待办
                      --done / -d      只看已完成
    done <编号>     把某条任务标记为完成
    rm <编号>       删除某条任务
    help            打印这份说明

选项：
    --file <路径>, -f <路径>   数据文件，默认 todo.txt
    --help, -h                 打印这份说明

退出码：
    0  成功
    2  参数或数据有问题（错误信息写到标准错误）
"
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(raw: &[&str]) -> Vec<String> {
        raw.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn add_joins_the_remaining_arguments_into_one_text() {
        let options = parse(&args(&["add", "买", "牛奶"])).unwrap();
        assert_eq!(
            options.command,
            Command::Add {
                text: "买 牛奶".to_owned()
            }
        );
        assert_eq!(options.file, PathBuf::from(DEFAULT_FILE));
    }

    #[test]
    fn file_option_comes_before_the_command() {
        let options = parse(&args(&["--file", "x.txt", "list", "--done"])).unwrap();
        assert_eq!(options.file, PathBuf::from("x.txt"));
        assert_eq!(
            options.command,
            Command::List {
                filter: Filter::Done
            }
        );
    }

    #[test]
    fn flags_after_the_command_belong_to_the_command() {
        // `add` 之后的 `--file` 是任务文本的一部分，不再是选项。
        let options = parse(&args(&["add", "--file", "x"])).unwrap();
        assert_eq!(
            options.command,
            Command::Add {
                text: "--file x".to_owned()
            }
        );
    }

    #[test]
    fn missing_file_path_is_an_error() {
        let err = parse(&args(&["--file"])).unwrap_err();
        assert!(err.to_string().contains("`--file`"), "实际：{err}");
    }

    #[test]
    fn non_numeric_id_is_an_error() {
        let err = parse(&args(&["done", "abc"])).unwrap_err();
        assert!(err.to_string().contains("需要一个数字编号"), "实际：{err}");
    }
}
