//! 端到端测试：把编译出来的 `eval` 当黑盒跑，只看退出码、标准输出和标准错误。

use std::io::Write;
use std::process::{Command, Output, Stdio};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_eval"))
        .args(args)
        .output()
        .expect("应该能启动 eval")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn evaluates_an_expression_given_as_several_arguments() {
    let output = run(&["1", "+", "2", "*", "3"]);
    assert!(output.status.success());
    assert_eq!(stdout(&output).trim(), "7");
    assert!(output.stderr.is_empty(), "成功时标准错误应为空");
}

#[test]
fn prints_the_parse_tree_with_ast() {
    let output = run(&["--ast", "1 + 2 * 3"]);
    assert!(output.status.success());
    assert_eq!(stdout(&output).trim(), "(1 + (2 * 3))");
}

#[test]
fn variables_bound_with_var_are_visible_to_the_expression() {
    let output = run(&["--var", "x=3", "--var", "y=4", "sqrt(x^2 + y^2)"]);
    assert!(output.status.success());
    assert_eq!(stdout(&output).trim(), "5");
}

#[test]
fn reads_the_expression_from_standard_input() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_eval"))
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("应该能启动 eval");
    child
        .stdin
        .as_mut()
        .expect("stdin 已经接上管道")
        .write_all(b"2 ^ 10\n")
        .expect("应该写得进去");
    let output = child.wait_with_output().expect("应该能收完输出");
    assert!(output.status.success());
    assert_eq!(stdout(&output).trim(), "1024");
}

#[test]
fn a_syntax_error_exits_with_code_2_and_points_at_the_offset() {
    let output = run(&["2 + * 3"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty(), "出错时标准输出应为空");
    let err = stderr(&output);
    assert!(
        err.contains("偏移 4 处应该是一个值，实际看到 `*`"),
        "实际错误输出：{err}"
    );
    assert!(
        err.contains("2 + * 3\n    ^"),
        "应该画出指向第 5 列的脱字符：{err}"
    );
}

#[test]
fn an_unbound_variable_exits_with_code_2_and_names_it() {
    let output = run(&["x + 1"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("变量 `x` 没有绑定"));
}

#[test]
fn a_non_finite_result_is_an_error_not_infinity() {
    let output = run(&["1 / 0"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("不是有限数"));
}

#[test]
fn a_malformed_var_option_is_a_usage_error() {
    let output = run(&["--var", "x", "1"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("不是 `名字=值` 的形式"));
}

#[test]
fn help_goes_to_stdout_and_exits_with_zero() {
    let output = run(&["--help"]);
    assert!(output.status.success());
    assert!(stdout(&output).contains("eval —— 算一条表达式"));
    assert!(output.stderr.is_empty());
}

#[test]
fn a_wide_space_does_not_shift_the_caret_column() {
    let output = run(&["　1 @ 2"]);
    assert_eq!(output.status.code(), Some(2));
    let err = stderr(&output);
    assert!(
        err.contains("偏移 5 处有不认识的字符 `@`"),
        "实际错误输出：{err}"
    );
    // U+3000 占两列，所以脱字符前是 4 个空格（前缀按列算，不是按字节数）。
    assert!(err.contains("　1 @ 2\n    ^"), "实际错误输出：{err}");
}

#[test]
fn deeply_nested_input_is_an_error_not_a_stack_overflow() {
    let src = format!("{}1{}", "(".repeat(1000), ")".repeat(1000));
    let mut child = Command::new(env!("CARGO_BIN_EXE_eval"))
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("应该能启动 eval");
    child
        .stdin
        .as_mut()
        .expect("stdin 已经接上管道")
        .write_all(src.as_bytes())
        .expect("应该写得进去");
    let output = child.wait_with_output().expect("应该能收完输出");
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty(), "出错时标准输出应为空");
    assert!(stderr(&output).contains("嵌套太深"));
}
