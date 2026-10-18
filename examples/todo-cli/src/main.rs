//! `todo-cli`：只用标准库的待办清单工具。

mod cli;
mod error;
mod store;
mod task;

use std::io::{self, Write};
use std::process::ExitCode;

use cli::Command;
use error::Result;
use store::Store;
use task::Status;

fn main() -> ExitCode {
    // `args()` 的第一项是程序自己的路径，跳过去。
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("错误：{e}");
            ExitCode::from(2)
        }
    }
}

/// 跑完一条命令；所有可恢复错误都在这里向上冒泡。
fn run(args: &[String]) -> Result<()> {
    let options = cli::parse(args)?;
    let mut store = Store::open(&options.file)?;

    match &options.command {
        Command::Help => print!("{}", cli::usage()),
        Command::Add { text } => {
            let id = store.add(text)?;
            store.save()?;
            println!("已添加 #{id}");
        }
        Command::List { filter } => {
            let picked = store.select(*filter);
            render_list(&picked, &mut io::stdout().lock())?;
        }
        Command::Done { id } => {
            if store.mark_done(*id)? {
                store.save()?;
                println!("已把 #{id} 标记为完成");
            } else {
                println!("#{id} 本来就是完成状态，没有改动");
            }
        }
        Command::Remove { id } => {
            let removed = store.remove(*id)?;
            store.save()?;
            println!("已删除 #{}：{}", removed.id(), removed.text());
        }
    }
    Ok(())
}

/// 把挑出来的任务写进任意一个 `Write`：测试里换成 `Vec<u8>` 就能断言输出。
fn render_list<W: Write>(tasks: &[&task::Task], out: &mut W) -> Result<()> {
    if tasks.is_empty() {
        writeln!(out, "（没有符合条件的任务）")?;
        return Ok(());
    }
    for task in tasks {
        writeln!(out, "{task}")?;
    }
    let done = tasks
        .iter()
        .filter(|task| task.status() == Status::Done)
        .count();
    writeln!(
        out,
        "共 {} 条：{} 条待办，{} 条已完成",
        tasks.len(),
        tasks.len() - done,
        done
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use task::Task;

    #[test]
    fn empty_selection_prints_a_hint() {
        let mut buf = Vec::new();
        render_list(&[], &mut buf).unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "（没有符合条件的任务）\n");
    }

    #[test]
    fn list_shows_the_marker_and_a_summary() {
        let pending = Task::new(1, "写测试");
        let mut done = Task::new(2, "写文档");
        done.mark_done();
        let picked = vec![&pending, &done];

        let mut buf = Vec::new();
        render_list(&picked, &mut buf).unwrap();
        let out = String::from_utf8(buf).unwrap();
        assert!(out.contains("#1 待办 写测试"), "实际输出：{out}");
        assert!(
            out.contains("共 2 条：1 条待办，1 条已完成"),
            "实际输出：{out}"
        );
    }
}
