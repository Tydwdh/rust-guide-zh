# 第 1 章 起步：工具链与第一个程序

## 这一章解决什么问题

Rust 不是「写完就能跑」的脚本语言：源文件要先经过编译器的完整检查，生成原生可执行文件，然后才谈运行。所以起步阶段第一件事不是背语法，而是认清三个角色——谁负责编译、谁负责管理一个项目、编译失败时去哪儿看。

读完本章你能做到：

- 装好并验证工具链（rustup、rustc、cargo）；
- 说清三者各管什么，为什么日常命令都长成 `cargo xxx`；
- 用 `cargo new` 建工程，读懂生成的 `Cargo.toml` 与 `src/main.rs`；
- 用 `check`、`build`、`run` 完成「改一行、看一眼」的循环；
- 第一次面对编译错误时，知道从哪一行开始读。

## 为什么这样设计

一个程序从文本到运行只有两件事：编译、运行。Rust 把两件事都交给你显式决定。

对比一下：Python/JavaScript 的「运行」由解释器边读边执行；C/C++ 有编译，但依赖和构建往往要另找一套工具。Rust 的选择是：**编译由 rustc 完成，项目管理由 cargo 完成，两者同属官方工具链**。于是「写 → 编译 → 运行」是一条明确的流水线：

```text
源代码  ──rustc──►  可执行文件  ──操作系统──►  运行结果
```

rustc 只认一次编译任务：给它一组源文件和相关参数，它做完整检查（类型、内存安全……）并生成机器码。它不知道什么叫「项目」「依赖版本」「测试」。这些属于 cargo：

```text
你的命令（cargo run）
   │  1. 读 Cargo.toml：包名、依赖、用哪个 profile
   ▼
cargo 编排：算出这个包（package）要编哪些 crate（编译单元）、传什么参数、产物放哪
   │  2. 调用
   ▼
rustc（真正在编译）
   │  3. 产物写入 target/
   ▼
target/debug/hello.exe  ── cargo 顺手替你执行 ──► 终端里的输出
```

再往上还有一层 rustup：工具链管理器，负责「装哪个版本的 Rust、配套工具（格式化、lint、离线文档）齐不齐」。合计三层分工：

| 层 | 工具 | 负责 |
|---|---|---|
| 工具链 | rustup | 安装、更新、切换 rustc / cargo 及其组件 |
| 项目 | cargo | 清单、依赖、构建编排，把 rustc 叫起来 |
| 编译 | rustc | 把一个 crate 编译成机器码 |

记住这个分工，命令就好记了：**涉及「这个项目」的操作走 cargo，涉及「这一堆源文件」的操作走 rustc**；日常你几乎不会直接调用 rustc。

下图把三层的关系与调用方向摊开：

<svg viewBox="0 0 660 236" width="100%" role="img" aria-label="rustup、cargo、rustc 三层分工与调用方向" xmlns="http://www.w3.org/2000/svg">
<text x="0" y="16" font-size="14" fill="currentColor">三层分工：你只敲 cargo，cargo 去调 rustc，rustup 管工具链本身</text>
<rect x="130" y="32" width="200" height="32" rx="5" fill="none" stroke="currentColor" stroke-opacity="0.55" />
<text x="230" y="53" font-size="12" text-anchor="middle" fill="currentColor">你：cargo check / build / run</text>
<g stroke="currentColor" stroke-width="1.5"><line x1="230" y1="64" x2="230" y2="80" /></g>
<path d="M226 80 L230 87 L234 80 Z" fill="currentColor" />
<rect x="40" y="87" width="380" height="52" rx="5" fill="none" stroke="currentColor" stroke-opacity="0.55" />
<text x="56" y="107" font-size="13" fill="currentColor">cargo · 项目层</text>
<text x="56" y="125" font-size="12" fill="currentColor" opacity="0.75">读 Cargo.toml，算出编哪些 crate、传什么参数、产物放哪</text>
<g stroke="currentColor" stroke-width="1.5"><line x1="230" y1="139" x2="230" y2="155" /></g>
<path d="M226 155 L230 162 L234 155 Z" fill="currentColor" />
<text x="238" y="155" font-size="12" fill="var(--links)">调用</text>
<rect x="40" y="162" width="380" height="52" rx="5" fill="none" stroke="currentColor" stroke-opacity="0.55" />
<text x="56" y="182" font-size="13" fill="currentColor">rustc · 编译层</text>
<text x="56" y="200" font-size="12" fill="currentColor" opacity="0.75">一个 crate：检查 + 生成机器码 → target/debug/hello.exe</text>
<rect x="450" y="87" width="190" height="127" rx="5" fill="none" stroke="var(--links)" stroke-opacity="0.7" />
<text x="466" y="107" font-size="13" fill="var(--links)">rustup · 工具链层</text>
<text x="466" y="128" font-size="12" fill="currentColor" opacity="0.75">装 / 更新 / 切换：</text>
<text x="466" y="147" font-size="12" fill="currentColor" opacity="0.75">rustc、cargo、rustfmt、</text>
<text x="466" y="166" font-size="12" fill="currentColor" opacity="0.75">clippy、离线文档</text>
<g stroke="currentColor" stroke-width="1.5"><line x1="447" y1="112" x2="427" y2="112" /><line x1="447" y1="189" x2="427" y2="189" /></g>
<path d="M427 108 L420 112 L427 116 Z" fill="currentColor" />
<path d="M427 185 L420 189 L427 193 Z" fill="currentColor" />
<text x="0" y="232" font-size="12" fill="currentColor" opacity="0.7">一句话：rustup 管「装什么」，cargo 管「怎么编」，rustc 管「变成机器码」。</text>
</svg>

## 机制与规则

### 安装与验证：rustup

安装入口只有一个：<https://www.rust-lang.org/tools/install>。Windows 下载 `rustup-init.exe`，安装过程中会提示你装 Visual Studio 的 C++ 生成工具，它提供链接器；Linux 与 macOS 用官方脚本 `curl --proto '=https' --tlsv1.2 https://sh.rustup.rs -sSf | sh`。装完以后：`rustup update` 升级，`rustup self uninstall` 卸载，`rustup doc` 打开本地离线文档。

装好先验证。下面是本机 Windows 上真实执行的结果：

```console
$ rustup --version
rustup 1.29.1 (d95a37b6a 2026-08-13)
info: This is the version for the rustup toolchain manager, not the rustc compiler.
info: the currently active `rustc` version is `rustc 1.98.0 (88d9e12ae 2026-08-18)`

$ rustc --version
rustc 1.98.0 (88d9e12ae 2026-08-18)

$ cargo --version
cargo 1.98.0 (797e8a9bc 2026-08-05)
```

三个命令各回答一个问题：rustup 自己是什么版本、编译器是哪个版本、项目管理器是哪个版本。注意 rustup 打印的那两行 `info:`：它在提醒你「我不是编译器」，并顺手报出当前默认工具链里的 rustc 版本——这正是上一节说的分层。`rustc x.y.z (abcdef 日期)` 是官方版本串格式：`x.y.z` 是语义化版本号，括号里是构建所用的提交号与日期。

Linux 与 macOS 上的命令完全一致，只有安装步骤不同。

### 先单独用一次 rustc

要看清 cargo 帮了什么忙，先手动编译一次。把下面两行存成 `hello.rs`：

```rust
fn main() {
    println!("Hello, world!");
}
```

然后：

```console
$ rustc --edition 2024 hello.rs
$ .\hello.exe
Hello, world!
```

（Windows 写法；Linux/macOS 下可执行文件没有 `.exe`，运行写 `./hello`。）注意 rustc 成功时一声不吭：没有输出就是好消息。产物是原生可执行文件，运行时不经过 Rust 工具链——但它默认动态链接系统 C 运行库（本机实测：导入表里有 `VCRUNTIME140.dll`、`api-ms-win-crt-runtime`），换一台 Windows 机器运行就得有对应的 MSVC 运行库；想让产物连这个也自带，编译时加 `-C target-feature=+crt-static` 改成静态链接（实测这样导入表里就不再出现 `VCRUNTIME140.dll`）。

代价是 `rustc hello.rs` 只处理你递给它的文件。依赖、多文件组织、测试、发布配置……都不在它职责内。于是我们回到 cargo。

### 建工程：cargo new

```console
$ cargo new hello
    Creating binary (application) `hello` package
note: see more `Cargo.toml` keys and their definitions at https://doc.rust-lang.org/cargo/reference/manifest.html
```

（`note:` 行只是指向 Cargo 清单规范的链接。）生成的目录：

```text
hello/
├── .git/           # cargo new 默认初始化的 git 仓库（内容省略）
├── .gitignore      # 内容只有一行 /target
├── Cargo.toml      # 包清单：名字、版本、edition、依赖
└── src/
    └── main.rs     # 程序入口
```

`cargo new` 默认还会 `git init` 帮你把仓库建好；不想要版本控制就加 `--vcs none`。

`Cargo.toml` 是纯文本清单（TOML 格式）：

```toml
[package]
name = "hello"
version = "0.1.0"
edition = "2024"

[dependencies]
```

- `[package]`：这一段描述「这是哪个包」。
- `name`：包名，也是产物文件名、将来发布到 registry 用的名字。
- `version`：语义化版本，`0.1.0` 是新建包的默认值。
- `edition`：语言方言，默认就是 `2024`，本书全部按 2024 编写；它不是编译器版本，下面「本书约定」里细说。
- `[dependencies]`：依赖表，现在是空的；加依赖就在这一节里写（第 8 章展开）。

`src/main.rs` 是 cargo 约定的入口文件：

```rust
fn main() {
    println!("Hello, world!");
}
```

`main` 是可执行程序的入口函数，操作系统从这里开始执行。`println!` 带 `!`，它是**宏（macro）**而不是普通函数；`!` 是「这是宏」的记号（第 27 章展开）。

### 日常命令：check / build / run

改一行代码、看一眼结果，这是占据你最多时间的循环。三个命令的分工如下（真实输出）：

```console
$ cargo check
    Checking hello v0.1.0 (\path\to\hello)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.87s

$ cargo run
   Compiling hello v0.1.0 (\path\to\hello)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.18s
     Running `target\debug\hello.exe`
Hello, world!
```

（为便于阅读，编译行里的项目路径统一写成 `\path\to\hello`；报错里的工具链源码路径同样省略了前缀。）

- `cargo check`：只检查到「有没有错误」为止，跳过生成机器码这一步（官方说法：`Check a local package and all of its dependencies for errors`），它不产出可执行文件——这正是它快的原因。改完代码先 `check`。
- `cargo build`：完整编译，产出可执行文件，但不运行。
- `cargo run`：先构建、再运行，等价于 build 加运行；`Running ...` 那行是 cargo 在告诉你它执行了哪个文件。

下图把三条命令各自做到哪一步摊开：

<svg viewBox="0 0 700 252" width="100%" role="img" aria-label="cargo check、build、run 三条命令各自做到哪一步、产出什么" xmlns="http://www.w3.org/2000/svg">
<text x="0" y="16" font-size="14" fill="currentColor">check / build / run：同一份代码，各自走到哪一步才停</text>
<text x="16" y="48" font-size="13" fill="currentColor" opacity="0.75">命令</text>
<text x="224" y="48" font-size="13" text-anchor="middle" fill="currentColor" opacity="0.75">① 分析检查</text>
<text x="408" y="48" font-size="13" text-anchor="middle" fill="currentColor" opacity="0.75">② 生成机器码</text>
<text x="592" y="48" font-size="13" text-anchor="middle" fill="currentColor" opacity="0.75">③ 运行程序</text>
<text x="16" y="89" font-size="13" font-family="monospace" fill="currentColor">cargo check</text>
<rect x="136" y="65" width="176" height="34" rx="5" fill="none" stroke="currentColor" stroke-opacity="0.55" />
<text x="224" y="87" font-size="12" text-anchor="middle" fill="currentColor">类型 / 借用 / 语法检查</text>
<rect x="320" y="65" width="176" height="34" rx="5" fill="none" stroke="currentColor" stroke-opacity="0.3" stroke-dasharray="4 3" />
<text x="408" y="87" font-size="12" text-anchor="middle" fill="currentColor" opacity="0.35">—</text>
<rect x="504" y="65" width="176" height="34" rx="5" fill="none" stroke="currentColor" stroke-opacity="0.3" stroke-dasharray="4 3" />
<text x="592" y="87" font-size="12" text-anchor="middle" fill="currentColor" opacity="0.35">—</text>
<text x="16" y="139" font-size="13" font-family="monospace" fill="currentColor">cargo build</text>
<rect x="136" y="115" width="176" height="34" rx="5" fill="none" stroke="currentColor" stroke-opacity="0.55" />
<text x="224" y="137" font-size="12" text-anchor="middle" fill="currentColor">类型 / 借用 / 语法检查</text>
<rect x="320" y="115" width="176" height="34" rx="5" fill="none" stroke="currentColor" stroke-opacity="0.55" />
<text x="408" y="137" font-size="12" text-anchor="middle" fill="currentColor">生成机器码 → hello.exe</text>
<rect x="504" y="115" width="176" height="34" rx="5" fill="none" stroke="currentColor" stroke-opacity="0.3" stroke-dasharray="4 3" />
<text x="592" y="137" font-size="12" text-anchor="middle" fill="currentColor" opacity="0.35">—</text>
<text x="16" y="189" font-size="13" font-family="monospace" fill="currentColor">cargo run</text>
<rect x="136" y="165" width="176" height="34" rx="5" fill="none" stroke="currentColor" stroke-opacity="0.55" />
<text x="224" y="187" font-size="12" text-anchor="middle" fill="currentColor">类型 / 借用 / 语法检查</text>
<rect x="320" y="165" width="176" height="34" rx="5" fill="none" stroke="currentColor" stroke-opacity="0.55" />
<text x="408" y="187" font-size="12" text-anchor="middle" fill="currentColor">生成机器码 → hello.exe</text>
<rect x="504" y="165" width="176" height="34" rx="5" fill="none" stroke="var(--links)" stroke-opacity="0.8" />
<text x="592" y="187" font-size="12" text-anchor="middle" fill="var(--links)">执行 hello.exe，回显输出</text>
<text x="0" y="218" font-size="12" fill="currentColor" opacity="0.7">三个命令的产物都落在 target/debug/：</text>
<text x="0" y="238" font-size="12" fill="currentColor" opacity="0.7">check 只留下指纹与增量缓存；build 与 run 额外产出 hello.exe，run 还替你执行它。</text>
</svg>

三个命令输出的动词也不同：`Checking` 表示只做分析，`Compiling` 表示真在生成机器码，`Finished` 是收尾。`Finished` 后面的 `dev` 是 **profile（构建配置）** 的名字，`[unoptimized + debuginfo]` 说明它是「不优化、带调试信息」的调试版——日常开发都跑在这一档，编译快，也方便调试。本章只讲 `check`、`build`、`run` 三个主命令，外加 `fmt`、`clippy` 两条辅助命令；cargo 的完整命令面（依赖特性、workspace、发布等）在第 30 章展开。

交付时换成优化版：

```console
$ cargo build --release
   Compiling hello v0.1.0 (\path\to\hello)
    Finished `release` profile [optimized] target(s) in 2.29s
```

`--release` 让 profile 变成 `release`，产物落在 `target/release/` 而不是 `target/debug/`：运行更快，编译更慢。一句口诀：**平时 `cargo run`，交付用 `cargo run --release`**。

所有产物都在 `target/` 下，它是可删的缓存：

```text
hello/target/debug/
├── hello.exe        # 可执行文件（Linux/macOS 下无后缀）
├── hello.d          # 依赖信息
├── deps/            # 编译过的依赖与中间产物
├── incremental/     # 增量编译缓存
├── .fingerprint/    # "这个 crate 用什么参数编的"指纹
└── build/  examples/
```

删掉重建用 `cargo clean`。给程序传参数用 `cargo run -- 参数`：`--` 之后的归你的程序，之前的归 cargo。

第一次构建还会在包根目录生成 `Cargo.lock`：这是锁文件，记录本次成功构建时各依赖的确切版本（眼下没有依赖，所以它很短）。`cargo new` 生成的 `.gitignore` 只忽略 `/target`，也就是锁文件默认纳入版本控制——`target/` 是你的编译缓存，换台机器重新编出来即可，不该提交。

下图把包根目录按「该提交」与「可删缓存」分开：

<svg viewBox="0 0 700 248" width="100%" role="img" aria-label="包根目录里哪些文件该提交、target 目录只是可重建的编译缓存" xmlns="http://www.w3.org/2000/svg">
<text x="0" y="16" font-size="14" fill="currentColor">target/ 是编译缓存：不进版本控制，删了能从源码重建</text>
<rect x="16" y="40" width="270" height="172" rx="6" fill="none" stroke="var(--links)" stroke-opacity="0.7" />
<text x="32" y="62" font-size="13" fill="var(--links)">提交进版本控制</text>
<text x="32" y="90" font-size="12" font-family="monospace" fill="currentColor">Cargo.toml</text>
<text x="160" y="90" font-size="12" fill="currentColor" opacity="0.75">包清单</text>
<text x="32" y="112" font-size="12" font-family="monospace" fill="currentColor">Cargo.lock</text>
<text x="160" y="112" font-size="12" fill="currentColor" opacity="0.75">依赖锁文件</text>
<text x="32" y="134" font-size="12" font-family="monospace" fill="currentColor">src/main.rs</text>
<text x="160" y="134" font-size="12" fill="currentColor" opacity="0.75">你的代码</text>
<text x="32" y="156" font-size="12" font-family="monospace" fill="currentColor">.gitignore</text>
<text x="160" y="156" font-size="12" fill="currentColor" opacity="0.75">内容只有 /target</text>
<text x="32" y="192" font-size="12" fill="currentColor" opacity="0.75">拷走这几样，别人就能重现构建</text>
<rect x="414" y="40" width="270" height="172" rx="6" fill="none" stroke="var(--warning-border)" stroke-dasharray="5 4" />
<text x="430" y="62" font-size="13" fill="var(--warning-border)">不进版本控制（缓存）</text>
<text x="430" y="90" font-size="12" font-family="monospace" fill="currentColor">target/debug/hello.exe</text>
<text x="430" y="112" font-size="12" font-family="monospace" fill="currentColor">target/debug/deps/</text>
<text x="430" y="134" font-size="12" font-family="monospace" fill="currentColor">target/debug/incremental/</text>
<text x="430" y="156" font-size="12" font-family="monospace" fill="currentColor">target/debug/.fingerprint/</text>
<text x="430" y="192" font-size="12" fill="currentColor" opacity="0.75">换台机器重编一遍即可</text>
<g stroke="currentColor" stroke-width="1.5">
<line x1="292" y1="92" x2="384" y2="92" />
<line x1="408" y1="162" x2="316" y2="162" />
</g>
<path d="M384 88 L391 92 L384 96 Z" fill="currentColor" />
<path d="M316 158 L309 162 L316 166 Z" fill="currentColor" />
<text x="350" y="84" font-size="12" text-anchor="middle" fill="var(--links)">cargo build 生成</text>
<text x="350" y="180" font-size="12" text-anchor="middle" fill="currentColor" opacity="0.75">cargo clean 清空</text>
<text x="0" y="234" font-size="12" fill="currentColor" opacity="0.7">别人把左边几样拉下来跑一次 cargo build，右边就回来了 —— 这就是 target/ 不必提交的原因。</text>
</svg>

再点到两个命令：`cargo fmt` 按社区统一风格重排代码（只动格式，不动语义），`cargo clippy` 是一组 lint，专门指认「能编过但很可能是写错了」的写法。它们随 rustup 默认安装（The Book 附录 D 明说 rustfmt 默认包含；用 `cargo fmt --version`、`cargo clippy --version` 可以验证），缺了就用 `rustup component add rustfmt clippy` 补上。习惯：提交前跑 `cargo fmt`；`clippy` 的提示逐条看懂再决定是否改。

### 本书约定

为免后面反复解释，把全书的底线在这里一次性钉住：

1. **edition 用 2024。** `cargo new` 默认就是 2024（`cargo new --edition` 的原文是 「Specify the Rust edition to use. Default is 2024.」），本书 `book.toml` 也设了 `edition = "2024"`，所有示例都在 2024 下编译验证。edition 是「语言方言」，不是编译器版本：edition 2024 自 Rust 1.85 起稳定，同一个 1.98 能编 2015/2018/2021/2024 四种 edition 的代码。如果报错里出现 `... is only allowed in Rust 2024 or later` 之类的提示，多半是 `Cargo.toml` 里的 `edition` 不是 2024。各 edition 的差异清单在第 33 章展开。
2. **版本基线是 stable 1.98。** 本章的 `rustc 1.98.0`、`cargo 1.98.0` 就是本书的验证环境（Windows x86_64-pc-windows-msvc）。只使用 stable 能力；万一出现 nightly-only 的内容，会显式标注。
3. **报错示例标注「以 Rust 1.98 为例」。** 编译器的措辞和建议会随版本改进，但「结论 → 位置 → 建议 → 补充」的结构不会变。
4. **命令行写法。** `$` 开头的是你要输入的命令，其余行是输出；Windows 的 cmd/PowerShell 提示符不同，命令本身一样。
5. **示例怎么跑。** 单文件片段可以用 `rustc --edition 2024 文件.rs` 直接编；带 `fn main` 的完整程序整段贴进 `src/main.rs` 再 `cargo run` 即可。偶尔出现的编译失败示例会在正文里明确标成「故意写错的反例」，别把它们当成能跑通的代码。
6. **跨章指引。** 需要别章的前提时，正文在括号里标出「（第 N 章展开）」；你可以先跳过去，不必一次读完。

## 动手看看

从零跑通一遍完整流程（本机实测）：

```console
$ cargo new hello
    Creating binary (application) `hello` package
$ cd hello
$ cargo run
   Compiling hello v0.1.0 (\path\to\hello)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.18s
     Running `target\debug\hello.exe`
Hello, world!
```

然后改 `src/main.rs`，加一行：

```rust
fn main() {
    println!("Hello, world!");
    println!("第一个程序跑起来了");
}
```

再跑一次：

```console
$ cargo run
   Compiling hello v0.1.0 (\path\to\hello)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.16s
     Running `target\debug\hello.exe`
Hello, world!
第一个程序跑起来了
```

注意第二次仍打印了 `Compiling`：cargo 发现文件变了，只重编受影响的部分（增量编译），所以明显更快。`target/` 的存在就是为这件事服务的。

## 报错现场

下面这段是**故意写错的反例**——把 `println!` 拼错成 `printlin!`，不会编译通过：

```rust,compile_fail
fn main() {
    printlin!("Hello, world!");
}
```

`cargo run` 的真实输出（以 Rust 1.98 为例）：

```text
   Compiling hello v0.1.0 (\path\to\hello)
error: cannot find macro `printlin` in this scope
   --> src\main.rs:2:5
    |
  2 |     printlin!("Hello, world!");
    |     ^^^^^^^^
    |
   ::: ...\library\std\src\macros.rs:141:1
    |
141 | macro_rules! println {
    | -------------------- similarly named macro `println` defined here
    |
help: a macro with a similar name exists
    |
  2 -     printlin!("Hello, world!");
  2 +     println!("Hello, world!");
    |

error: could not compile `hello` (bin "hello") due to 1 previous error
```

怎么读：

- `error: ...` 行是**结论**：编译器认为当前最该先修的那件事。
- `--> src\main.rs:2:5` 是**位置**：文件、行、列；下面的 `^^^^` 与代码框把出错范围指出来。
- `::: ...\library\std\src\macros.rs:141:1` 是**次级位置**：真正的出错点在你自己的文件里，但理解这条错误需要看一眼别处的定义——这里指向标准库中 `println!` 宏的定义。
- `help: ...` 是**建议**：说明「存在一个名字相近的宏」，并直接给出可以照抄的改法 `printlin!` → `println!`。
- 末尾 `error: could not compile ...` 是 cargo 的总结：本次构建失败（`cargo run` 的退出码是 101），所以程序根本没运行。

另一类常见角色是 `note:`。例如把 `src/main.rs` 清空后再 `cargo run`，输出如下（以 Rust 1.98 为例，只摘关键几行）：

```text
error[E0601]: `main` function not found in crate `hello`
  |
  = note: consider adding a `main` function to `src\main.rs`
```

`note:` 不改变结论，它是补充上下文：这里告诉你缺的是什么（`main` 函数）、该放在哪（`src\main.rs`）。本章你只需要认得这几类行；错误的层次、`E` 开头的编号怎么查、怎样从报错反推设计，是第 2 章的内容。

## 小结

- 三层分工：rustup 管工具链，cargo 管项目，rustc 管编译；你敲的 `cargo xxx` 本质上是在让 cargo 替你调用 rustc。
- 验证安装：`rustup --version`、`rustc --version`、`cargo --version`；`rustc 1.98.0 (88d9e12ae 2026-08-18)` 这样的版本串是官方格式。
- `cargo new` 生成 `Cargo.toml`（清单）加 `src/main.rs`（入口）；`main` 是执行起点，`println!` 是宏（`!`）。
- `check` 只分析（快，不产可执行文件），`build` 产可执行文件，`run` 等于构建加运行；产物在 `target/debug/`，`--release` 换成 `target/release/` 的优化版。
- 读报错按「结论（`error:`）→ 位置（`-->`）→ 建议（`help:`）→ 补充（`note:`）」的顺序；编译失败时 `cargo run` 的退出码是 101。
- 本书统一 edition 2024、stable 1.98 验证；命令行里 `$` 之后才是你要输入的内容。

## 练习

1. 你用 `rustc hello.rs` 就能编出可执行文件，为什么本书（以及几乎所有 Rust 项目）还是从 `cargo new` 开始？
2. 下面这段命令序列执行完后，`target/debug/hello.exe` 存在吗？为什么？再执行一次 `cargo run` 之后呢？

   ```console
   $ cargo new hello
   $ cd hello
   $ cargo check
   ```

3. 把 `Cargo.toml` 的 `edition = "2024"` 改成 `edition = "2015"`，程序 `fn main() { println!("Hello, world!"); }` 还能编译吗？如果换成用到 2024 新语法的代码会怎样？这说明 `edition` 是什么？

<details><summary>参考答案</summary>

1. rustc 一次只处理你递给它的文件，管不了依赖、多文件组织、测试、profile 和产物目录；这些是 cargo 的职责。cargo 的定位就是「项目级入口加构建编排」：读 `Cargo.toml`、算出该调用哪些 rustc、传什么参数、产物放哪，再调用 rustc。直接调 rustc 只适合单文件试验。

2. 不存在。`cargo check` 只做到错误检查，跳过生成机器码这一步（官方定义：「compile the packages without performing the final step of code generation」），所以 `target/debug/` 里只有指纹、增量缓存和各种锁文件，没有 `.exe`。执行 `cargo run` 后它先完整编译，才会出现 `target/debug/hello.exe`。

3. 这个只用了 `fn main` 与 `println!` 的程序在 2015 下也能编过——edition 之间不是「新旧编译器」的关系。但一旦用到 2024 才允许的语法（例如 let chains、新的 `impl Trait` 生命周期捕获规则），在 2015 下会直接报「只允许在 Rust 2024 及以后」之类的错误。edition 是**语言方言**，由 `Cargo.toml` 的 `edition` 字段按包指定；同一个编译器支持所有 edition，同一份代码在不同 edition 下也可能表现不同（第 33 章展开）。

</details>

## 延伸阅读

- The Book 第 1 章（安装与第一个程序）：<https://doc.rust-lang.org/book/ch01-00-getting-started.html>
- The Book 附录 D（rustfmt、clippy 等开发工具）：<https://doc.rust-lang.org/book/appendix-04-useful-development-tools.html>
- The Cargo Book（清单与命令规范）：<https://doc.rust-lang.org/cargo/>
- rustup 官方手册（工具链管理）：<https://rust-lang.github.io/rustup/>
- The Edition Guide（2024 edition 全量改动）：<https://doc.rust-lang.org/edition-guide/rust-2024/index.html>
- 标准库文档：<https://doc.rust-lang.org/std/>
