# 第 11 章 测试与调试

## 这一章解决什么问题

到目前为止，你验证代码的方式多半是「跑一下，用眼睛看输出对不对」。函数只有一两个时这没问题；一旦输入组合多起来，靠手跑必然漏。本章把这件事变成工程习惯：把「我期望它这样」写成可执行的定义，交给 `cargo test` 反复检查；当它失败时，把输出当成证据，回推到出错的那几行。

这一章也是第 2 章「读懂编译器」的姊妹篇：那章教你把**编译错误**读成线索，这章教你把**测试失败**读成线索。两者合起来，就是本书把「诊断」当方法的那条暗线。

读完你能做到：给一个函数写单元测试、给一个库写集成测试；读懂 `cargo test` 的每一段输出；用 `dbg!` 与 `RUST_BACKTRACE` 定位运行期问题；并按「复现 → 最小化 → 假设 → 验证」的循环，从一个失败的测试走到修复。

## 为什么这样设计

在多数语言里，测试是外挂的：Python 装 pytest，Java 装 JUnit。Rust 把它做进了工具链——`#[test]` 只是一个普通属性，`cargo test` 是内置的运行器，开箱即用，不引入任何第三方依赖。

为什么值得这样设计？因为「不敢改」是代码库变老后最贵的问题。测试是让你敢改的那份保险：改完能立刻知道有没有碰坏别的东西。

更准确的看法是：**测试是可执行的规格**。`assert_eq!(count_words("hello world"), 2)` 这一行同时是两样东西——文档（告诉读者这个函数干什么）和门禁（行为一变就变红）。所以「先写测试」在 Rust 里格外自然：你不是在写仪式，而是在写规格。

还有一层分工要记牢：**编译器保证内存安全和类型正确，测试保证语义正确**。第 2、4 章那个借用检查器能拦下悬垂引用，却拦不下「这个函数对空输入返回 1 而不是 0」。后者正是测试的领域，两者互不替代。

## 机制与规则

### 单元测试：贴着代码

单元测试（unit test）针对一个模块的内部行为，需要访问私有项，所以放在**与代码同一个文件**里。约定是开一个 `#[cfg(test)] mod tests`：

```rust
pub fn count_words(text: &str) -> usize {
    text.split_whitespace().count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_words() {
        assert_eq!(count_words("hello world"), 2);
    }
}
```

两个细节要理解：

- `#[cfg(test)]` 是条件编译（第 2 章见过 `cfg`）：只有跑 `cargo test` 时这个模块才被编译，`cargo build` 会把它整个删掉。所以测试既不拖慢正式构建，也不会进入发布产物。
- `use super::*;` 把被测模块里的东西引进测试模块。`tests` 是 `count_words` 所在模块的子模块，`super` 就指父模块（模块系统见第 8 章）。

### 断言宏

主力是三个宏：

| 宏 | 通过条件 |
|---|---|
| `assert!(cond)` | `cond` 为真 |
| `assert_eq!(a, b)` | `a == b` |
| `assert_ne!(a, b)` | `a != b` |

它们都接受一个可选的格式串，失败时把它打印出来，这是让失败信息「自解释」的关键：

```rust
#[test]
fn with_message() {
    let got = 3;
    assert!(got > 0, "期望正数，实际得到 {got}");
    assert_eq!(got, 3, "got 应该是 3");
    assert_ne!(got, 4);
}
```

> [!TIP]
> 期望值尽量写死（`assert_eq!(f(x), 2)`），别写成 `assert_eq!(f(x), g(x))`——后者只证明两者一致，不证明它们对，测试也就永远抓不到真正的错误。

### 期望 panic 的测试

有些函数的契约就是「非法输入时 panic」（第 7 章）。这类行为用 `#[should_panic]` 断言。带上 `expected` 才能确认是**因为预期原因** panic——它做的是子串匹配，只要 panic 信息里包含这串就算通过：

```rust
fn checked_len(s: &str) -> usize {
    if s.is_empty() {
        panic!("文本不能为空");
    }
    s.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic(expected = "文本不能为空")]
    fn empty_input_panics() {
        let _ = checked_len("");
    }
}
```

不写 `expected` 时，函数因任何原因 panic 都算通过，很容易把真正的 bug 盖住。

### 测试返回 `Result`

测试函数默认返回 `()`。也可以返回 `Result<(), E>`，这样测试体里就能用 `?`（第 7 章）：

```rust
#[test]
fn checked_division() -> Result<(), String> {
    let n: i32 = 10;
    let d: i32 = 2;
    if d == 0 {
        return Err(String::from("除数不能为 0"));
    }
    assert_eq!(n / d, 5);
    Ok(())
}
```

对应关系要记清：`Ok(())` = 通过，`Err(...)` = 失败，中间的断言 panic 也 = 失败。唯一的限制是：返回 `Result` 的测试**不能**再标 `#[should_panic]`——前者靠返回 `Err` 表示失败，后者靠 panic，语义冲突，编译器会直接拒绝（1.98 实测：`functions using #[should_panic] must return ()`）。

### 集成测试：只碰公开 API

单元测试看内部，集成测试（integration test）则从**外部**使用你的库——像真实用户那样，因此只能调用公开 API，测的是「各部分拼起来对不对」。它们放在项目根的 `tests/` 目录里，每个文件都是一个独立 crate：

```rust,ignore
{{#include ../snippets/ch11/tests/integration.rs}}
```

注意几点：

- `tests/` 里的文件**不需要** `#[cfg(test)]`：Cargo 只在 `cargo test` 时编译这个目录。
- 每个文件是独立 crate，所以要写 `use 包名::…` 把库引进来（包名里的 `-` 换成 `_`）。
- 想共享辅助函数，放 `tests/common/mod.rs`，不要放 `tests/common.rs`——后者会被当成一个（空的）测试 crate 跑起来，前面的 `tests/common` 就会出现一次「running 0 tests」。

目录长这样：

```text
ch11-demo
├── Cargo.toml
├── src
│   └── lib.rs          # 单元测试写在里面
└── tests
    └── integration.rs  # 集成测试，只碰公开 API
```

本章演示用的完整可跑版本在 `snippets/ch11/`。

### 读 `cargo test` 的输出

`cargo test` 会依次跑三类目标，每类一段输出：**单元测试**（`Running unittests src\lib.rs`）、**集成测试**（`Running tests\integration.rs`）、最后是**文档测试**（`Doc-tests`，第 32 章展开）。每段都以一行汇总收尾：

```text
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

这行从左到右就是你的体检报告：通过几个、失败几个、被忽略（`#[ignore]`）几个、被过滤掉几个。**关键规则**：任何一段失败，它后面的段就不跑了——所以单元测试一片红时，看不到集成测试的结果是正常的，先把单元测试修绿。

常用的三个开关：

- `cargo test <子串>`：只跑名字里含该子串的测试（子串同时匹配三类目标里的函数名）。
- `cargo test -- --no-capture`：`--` 之后的参数交给测试运行器。默认它会**捕获**测试打印的 stdout/stderr，只在失败时显示；`--no-capture` 让它们实时打出来——调试时很有用。（1.88 前的拼写是 `--nocapture`，仍被接受，但已不在 `--help` 里，用新写法。）
- `cargo test -- --test-threads=1`：串行执行。测试默认是多线程并行的，当几个测试共享文件或端口、互相干扰时，这条命令能消除顺序不确定性。

还有两个按目标过滤的开关，调试时能少看一堆输出：`cargo test --lib` 只跑单元测试，`cargo test --test integration` 只跑 `tests/integration.rs` 这一个文件。注意 `--lib`、`--test` 属于 cargo 的参数，写在 `--` 前面。

### 调试：`dbg!` 与 `#[derive(Debug)]`

`dbg!` 是「带上下文的 `eprintln!`」。它把表达式的**值**和它在**源码里的位置**一起打到 stderr，并原样返回这个值，所以可以嵌进表达式中间：

```rust
let n = dbg!(1 + 2);
assert_eq!(n, 3);
```

它比 `eprintln!` 好在三处：自动带上值和文件行列号、自动标出是哪段表达式、不打断代码流程（返回传入的值）。代价是被调试的类型必须实现 `Debug`——所以自定义结构体要加 `#[derive(Debug)]`（第 5 章讲过）。

> [!NOTE]
> `dbg!` 输出走 stderr，会被测试运行器捕获。想在测试里看到它，配 `--no-capture` 或 `--show-output`。

需要真正单步执行时，用编辑器里的调试器：VS Code 装 rust-analyzer 与 **CodeLLDB** 扩展，在行号处打红点设断点，按 F5 启动，就能逐行执行、查看变量与调用栈。扩展入口见 CodeLLDB 的仓库（本节末「延伸阅读」）。而如果只是想知道「为什么 panic」，直接用 `RUST_BACKTRACE=1` 跑，会打印出 panic 的完整调用栈——细节见第 7 章。

## 动手看看

我们写一个迷你文本统计库，刻意留下一个 bug，完整走一遍「写测试 → 跑 → 读输出 → 定位 → 修」。

起点（这段代码是错的，别照抄）：

```rust,ignore
/// 统计 `text` 中的单词个数（以空白分隔）。
pub fn count_words(text: &str) -> usize {
    text.split(' ').count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_words() {
        assert_eq!(count_words("hello world"), 2);
    }

    #[test]
    fn ignores_extra_spaces() {
        assert_eq!(count_words("  hello   world  "), 2);
    }

    #[test]
    fn empty_text() {
        assert_eq!(count_words(""), 0);
    }
}
```

跑 `cargo test`（以 Rust 1.98 为例）：

```console
$ cargo test
   Compiling ch11-demo v0.1.0 (D:\Rust\.scratch\tmp-20\ch11demo)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.72s
     Running unittests src\lib.rs (target\debug\deps\ch11_demo-bd516542a222d203.exe)

running 3 tests
test tests::counts_words ... ok
test tests::ignores_extra_spaces ... FAILED
test tests::empty_text ... FAILED

failures:

---- tests::ignores_extra_spaces stdout ----

thread 'tests::ignores_extra_spaces' (19164) panicked at src\lib.rs:17:9:
assertion `left == right` failed
  left: 8
 right: 2

---- tests::empty_text stdout ----

thread 'tests::empty_text' (9764) panicked at src\lib.rs:22:9:
assertion `left == right` failed
  left: 1
 right: 0
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    tests::empty_text
    tests::ignores_extra_spaces

test result: FAILED. 1 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

error: test failed, to rerun pass `--lib`
```

怎么读这段输出，三步：

1. **先看总结行**：`1 passed; 2 failed`——`counts_words` 过了，说明主路径没错；问题只在两种边界输入上。
2. **看每个失败的名字**：`ignores_extra_spaces`（多余空格）和 `empty_text`（空串）。两个名字已经把「什么输入出问题」说清楚了。
3. **看断言差异**：每个失败块给出 `left`/`right`。`ignores_extra_spaces` 是 `left: 8 right: 2`——函数返回 8，我们期望 2。注意 `left` 是**实际值**，`right` 是**期望值**（左边是 `assert_eq!(a, b)` 的 `a`）。空串那条是 `left: 1 right: 0`。

**定位**：「多余空格被数成了单词」指向拆分逻辑。`"  hello   world  "` 按空格 `' '` 拆，会拆出一串空片段（连续空格和首尾空格都产生空串），所以数出 8。`"".split(' ')` 也会得到 1 个空片段，所以空串返回 1。两处失败的**同一个根因**：`split(' ')` 按「一个分隔符」拆，而我们要的是「按任意长度的空白拆」。

**假设与验证**：标准库里有 `str::split_whitespace`，语义正是「按任意长度的空白拆分，并自动忽略首尾空白」。把它换上去，同时通过两条测试，假设就得到验证：

```rust
/// 统计 `text` 中的单词个数（以空白分隔）。
pub fn count_words(text: &str) -> usize {
    text.split_whitespace().count()
}
```

再跑一次，三处目标全绿：

```console
$ cargo test
   Compiling ch11-demo v0.1.0 (D:\Rust\.scratch\tmp-20\ch11demo)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.55s
     Running unittests src\lib.rs (target\debug\deps\ch11_demo-bd516542a222d203.exe)

running 3 tests
test tests::counts_words ... ok
test tests::empty_text ... ok
test tests::ignores_extra_spaces ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests\integration.rs (target\debug\deps\integration-247b0bf8a42c13fc.exe)

running 1 test
test counts_words_in_a_sentence ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests ch11_demo

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

回看整条路径：**复现**（跑出失败）→ **最小化**（失败名的输入就是最小例子）→ **假设**（按单个空格拆是错的）→ **验证**（换 `split_whitespace` 后全绿）。这条路后面每章都会用到，尤其是第 16、22 章那种和借用检查器、`Pin` 较劲的地方。

顺带一提，用子串过滤只跑想看的测试也很快，名字同时匹配单元与集成测试：

```console
$ cargo test counts_words
     Running unittests src\lib.rs (target\debug\deps\ch11_demo-bd516542a222d203.exe)

running 1 test
test tests::counts_words ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2 filtered out; finished in 0.00s

     Running tests\integration.rs (target\debug\deps\integration-247b0bf8a42c13fc.exe)

running 1 test
test counts_words_in_a_sentence ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

## 报错现场

测试失败不是「程序崩了」，而是**规格没被满足**。识别两类最常见的失败输出，就能快速分流。

一、`assert_eq!` 差异。这是最有用的一类：它把实际值与期望值并排给出。

```text
---- tests::ignores_extra_spaces stdout ----

thread 'tests::ignores_extra_spaces' (19164) panicked at src\lib.rs:17:9:
assertion `left == right` failed
  left: 8
 right: 2
```

看到 `left`/`right` 先别急着改测试——想想哪个是「正确」的。这里 `right: 2` 是期望、`left: 8` 是函数实际返回，于是问题落在被调用的函数上。最后那行 `note: run with RUST_BACKTRACE=1 …` 提示你可以打开 backtrace 看调用栈（第 7 章）。panic 后面的 `src\lib.rs:17:9` 精确告诉你是哪一行的断言在报——这里第 17 行就是那条 `assert_eq!`。

二、`#[should_panic]` 却没 panic。如果你断言「它应该 panic」，而它安静地返回了，输出是这样：

```text
running 2 tests
test tests::empty_input_panics - should panic ... ok
test tests::wrong_expectation - should panic ... FAILED

failures:

---- tests::wrong_expectation stdout ----
note: test did not panic as expected at src\lib.rs:18:8

failures:
    tests::wrong_expectation

test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
```

关键句是 `test did not panic as expected`。要么是函数该 panic 却没检查输入（真 bug），要么是测试本身写错了对象。注意上面那条通过的测试带 `- should panic` 标记，提醒你它属于哪一类。

碰到上面任何一种，都照「动手看看」里那条循环走：先复现，把输入缩到最小，提出一个可证伪的假设，改一处，再跑。

## 小结

- 测试是**可执行的规格**：把期望写成断言，让 `cargo test` 反复替你检查。编译器管内存与类型，测试管语义。
- 单元测试贴着代码放，用 `#[cfg(test)] mod tests` 包起来，能测私有项；集成测试放 `tests/` 目录，只测公开 API，每个文件是独立 crate。
- 断言用 `assert!` / `assert_eq!` / `assert_ne!`，配上格式串让失败信息自解释；期待 panic 用 `#[should_panic(expected = "…")]`；需要 `?` 时让测试返回 `Result`。
- 读输出从总结行的 `passed/failed` 入手，再看失败名、断言差异（`left` 实际、`right` 期望）和 panic 位置。任一段失败，后面的段不会跑。
- 调试用 `dbg!`（带位置、带回显）替代裸 `eprintln!`；panic 调用栈用 `RUST_BACKTRACE=1`；需要单步就上编辑器的调试器。
- 从失败回推的循环：复现 → 最小化 → 假设 → 验证。这是全书一致的诊断方法。

## 语言对照角

如果你来自别的语言，几点差异值得注意：

- **Python / Java**：测试要装框架（pytest、JUnit），断言是库函数调用，失败报告要框架来渲染；Rust 的 `#[test]` 与 `cargo test` 由编译器与工具链直接提供，不装任何东西就能得到逐条测试名、断言差异和汇总行。
- **C++**：常见的是 GoogleTest、Catch2，需要构建系统接线；Rust 内置的这套相当于「开箱自带的最简 GoogleTest」。另外，C 的 `assert` 会被 `NDEBUG` 关掉，而 Rust 的 `assert!` / `assert_eq!` **永远生效**——只有 `debug_assert!` 系列才在 release 下关闭。
- **Go**：`go test` 与 Rust 最像——测试和被测代码放一起、约定命名、表驱动测试是社区惯用法。区别是 Go 用 `t.Fatalf` 表达失败，Rust 用 panic 型断言。
- 多数语言用 `try/catch` 或显式异常表达「期望出错」，Rust 把它编码进属性（`#[should_panic]`）或返回值（`Result`），第 7 章讲过这种「把异常变成值」的取舍。

## 练习

1. 下面这个测试能不能抓到 bug？说明理由。

   ```rust,ignore
   fn square(x: i32) -> i32 { x * x }

   #[test]
   fn squares() {
       assert_eq!(square(3), square(2) + square(1));
   }
   ```

2. 你给一个返回 `Result` 的测试加了 `#[should_panic]`，想同时验证「失败时返回 `Err`」和「panic」。这样对吗？

3. 集成测试里写了 `use my_crate::helper;`，编译报错说找不到 `my_crate`。最可能的原因是什么？

<details><summary>参考答案</summary>

1. 抓不到，而且这个写法本身有问题。它断言的是 `square(3) == square(2) + square(1)`。对正确的实现 `x * x` 来说，就是 `9 == 4 + 1`——**不成立**，测试会失败，但函数其实没错。反过来，对错误的实现 `fn square(x: i32) -> i32 { 3 * x }`，等式 `9 == 6 + 3` 却成立，测试通过而函数是错的。两头都不靠谱：用被测函数自己的输出互相验证，证明的只是「两者一致」，不是「结果正确」。正确写法是拿写死的期望值比：`assert_eq!(square(3), 9);`。

2. 不对。`#[should_panic]` 要求测试函数返回 `()`，而返回 `Result` 的测试必须返回 `Result`，两者互斥，编译器会直接报错（`functions using #[should_panic] must return ()`）。二选一：要验证 `Err`，就让测试返回 `Result`、把「应失败」的情况写成断言；要验证 panic，就让测试返回 `()` 并用 `#[should_panic]`。

3. 最可能是包名没对上。集成测试里用的 crate 名来自 `[package] name`，且其中的连字符要换成下划线（包名 `my-crate` → crate 名 `my_crate`）。另外，如果项目是**只有 `src/main.rs` 的二进制 crate**，它不向外部暴露任何函数，集成测试根本无从 `use`——这时要把逻辑挪进 `src/lib.rs`，让集成测试去测库。

</details>

## 延伸阅读

- The Book, 第 11 章《Writing Automated Tests》：<https://doc.rust-lang.org/1.98.0/book/ch11-00-testing.html>
- Cargo Book,《Tests》指南：<https://doc.rust-lang.org/1.98.0/cargo/guide/tests.html>
- `cargo test` 命令参考：<https://doc.rust-lang.org/1.98.0/cargo/commands/cargo-test.html>
- 标准库断言宏与 `dbg!`：<https://doc.rust-lang.org/1.98.0/std/macro.assert_eq.html>、<https://doc.rust-lang.org/1.98.0/std/macro.dbg.html>
- rustc,《Tests》说明（libtest 运行器）：<https://doc.rust-lang.org/1.98.0/rustc/tests/index.html>
- VS Code 调试扩展 CodeLLDB：<https://github.com/vadimcn/code-lldb>
