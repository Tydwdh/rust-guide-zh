# 第 31 章 测试进阶

## 这一章解决什么问题

第 11 章给了你测试的基本功：写单元测试与集成测试、读懂 `cargo test` 的输出、从失败回推到原因。这一章解决的是往下一层的问题——**测试本身怎么才靠得住**。

具体是三件事。

第一件最隐蔽：**绿了不等于测了**。书里的代码块走的是 rustdoc 通道，`cargo test` 走的是另一条；两条通道的编译配置不一样，于是同一段测试代码在一条上「通过」，在另一条上完全不参与编译。这不是理论担忧——下面给出完整命令与输出。

第二件：**doctest 的属性有一套语义边界**。`no_run`、`should_panic`、`compile_fail` 各自断言什么、不断言什么，比 `#[test]` 那些属性更容易用错，因为它们的默认行为更宽松。

第三件：**一个用例不够用时怎么办**。工具链只给了最小内核（`#[test]` + libtest + `cargo test`），参数化、属性测试、快照测试、更快的运行器都在生态里。这一章讲清它们各自的定位与代价，以及 Miri、loom 这两类不跑普通测试的工具该放在 CI 的哪一环。

读完你能做到：判断一段测试代码有没有真的被编译执行；在几个 doctest 属性里选对那个；把重复用例写成表驱动测试；在需要时知道该装哪个外部工具，以及它换来的是什么、要付出什么。单元测试与集成测试的基本写法不再重复，见第 11 章。

## 为什么这样设计

多数语言的测试栈是「框架 + 运行器」两层：断言库、mock 库、参数化装饰器、快照插件，各装各的。Rust 只把**最小内核**做进工具链——`#[test]` 是属性，libtest 是内置运行器，`cargo test` 负责收集与调度——其余交给生态。这让新项目零配置就能跑测试，代价是「想加功能就得装 crate」。

更重要的是这三层是怎么分出来的：不是按代码量，而是按**编译配置**。

- **单元测试**与代码在同一个 crate、同一份源码里，只在 `cargo test` 时带着 `cfg(test)` 编译进去。它能看见私有项，因为它就是代码的一部分。
- **集成测试**在 `tests/` 目录，每个文件是独立 crate，只能通过公开 API 使用你的库——它模拟的是真实用户。
- **文档测试（doctest）**在 `///` 注释的代码块里，由 rustdoc 抽出来编译成独立程序。关键差别是：rustdoc 编译 doctest 时**不开 `cfg(test)`**。

最后这条就是本章第一个大坑的源头。`#[cfg(test)]` 是一个条件编译开关：`cargo test` 把它打开，`cargo build` 和 rustdoc 都关着。所以 `#[cfg(test)] mod tests { ... }` 里的一切，在 rustdoc 眼里根本不存在；`#[test]` 函数也一样。判断一条测试的价值，可以用三个问题串起来：

> 它被编译了吗？它被执行了吗？它断言的是你想断言的东西吗？

三个「是」才算数；少一个，测试就从资产变成负资产——它给你安全感，却不提供保护。

doctest 还有一层设计上的双重身份：注释里的代码块同时是**文档**和**测试**，这是「测试是可执行的规格」最纯粹的形态（第 11 章的暗线）。代价是它多出一套独立属性，必须单独记。

## 机制与规则

### 三种测试按编译配置分工

| | 单元测试 | 集成测试 | 文档测试 |
|---|---|---|---|
| 位置 | 与代码同文件，`#[cfg(test)] mod tests` | `tests/*.rs` | `///` 注释里的代码块 |
| 视角 | crate 内部，可访问私有项 | 外部用户，只碰公开 API | 外部用户，且必须能在示例里表达 |
| 谁编译它 | `cargo test`（开 `cfg(test)`） | `cargo test`（独立 crate） | rustdoc（**不开** `cfg(test)`） |
| 输出段 | `Running unittests src\lib.rs` | `Running tests\integration.rs` | `Doc-tests` |
| 典型用途 | 边界条件、私有逻辑 | API 契约、跨模块拼装 | 保证文档里的示例真能用 |

还有一类目标容易被忘掉：`cargo test` 会**编译** `examples/` 下的示例（保证它们不随主代码腐坏），但默认不运行它们。本机 1.98 实测：在 `examples/` 里放一个类型错误的示例，`cargo test --no-run` 直接报 `could not compile ... (example "broken")` 并退出 101。

### 假绿：`mdbook test` 绿不等于测试跑过

这是本章的核心陷阱，值得完整走一遍实证（本机 Windows 11、Rust 1.98、mdBook 0.5.4）。

在章文件里放这样一个块（故意不加 `ignore`——它要走 rustdoc 通道，这句话才有人替我们验证）：

```rust
#[test]
fn broken_test() {
    let got: i32 = "8080";   // 类型不匹配
    assert_eq!(got, 8080);
}
```

运行 `mdbook test -c ch31-advanced-testing.md`：**exit 0**，只打印一行 INFO，没有任何测试失败报告。原因不是 mdBook 出了问题，而是 rustdoc 编译这个块时 `cfg(test)` 是关的，`#[test]` 函数整个被条件编译剔除；块本身编译并运行成功，于是「通过」。

把同一段代码原样搬进 crate 的 `#[cfg(test)] mod tests`，`cargo test` 立刻报错：

```console
$ cargo test
error[E0308]: mismatched types
  --> src\lib.rs:15:24
   |
15 |         let got: i32 = "8080";
   |                  ---   ^^^^^^ expected `i32`, found `&str`
   |                  |
   |                  expected due to this

error: could not compile `false-green-probe` (lib test) due to 1 previous error; 1 warning emitted
```

所以规则很硬：

- **测试代码必须放进可跑的 crate**，用 `cargo test` 真跑。本书的演示代码就是 `snippets/chNN/` 下的真 crate：`mdbook test` 只保证正文普通示例块能编译运行，它不会替你跑任何测试。
- `mdbook test` 的绿只说明块**语法**合法（解析在条件编译之前跑，语法错误照样会红），以及没被剔除的部分能编译；凡是落在 `#[test]` 里的类型错误、API 误用，它一概看不见。
- 反过来说，`#[cfg(test)]` 的东西在 `cargo build` 下不存在——测试不拖慢发布构建，也不进产物。

下图把两条通道摊开：

<svg viewBox="0 0 680 236" width="100%" role="img" aria-label="同一段测试代码走 rustdoc 通道时 cfg test 关闭、测试代码被剔除所以显示绿色，走 cargo test 通道时 cfg test 打开所以真正编译并报错" xmlns="http://www.w3.org/2000/svg">
<text x="0" y="16" font-size="14" fill="currentColor">同一段测试代码，两条通道，两种结论</text>
<g fill="none" stroke="currentColor" stroke-opacity="0.55">
<rect x="18" y="34" width="300" height="84" rx="5" />
<rect x="362" y="34" width="300" height="84" rx="5" />
<rect x="18" y="146" width="300" height="70" rx="5" stroke="var(--links)" />
<rect x="362" y="146" width="300" height="70" rx="5" stroke="var(--warning-border)" />
</g>
<g font-size="12" fill="currentColor">
<text x="34" y="56" font-size="13" fill="var(--links)">① 书里的代码块</text>
<text x="34" y="76" font-family="monospace">mdbook test / rustdoc --test</text>
<text x="34" y="96">cfg(test) 关闭</text>
<text x="34" y="112">#[test] / cfg(test) 整块被剔除</text>
<text x="378" y="56" font-size="13" fill="var(--links)">② 可跑的 crate</text>
<text x="378" y="76" font-family="monospace">cargo test</text>
<text x="378" y="96">cfg(test) 打开</text>
<text x="378" y="112">测试代码编译并注册进运行器</text>
<text x="34" y="172" font-size="13" fill="var(--links)">结论：ok（exit 0）</text>
<text x="34" y="192">没被剔除的部分编译通过</text>
<text x="34" y="208">测试代码一行也没编译</text>
<text x="378" y="172" font-size="13" fill="var(--warning-border)">结论：E0308（exit 101）</text>
<text x="378" y="192">类型错误在这里才现形</text>
<text x="378" y="208">这才是关于这段代码的真话</text>
</g>
</svg>

### doctest 的五个开关

代码块的属性写在栅栏的行内（三个反引号后面），常用逗号分隔（官方也接受空格与制表符）。五个模式：

| 属性 | 编译 | 运行 | 通过条件 |
|---|---|---|---|
| （无） | 是 | 是 | 正常退出（panic 或非零退出即失败） |
| `ignore` | 否 | 否 | 恒通过 |
| `no_run` | 是 | 否 | 编译通过 |
| `should_panic` | 是 | 是 | 必须 panic |
| `compile_fail` | 期望失败 | 否 | 必须编译失败 |

官方定义很短，大意如下（rustdoc 书）：`no_run`「编译你的代码但不运行它」；`should_panic`「告诉 rustdoc 代码应当编译正确但执行时 panic；如果不 panic，测试就失败」；`compile_fail`「告诉 rustdoc 编译应当失败；如果编译通过，测试就失败」；`ignore`「告诉 rustdoc 忽略你的代码」。

**默认块**：不带任何属性，编译并运行，正常退出才算通过。它被 rustdoc 包进一个 `fn main`，所以块里直接用 `?` 会报 E0277，因为包装出来的函数返回 `()`。官方文档给了两种解法——自己写 `fn main() -> Result<(), E>`，或者在块尾加一行隐藏行 `# Ok::<(), E>(())`。本机 1.98 实测，顶格写 `let n: i32 = "3".parse()?;` 得到的报错正是：

```text
error[E0277]: the `?` operator can only be used in a function that returns `Result` or `Option` (or another type that implements `FromResidual`)
```

按官方给的办法，自己补一个返回 `Result` 的 `main` 就能过（三行 `#` 开头的隐藏行在渲染时收起，读者只看得到中间两行）：

```rust
# fn main() -> Result<(), std::num::ParseIntError> {
let n: i32 = "3".parse()?;
assert_eq!(n, 3);
# Ok(())
# }
```

**`no_run`**：编译但不运行，用于示例依赖外部资源（网络、文件、数据库、环境）的场合。它保证 API 用法正确，又不会在无网环境里失败：

```rust,no_run
use std::net::TcpStream;

// 会连网络：只编译，不运行。
let _ = TcpStream::connect("example.com:80");
```

**`should_panic`**：运行且必须 panic。它和**不带 `expected` 的** `#[should_panic]` 一样，只断言「panic 了」；只有带 `expected` 的那条属性（`#[should_panic(expected = "…")]`）才做子串匹配，把「因为预期原因」也钉住（第 11 章）。

```rust,should_panic
assert!(2 + 2 == 5, "断言失败即 panic");
```

> [!WARNING]
> doctest 里没有 `expected` 这回事。fence 写成 `should_panic,expected="…"` 时，rustdoc 把逗号后面那截当成无法识别的属性，只给一句 `unexpected character =` 警告（lint `rustdoc::invalid_codeblock_attributes`），然后**整个块不参与测试**：本机 1.98 实测，同一文件里的普通 `should_panic` 块照跑，它却连测试列表都没进，exit 0。要在 doctest 里核对 panic 消息，只能在块内自己捕获，或者把这条断言放回 crate 的 `#[should_panic(expected = "…")]` 测试。

**`compile_fail`**：块必须编译失败。它的边界最容易踩：它只断言「失败」，**不断言为什么失败**。块因为拼错函数名、漏一个 `use`，甚至语法错误（本机实测：放一个未闭合的括号照样通过），任何原因挂掉都算通过。想收紧，官方在 nightly 上支持带错误码：`compile_fail,E0308`。

```rust,compile_fail
// 这段必须编译失败：&str 不是 u16。
let port: u16 = "8080";
```

> [!WARNING]
> 错误码校验是 nightly-only。官方文档明说：「Attempting to use these error numbers on stable will result in the code sample being interpreted as plain text.」本机 stable 1.98 实测：一个真实错误码为 E0308 的块，标成 `compile_fail,E0277` 照样通过——错误码那一截被忽略，只剩 `compile_fail` 本身生效。所以本书正文统一写 `rust,compile_fail`，把错误码与关键报错行放进紧跟的 `text` 输出块，在 stable 上真验。

**`ignore`**：只渲染，不编译不运行，连语法都不查：

```rust,ignore
// 缺右花括号——ignore 连语法都不查，这个块永远不会红。
fn half(n: u32) -> u32 {
    n / 2
```

它是最后手段。能用隐藏行（行首 `# `）拼出一个可编译的完整版本，就别用 `ignore`；`ignore` 会让示例悄悄腐烂。

### 参数化：一个测试函数 + 一张表

libtest 的测试函数签名是固定的：无参数，返回 `()` 或 `Result<(), E>`。所以 Rust 里没有「参数化测试」这种语法装置，惯用法是**表驱动（table-driven）**：一个 `#[test]` 函数里放一张表，循环跑完。

表里最常出现的三种形态：输入→期望值、输入→应当报错、输入→自身（round-trip，验证编码与解码互逆）。下面是从示范 crate 里摘的表（完整版本在 `snippets/ch31/`）：

```rust,ignore
#[test]
fn parses_levels_case_insensitively() {
    let cases = [
        ("error", LogLevel::Error),
        ("ERROR", LogLevel::Error),
        ("Warn", LogLevel::Warn),
        ("info", LogLevel::Info),
        ("  debug  ", LogLevel::Debug),
    ];
    for (input, expected) in cases {
        let got = input.parse::<LogLevel>();
        assert_eq!(got, Ok(expected), "输入 {input:?}");
    }
}
```

三个要点：

- **一对 `(输入, 期望)` 就是一行**。加用例只加一行，不用复制函数、不用取名字。
- **断言必须带上输入**。`assert_eq!` 在比较的两个值之后还接受一个格式串，把 `{input:?}` 写进去，失败时才知道是哪一行出的问题。漏掉它，一个表里有几个用例你就得自己二分。
- 表里只放**数据**，不放逻辑。如果某个用例需要的准备代码完全不同，它就不是同一张表里的事，单独写一个测试函数更清楚。

用例多到需要按名字单独跑、或者需要二进制夹具时，外部 crate `rstest` 提供了 `#[case]` 与 `#[fixture]` 属性宏（说明性，本章不展开）。

### 属性测试：让生成器找输入（proptest，说明性）

表驱动的盲区是：**你想得到哪些输入，才能写进表里**。属性测试换个方向——不写具体用例，写**性质**，让框架生成输入去证伪。

`proptest` 是 Rust 里这一类工具的代表（QuickCheck 家族，设计上参考了 Python 的 Hypothesis）。它的核心机制是：

- **策略（strategy）**描述「合法输入长什么样」——`0u32..10000` 这样的范围、正则字符串、`any::<Vec<u8>>()`。
- **自动缩小（shrink）**：找到反例后，框架自动把输入往小里收敛，直到找出最小反例。官方例子里，日期解析 bug 的反例从 `7360-12-20` 一路缩到 `0000-10-01`。
- **失败持久化**：反例写进 `proptest-regressions/` 目录，官方建议把它加入版本控制，之后每轮测试都会重放。
- 宏是 `proptest! { ... }`，断言用 `prop_assert_eq!` 一类；官方 README 记的 MSRV 是 1.88。

外部依赖，写法示意：

```rust,ignore
use proptest::prelude::*;

proptest! {
    #[test]
    fn normalize_never_panics(s in "\\PC*") {
        let _ = ch31_demo::normalize_word(&s);
    }

    #[test]
    fn normalize_is_idempotent(s in "\\PC*") {
        let once = ch31_demo::normalize_word(&s);
        let twice = ch31_demo::normalize_word(&once);
        prop_assert_eq!(once, twice);
    }
}
```

什么代码值得上属性测试？输入空间大、性质好写的：解析器、编解码、布局与字节级转换（第 25 章那些函数就是理想目标）、以及「任意输入下都不该 panic」这条最低性质。

### 快照测试：把期望值外置（insta，说明性）

有些函数的输出是一大块文本（渲染结果、序列化结果、编译器诊断）。手写期望值代价高、维护烦。快照测试把期望值存成文件，第一次生成、之后比对：

- `insta` 的断言宏按生成方式分三类：`assert_snapshot!`（自己转成字符串）、`assert_debug_snapshot!`（用 `Debug`）、`assert_yaml_snapshot!` 等（序列化，推荐 YAML，因为 diff 好看且支持脱敏）。
- 新快照默认写进同目录的 `.snap.new` 文件，用 `cargo insta review` 交互式审阅、`cargo insta test --review` 一步到位。
- 环境变量 `INSTA_UPDATE` 控制更新策略（`auto`/`always`/`new`/`no`…）；CI 环境下（检测到 `CI` 变量）默认不写新快照。
- 内联快照写成 `assert_snapshot!(v, @"内容")`，值就落在测试代码里；`glob!` 宏还能把「一组输入文件 × 一个断言」自动展开成多个快照。

外部依赖，写法示意：

```rust,ignore
#[test]
fn renders_config_summary() {
    let summary = render_summary(&config);
    insta::assert_snapshot!(summary);
}
```

> [!TIP]
> 快照测试真正的成本在审阅：新快照默认通过，等于把「期望值对不对」外包给人。一行精确断言能说清的事别用快照。

### 更快的运行器：cargo nextest（说明性）

`cargo nextest` 是社区写的测试运行器，不改测试代码，只换执行模型。它分两个阶段：先用 `cargo test --no-run` 构建全部测试二进制并**列出**其中的测试，再**逐个测试**起独立进程并行执行。这个「进程隔离」换来几件事：单个测试崩溃或死锁不拖垮整个二进制；失败不中止其余测试；可以按名字/二进制/平台做集合筛选（filterset 表达式），也支持重试、慢测试超时、压力循环，以及给 CI 用的归档、分片与 JUnit 报告。官方对外的说法是最高可比 `cargo test` 快约 3 倍。

代价有两处明确：

- 它**不支持文档测试**。官方原文：「Doctests are currently not supported because of limitations in stable Rust. For now, run doctests in a separate step with `cargo test --doc`.」也就是说上了 nextest，CI 里要补一步 `cargo test --doc`，否则 doctest 从此没人跑。
- 它需要额外安装，不是 cargo 自带的。

### Miri 与 loom 在 CI 里的位置

前两节的工具是在「跑同样的测试」上做文章。Miri 和 loom 不同：它们用完全不同的方式执行你的代码，专门找普通测试找不到的问题（UB 与并发交错），因此慢得多，也不会每次提交都跑。

**Miri** 是官方工具，定位一句话：「Miri is an Undefined Behavior detection tool for Rust.」它解释执行 MIR，能发现越界访问与 use-after-free、未初始化数据、对齐错误、`bool`/枚举判别值非法、数据竞争与部分弱内存效应，以及实验性的 Stacked Borrows / Tree Borrows 别名规则违例。它需要 nightly：

```console
$ rustup +nightly component add miri
$ cargo +nightly miri test
```

几条使用要点：它默认把程序与宿主隔离开（多数系统 API、网络、FFI 都不可用，需要真系统调用时用 `MIRIFLAGS="-Zmiri-disable-isolation"`）；`cargo miri` 会设置 `cfg(miri)`，可以用 `#[cfg_attr(miri, ignore)]` 跳过 Miri 支持不了的测试；在 Windows 上跑建议用 `--target x86_64-unknown-linux-gnu`；它还能配合 nextest（`cargo miri nextest run`，每个测试单独起一个 Miri 进程）——注意 nextest 的进程隔离意味着 Miri 检测不到「两个测试互相竞争同一资源」这类问题。

关于它能证明什么，官方说得很克制：Miri 不保证代码是**健全的**（sound），它只告诉你「这一次执行、这一组输入」里有没有 UB。找到 UB 就一定有问题；没找到，可能只是输入还不够（大意译自官方文档：「when Miri does not find UB, then you may just have to test more inputs or more possible non-deterministic choices」）。它对 UB 的认识也绑定当前编译器版本。CI 里它通常是一个独立 job（官方给出过 GitHub Actions 片段：装 nightly + miri 组件，跑 `cargo miri setup` 再跑 `cargo miri test`）。第 23、24 章用过 Miri，这里只关心它在流程里的位置。

还有一点和本章主题咬合：**Miri 跑的是你的测试**。测试代码没被真正编译（见前面的假绿），Miri 也就无从检查——`cargo miri test` 只跑被 `cfg(test)` 收进来的那些。

**loom** 解决另一半问题：并发代码的调度。它的定位是「Loom is a testing tool for concurrent Rust code. It runs a test many times, permuting the possible concurrent executions of that test under the C11 memory model.」——把可能的线程交错一遍遍重放，用状态归约控制组合爆炸。用法上它是一个普通依赖，只在 `cfg(loom)` 下引入：

```toml
[target.'cfg(loom)'.dependencies]
loom = "0.7"
```

测试写成 `loom::model(|| { ... })` 包住的闭包，把 `std::sync` 换成 `loom::sync`，然后用 `RUSTFLAGS="--cfg loom" cargo test --release` 触发（loom 的 quickstart 给的就是这条命令，不要求 nightly 工具链）。README 里明确列了模型的缺口：`SeqCst` 访问被当作 `AcqRel` 处理（比实际约束弱，会产生误报），load buffering 行为没有被探索（因此 loom 说没问题也可能真有问题）。所以它只适合**被显式建模到的那一小段代码**：第 19、20 章那种自己写原子类型或无锁结构的场合，把它当补充闸门，不是替代品。

下图是 CI 里这几道闸门的位置：

<svg viewBox="0 0 680 214" width="100%" role="img" aria-label="CI 闸门顺序：每次提交跑 cargo test 与可选的 nextest，每日或合并前跑 Miri 与 loom，后者只覆盖建模过的并发代码" xmlns="http://www.w3.org/2000/svg">
<text x="0" y="16" font-size="14" fill="currentColor">CI 里的闸门：越靠左越常用，越靠右越慢、覆盖面越窄</text>
<g fill="none" stroke="currentColor" stroke-opacity="0.55">
<rect x="10" y="40" width="152" height="80" rx="5" />
<rect x="178" y="40" width="152" height="80" rx="5" />
<rect x="346" y="40" width="152" height="80" rx="5" />
<rect x="514" y="40" width="156" height="80" rx="5" />
</g>
<g font-size="12" fill="currentColor">
<text x="24" y="62" font-size="13" fill="var(--links)">每次提交</text>
<text x="24" y="82" font-family="monospace">cargo test</text>
<text x="24" y="100">单元 + 集成 + doctest</text>
<text x="192" y="62" font-size="13" fill="var(--links)">可选，每次</text>
<text x="192" y="82" font-family="monospace">nextest run</text>
<text x="192" y="100" font-family="monospace">+ cargo test --doc</text>
<text x="360" y="62" font-size="13" fill="var(--warning-border)">每日 / 合并前</text>
<text x="360" y="82" font-family="monospace">cargo +nightly miri test</text>
<text x="360" y="100">UB 检测（nightly、慢）</text>
<text x="528" y="62" font-size="13" fill="var(--warning-border)">每日 / 合并前</text>
<text x="528" y="82" font-family="monospace">--cfg loom</text>
<text x="528" y="100">只覆盖建模过的并发代码</text>
</g>
<g stroke="var(--links)" stroke-width="2" fill="var(--links)">
<line x1="164" y1="80" x2="172" y2="80" /><polygon points="172,76 178,80 172,84" />
<line x1="332" y1="80" x2="340" y2="80" /><polygon points="340,76 346,80 340,84" />
<line x1="500" y1="80" x2="508" y2="80" /><polygon points="508,76 514,80 508,84" />
</g>
<text x="0" y="146" font-size="12" fill="currentColor">左边两道是「快速、全覆盖、每次跑」；右边两道是「慢、有前提、按周期跑」。</text>
<text x="0" y="170" font-size="12" fill="currentColor">Miri 跑的是你 crate 的测试——测试没被真正编译，它也就没得跑。</text>
<text x="0" y="194" font-size="12" fill="currentColor">CI 模板本身见第 30 章。</text>
</svg>

## 动手看看

`snippets/ch31/` 是一个真 crate（`ch31-demo`），把前面的规则落成可跑代码：解析日志级别与规范单词两张表、一个 `#[should_panic(expected = …)]` 测试，以及各种属性齐备的 doctest。

跑它：

```console
$ cd snippets/ch31
$ cargo test
   Compiling ch31-demo v0.1.0 (D:\Rust\snippets\ch31)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.22s
     Running unittests src\lib.rs (target\debug\deps\ch31_demo-7d0b6d46fa54106c.exe)

running 5 tests
test tests::parses_levels_case_insensitively ... ok
test tests::normalizes_words ... ok
test tests::rejects_unknown_levels ... ok
test tests::round_trips_through_as_str ... ok
test tests::rejects_zero_port - should panic ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests ch31_demo

running 6 tests
test src\lib.rs - read_level_from_file (line 124) - compile ... ok
test src\lib.rs - port_or_panic (line 109) ... ok
test src\lib.rs - port_or_panic (line 103) - should panic ... ok
test src\lib.rs - LogLevel (line 46) ... ok
test src\lib.rs - LogLevel (line 64) ... ok
test src\lib.rs - LogLevel (line 56) ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s


running 1 test
test src\lib.rs - LogLevel (line 72) - compile fail ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s

all doctests ran in 0.27s; merged doctests compilation took 0.21s
```

几处细节值得对照着读：

- `- should panic` 后缀是 libtest 给出的标记，表示这条测试属于「期望 panic」类。
- `- compile` 出现在 `read_level_from_file` 那一行：`no_run` 的块只编译不运行，所以它的结果类型是「编译」。
- **最后单独一段只有一条 `- compile fail`**：这个 crate 是 edition 2024，rustdoc 会把能合并的 doctest 合并成一个文件编译（输出里的 `merged doctests` 一行；官方说明这条行为从 **2024 edition** 起），而必须编译失败的块没法并进去，只能单独跑。
- 三个段落的顺序与入口，和前面那张分工表一一对应。

想只看某一类，用 cargo 的过滤开关：

```console
$ cargo test --lib
     Running unittests src\lib.rs (target\debug\deps\ch31_demo-7d0b6d46fa54106c.exe)

running 5 tests
…
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

```console
$ cargo test --doc
   Doc-tests ch31_demo

running 6 tests
…
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

`--lib` 只跑单元测试，`--doc` 只跑文档测试——上了 nextest 的项目的 CI 里，`cargo test --doc` 就是专门给这第三段留的位置。

最后，亲手复现一次「假绿」和它的反面。把 `snippets/ch31/src/lib.rs` 里 `normalize_word` 的实现换成只做 trim + 小写的错版（正确版本按空白切分再拼回），跑 `cargo test --lib`：

```console
$ cargo test --lib
running 5 tests
test tests::parses_levels_case_insensitively ... ok
test tests::rejects_unknown_levels ... ok
test tests::round_trips_through_as_str ... ok
test tests::normalizes_words ... FAILED
test tests::rejects_zero_port - should panic ... ok

failures:

---- tests::normalizes_words stdout ----

thread 'tests::normalizes_words' (14580) panicked at src\lib.rs:184:13:
assertion `left == right` failed: 输入 "Two  Words"
  left: "two  words"
 right: "two words"
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

failures:
    tests::normalizes_words

test result: FAILED. 4 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

error: test failed, to rerun pass `--lib`
```

注意失败信息里的 `输入 "Two  Words"`——这正是「断言带输入」的回报：表里有四行，输出直接点名是哪一行。

## 报错现场

**一、`compile_fail` 块其实编译通过了。** 你把它标成 `compile_fail`，本意是「这段代码不该能编译」，结果是这样的（`probe_ec.rs` 是探针文件名）：

```text
running 1 test
test probe_ec.rs - (line 3) - compile fail ... FAILED

failures:

---- probe_ec.rs - (line 3) stdout ----
Test compiled successfully, but it's marked `compile_fail`.

failures:
    probe_ec.rs - (line 3)

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
```

关键句是 ``Test compiled successfully, but it's marked `compile_fail` ``。多半是两种情况：代码被你改对了（那就该撤掉这个属性，改成正常示例），或者你断言的那个错误其实不存在（去读编译器真正说什么）。反过来，块因为**别的**原因失败时会静悄悄通过——这也是为什么正文里 `compile_fail` 后面必须紧跟真实的报错输出。

**二、doctest 该 panic 却没 panic。** 把 `should_panic` 块里的输入换成合法值，`cargo test --doc` 会这样报：

```text
running 6 tests
test src\lib.rs - read_level_from_file (line 124) - compile ... ok
test src\lib.rs - LogLevel (line 56) ... ok
test src\lib.rs - LogLevel (line 46) ... ok
test src\lib.rs - LogLevel (line 64) ... ok
test src\lib.rs - port_or_panic (line 103) - should panic ... FAILED
test src\lib.rs - port_or_panic (line 109) ... ok

failures:

---- src\lib.rs - port_or_panic (line 103) stdout ----
note: test did not panic as expected at src\lib.rs:103:0

failures:
    src\lib.rs - port_or_panic (line 103)

test result: FAILED. 5 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

与第 11 章 `#[should_panic]` 的失败输出有一点不同：测试名带路径与行号（`src\lib.rs - port_or_panic (line 103)`），位置后面是 `:0`，因为 doctest 没有函数体那么细的行列信息。定位方式不变：看失败名，再回去检查那个块。

**三、把测试代码留在章文件里。** 最像 bug 又最不是 bug 的场景：整章 `mdbook test` 全绿，你据此以为测试都被检查过。诊断动作只有一条——把这段代码搬进 crate 的 `#[cfg(test)] mod tests` 再跑一次 `cargo test`。

## 小结

- 三种测试按**编译配置**分工：单元测试与代码同 crate（带 `cfg(test)`，能碰私有项）；集成测试是 `tests/` 下的独立 crate（只碰公开 API）；doctest 由 rustdoc 抽取，**不开** `cfg(test)`。
- **假绿**是本章最该记住的陷阱：`#[test]` 与 `#[cfg(test)]` 里的代码在 rustdoc 通道被整块剔除，`mdbook test` 绿只保证语法合法；测试代码必须放进可跑的 crate，用 `cargo test` 真跑。
- doctest 五个模式：默认（编译 + 运行）、`ignore`（什么都不做）、`no_run`（只编译）、`should_panic`（运行且必须 panic）、`compile_fail`（必须编译失败，但不断言失败原因）。错误码校验（`compile_fail,E0308`）是 nightly-only，stable 会忽略错误码。
- 参数化靠**表驱动**：一个 `#[test]` + 一张 `(输入, 期望)` 表；断言必须带上输入，否则失败时定位不到是哪一行。
- 三类外部工具各补一角：`proptest` 用生成器 + 缩小找想不到的输入；`insta` 把大块期望输出外置成可审阅的快照；`cargo nextest` 用进程隔离换并行、重试与 CI 能力，但**不跑 doctest**，要补 `cargo test --doc`。
- Miri 与 loom 是 CI 里的周期闸门，不是每次都跑：Miri（nightly）检测 UB，只证明「这次执行没 UB」；loom 枚举并发交错，只覆盖被显式建模的那段代码，且模型本身有已知缺口。细节见第 23/24 章与第 19/20 章。

## 练习

1. 同事说：「正文里的测试代码不用管，`mdbook test` 会替我们检查。」这句话哪里不对？如果你要设计一条能抓住这类问题的检查，会加什么？

2. 你写了一个 `compile_fail` 块，想把「忘记 `use` 会报错」教给读者。审校指出：这个块里函数名还拼错了一个字母。这算问题吗？在只用 stable 的前提下，怎么防止这种块悄悄失效？

3. 下面这张表里的断言有什么缺陷？给出改法，并说说怎么用一个测试函数同时表达「大小写不敏感」和「拒绝非法输入」两类用例。

   ```rust,ignore
   #[test]
   fn parses() {
       let cases = ["info", "WARN", "Debug", "", "verbose"];
       for s in cases {
           let _ = s.parse::<LogLevel>();
       }
   }
   ```

<details><summary>参考答案</summary>

1. 不对。`mdbook test` 走的是 rustdoc 的 doctest 通道，编译时 `cfg(test)` 是关的：`#[test]` 函数与 `#[cfg(test)] mod tests` 会被整块条件编译剔除，里面的类型错误、API 误用一概不会被发现（本机 1.98 实测：块里写 `let got: i32 = "8080";`，`mdbook test -c ch31-advanced-testing.md` 仍 exit 0）。它唯一能抓住的是语法错误——解析先于条件编译。要做检查，就把测试代码放进可编译的 crate（本书的 `snippets/chNN/`），让 CI 跑 `cargo test`；正文普通示例仍交给 `mdbook test`，两者互补。

2. 算问题。`compile_fail` 只断言「编译失败」，不断言原因；这个块即便拼错的函数名改对了，也会因为别的原因失败，于是它教的就不再是「忘记 `use` 的后果」。stable 上的防法是两道：一是把块缩到最小，紧跟一个 `text` 输出块，原样贴编译器的报错（包括错误码 E0433 一类），让读者和你都能用眼睛核对；二是加一个 nightly job 用 `compile_fail,E0433` 把错误码钉死——因为错误码校验是 nightly-only（官方：stable 上错误码会被当作普通文本），stable 主流程里只能靠断言以外的证据。

3. 缺陷有三层：它只要求「不 panic」，解析成功与失败都被吞掉；用例表丢了期望值那一维；没有断言消息，失败时不知道是哪一行。改法是把两类用例拆成两张表（或一张表加一个期望字段），并把输入写进断言消息：

   ```rust,ignore
   #[test]
   fn parses_levels() {
       let ok_cases = [
           ("info", LogLevel::Info),
           ("WARN", LogLevel::Warn),
           ("  Debug ", LogLevel::Debug),
       ];
       for (input, expected) in ok_cases {
           assert_eq!(input.parse::<LogLevel>(), Ok(expected), "输入 {input:?}");
       }

       let err_cases = ["", "verbose"];
       for input in err_cases {
           assert!(input.parse::<LogLevel>().is_err(), "输入 {input:?}");
       }
   }
   ```

   拒绝类的用例也可以并进同一张表，把期望值类型从 `LogLevel` 换成 `Result<LogLevel, ConfigError>`，每行写一个期望错误值——前提是错误类型实现了 `PartialEq`，且你能在表里构造出与之等价的值（错误信息带动态部分时就不划算）。构造不出来，就退回到上面 `is_err()` 加输入的写法。

</details>

## 延伸阅读

- The Book, 第 11 章《Writing Automated Tests》：<https://doc.rust-lang.org/book/ch11-00-testing.html>
- rustdoc 书,《Documentation tests》（属性与隐藏行）：<https://doc.rust-lang.org/rustdoc/write-documentation/documentation-tests.html>
- rustdoc 书,《Unstable features》（`compile_fail,E0xxx` 错误码）：<https://doc.rust-lang.org/rustdoc/unstable-features.html>
- Cargo 书,《Tests》指南：<https://doc.rust-lang.org/cargo/guide/tests.html>
- `cargo test` 命令参考：<https://doc.rust-lang.org/cargo/commands/cargo-test.html>
- `cargo miri` 命令参考：<https://doc.rust-lang.org/cargo/commands/cargo-miri.html>
- Miri 仓库（UB 检测、CI 片段）：<https://github.com/rust-lang/miri>
- rustc,《Tests》说明（libtest 运行器与属性）：<https://doc.rust-lang.org/rustc/tests/index.html>
