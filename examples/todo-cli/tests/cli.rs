//! 端到端测试：把编译出来的二进制当黑盒跑一遍，只看退出码、标准输出和标准错误。

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

/// 每个测试用一个只属于自己的临时文件，避免相互踩数据。
fn temp_file(name: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!("todo-cli-test-{}-{name}", std::process::id()));
    let _ = fs::remove_file(&path);
    path
}

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_todo-cli"))
        .args(args)
        .output()
        .expect("应该能启动 todo-cli")
}

/// 走一遍「添加 → 列表 → 标记完成 → 删除」的完整流程。
#[test]
fn full_round_trip_through_the_binary() {
    let file = temp_file("round-trip.txt");
    let path = file.to_str().unwrap();

    let add = run(&["--file", path, "add", "写", "测试"]);
    assert!(add.status.success());
    assert_eq!(String::from_utf8_lossy(&add.stdout).trim(), "已添加 #1");

    let list = run(&["--file", path, "list"]);
    let printed = String::from_utf8_lossy(&list.stdout);
    assert!(printed.contains("#1 待办 写 测试"), "实际输出：{printed}");

    let done = run(&["--file", path, "done", "1"]);
    assert!(String::from_utf8_lossy(&done.stdout).contains("已把 #1 标记为完成"));

    let empty = run(&["--file", path, "list", "--pending"]);
    assert!(String::from_utf8_lossy(&empty.stdout).contains("（没有符合条件的任务）"));

    let rm = run(&["--file", path, "rm", "1"]);
    assert!(String::from_utf8_lossy(&rm.stdout).contains("已删除 #1：写 测试"));

    fs::remove_file(&file).unwrap();
}

/// 错误路径：编号不存在时给一句人话，并以 2 退出。
#[test]
fn unknown_id_exits_with_code_2_and_explains_why() {
    let file = temp_file("unknown-id.txt");
    let done = run(&["--file", file.to_str().unwrap(), "done", "7"]);

    assert_eq!(done.status.code(), Some(2));
    let err = String::from_utf8_lossy(&done.stderr);
    assert!(err.contains("没有编号为 7 的任务"), "实际错误输出：{err}");
    assert!(done.stdout.is_empty(), "出错时标准输出应为空");
}
