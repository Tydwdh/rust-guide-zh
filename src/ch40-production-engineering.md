# 第 40 章 生产工程

## 这一章解决什么问题

前 39 章回答的都是「怎么写」。这一章换个方向：**写完之后怎么交出去**。只要你打算把代码发到别人机器上跑，就会撞上三类决策：

- 崩溃时进程该怎么退场（panic 策略）；
- 要和别的语言共用一个动态库时，边界上能放什么（ABI）；
- 要给另一种架构编代码时，工具链、链接器、标准库分别从哪里来（交叉编译）。

再加两件离语言更远、但生产里躲不开的事：**自己编标准库**（build-std），和**依赖从哪里来**（registry 与 vendoring）。

读完这一章，你应该能：

- 说清 `panic = "unwind"` 与 `"abort"` 各自换来了什么，并为发布构建做出有依据的选择；
- 解释「Rust ABI 不稳定」在工程上的确切含义，知道什么该出 `cdylib`、什么该走源码依赖；
- 给一个 `#![no_std]` 库配好交叉编译，知道三元组、链接器、标准库各由谁提供；
- 知道 build-std 解决什么问题、为什么它还在 nightly；
- 分清 `[registries]`、`publish` 白名单、`cargo vendor` 各自管哪一段。

边界先画清楚：profile 每一项的含义在第 29 章 29.1；Cargo 清单的表结构与 workspace 在第 30 章；`unsafe extern`、`#[repr(C)]`、符号名与 FFI 的三条防线在第 26 章，布局规则在第 25 章；edition 与方言在第 33 章。本章只管「产物层」的决策。

> [!NOTE]
> 本章所有「实测」输出都来自本机 `rustc 1.98.0 (88d9e12ae 2026-08-18)`（stable，未装 nightly）、edition 2024、x86_64 Windows。nightly-only 的地方单独标注。报错与命令输出按「最小复现 + 关键行节选」呈现；console 块里的线程号与耗时数字每次运行都不同，粘的是当次实况。

## 为什么这样设计

「怎么写」和「怎么交」用的是两套完全不同的稳定性标准。语言层承诺过 `stable` 不破 API；**产物层没有这句话**。同一个 `pub fn`，在 rlib 里叫什么名字、`Vec<u8>` 在函数返回位置上怎么传、目标平台默认展开还是中止 panic——这些都不属于语言规范，只属于「当前这个编译器 + 当前这套参数」。

把这件事摆开看，整条交付链其实是三层：

| 层 | 交付物 | 稳定性承诺 |
|---|---|---|
| 源码层 | crate（crates.io / git / path 依赖） | 有：semver 约定 + RFC 流程 |
| 语言层 | 语法、类型、`#[repr(C)]` 的布局 | 有：stable 不破 |
| 产物层 | `.rlib` / `.dylib` / `.dll`、符号名、调用约定、panic 运行时 | 基本没有 |

所以工程化的做法不是「让产物层稳定」（做不到），而是**把不稳定的东西围到一处**：内部随便变，只在外面留一条稳定、窄、写清楚契约的缝。本章每一节都是这条原则的一个落点。

Rust Reference 给这条原则下过一个很具体的定义（Linkage 一节）：

> Rust code compiled or linked with a different instance of the Rust runtime counts as “foreign code” for the purpose of this section.

译：用**另一个** Rust 运行时实例编译或链接出来的 Rust 代码，在这一节的意义上算「外部代码」。同一节还有一条硬规矩：

> If a Rust artifact is potentially unwinding, then all its crates must be built with the unwind panic strategy. Otherwise, unwinding can cause undefined behavior.

译：如果某个 Rust 产物「可能展开」，那么它的所有 crate 都必须用 unwind 策略编译，否则展开会导致未定义行为。

换句话说：**跨编译器、跨 panic 策略、跨版本拼装产物是 UB，不是兼容性问题**。既然不能拼，就只剩两条路——要么整条链用同一个编译器一起编（源码依赖），要么把边界收窄到 C ABI。

## 机制与规则

### 40.1 panic 策略：unwind 还是 abort

第 7 章讲过两条路的样子，这一节讲怎么选。开关在 profile 里（第 29 章 29.1 的表格里有这一行）：

```toml
# snippets/ch40/panic-lab/Cargo.toml
[profile.release]
panic = "unwind"        # 默认值，展开：逐帧跑 Drop，最后退出

[profile.release-abort]
inherits = "release"
panic = "abort"         # 直接终止，不跑 Drop
```

用一个最小程序把差别跑出来——每一层留一个「析构时打一行」的哨兵，panic 处在一个 `catch_unwind` 里面：

```rust
{{#rustdoc_include ../snippets/ch40/panic-lab/src/main.rs}}
```

四次真跑（本机 Windows x86-64、Rust 1.98；`thread 'main'` 后面的线程号每次不同）：

**表 40-1　同一份源码，两档的实测对照**

| 档位 / 命令 | 标准输出的行 | 退出码 |
|---|---|---|
| `cargo run --release`（unwind + catch_unwind） | 3 行 `drop` + `catch_unwind 拦住了：true` | 0 |
| `cargo run --release -- uncaught`（unwind，不接） | 3 行 `drop` | 101 |
| `cargo run --profile release-abort`（abort + catch_unwind） | 一行也没有 | `0xc0000409` |
| `cargo run --profile release-abort -- uncaught`（abort，不接） | 一行也没有 | `0xc0000409` |

前两次的实况：

```console
$ cargo run --release
    Finished `release` profile [optimized] target(s) in 0.00s
     Running `target\release\panic-lab.exe`

thread 'main' (33608) panicked at src\main.rs:21:5:
模拟生产事故
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
  drop deepest
  drop middle
  drop main
catch_unwind 拦住了：true
```

```console
$ cargo run --profile release-abort
    Finished `release-abort` profile [optimized] target(s) in 0.00s
     Running `target\release-abort\panic-lab.exe`

thread 'main' (9608) panicked at src\main.rs:21:5:
模拟生产事故
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
error: process didn't exit successfully: `target\release-abort\panic-lab.exe` (exit code: 0xc0000409, STATUS_STACK_BUFFER_OVERRUN)
```

四行的差别值得逐条记住：

- **`abort` 下 `Drop` 一行都不跑。** `deepest`、`middle`、`main` 三层的哨兵全被跳过——因为根本没发生展开，栈是被回收的，不是被走回去的。这是「更快、更小」换来的第一笔代价：锁不会释放、缓冲不会刷盘、临时文件不会删。
- **`catch_unwind` 在 `abort` 下拦不住任何东西。** 表格里后两行输出完全一样，就是因为 `catch_unwind` 压根没机会返回。`std::panic::catch_unwind` 的文档写得很直白：「This function only catches unwinding panics, not those that abort the process.」
- **退出码换了一套。** unwind 下没接住的 panic 是 `101`；abort 下是 `0xc0000409`（PowerShell 里读作 `-1073740791`）。后者是 Windows 的快速失败状态码：`std` 在这个平台上用 `__fastfail` 实现中止（`std/src/sys/pal/windows/mod.rs` 里 `abort_internal` 的注释就写着「Use `__fastfail` to abort the process」）。
- **体积差得没传说中多。** 同一份源码，`release` 是 140 288 字节、`release-abort` 是 129 024 字节——省下 11 KB，约 8%。panic 策略影响的是「展开表 + `catch_unwind` + 格式化 panic 消息」那部分机器码，不是整个二进制。

> [!NOTE]
> `abort` 与 ABI 的组合会影响「panic 越界时到底算中止还是 UB」。Reference 的 `[items.fn.extern.unwind]` 给了一张四行表：`panic=unwind` + 展开 ABI → 正常展开；`panic=unwind` + 非展开 ABI → 中止；`panic=abort` + 展开 ABI → 不展开直接中止；`panic=abort` + 非展开 ABI → 不展开直接中止。它还补了一句（译自该节）：panic 在一个非展开 ABI 边界上被转成中止时，要么一个析构函数都不跑，要么边界之前的析构函数全跑，具体是哪种未指定。这条和 FFI 的对应关系在第 26 章 26.6 讲过。

**怎么选**，按「谁承担后果」来分：

- **库代码不要替使用者做这个决定。** 一个 crate 把自己编成 abort，就把使用者的 `catch_unwind`、把使用者的 worker 池、把使用者的插件隔离全废了。库代码的正确姿势是让错误走 `Result`（第 18 章），把 panic 留给「真的不该发生」的情况。
- **终产物（服务、CLI、固件）自己决定。** 选 `abort` 的常见理由：不需要 `catch_unwind` 兜底、希望崩溃即刻停止而不是带着半截状态继续跑、在意体积与退出速度；此外 `no_std` 目标的默认值本来就是 abort（40.3 会实测到）。选 `unwind` 的常见理由：需要进程内隔离（项目 3 的任务池就是——它靠 `catch_unwind` 把任务 panic 关在 worker 里）、要跑依赖展开的测试框架、要跨 FFI 传播（第 26 章的 `-unwind` ABI）。
- **不要在不同的 crate 里混用。** 上面 Reference 那条规矩就是为此写的：一个「可能展开」的产物，其全部 crate 必须都是 unwind 策略。rustc 在编译期要求同一次编译里的 panic 策略一致，常规混搭会被挡住；但把 rlib 直接交给别的链接器（或用 `dlopen` 那一类方式装载）时挡不住——一致性由你负责。
- **测试档位会忽略它。** `test` / `bench` 档位必须展开（否则测试框架没法工作），Cargo 对它们的 `panic` 设置不生效——所以「我在 release 里开了 abort，测试没发现问题」是正常现象（第 29 章 29.1 表格里 `panic` 那一行）。

### 40.2 动态库与 ABI：cdylib 是唯一的稳定缝

先把「Rust ABI 不稳定」这句口号变成能看见的东西。写一个最小 crate：

```rust
// counter.rs —— 一个最小 crate
pub struct Counter {
    hits: u64,
}

impl Counter {
    pub fn hits(&self) -> u64 {
        self.hits
    }
}
```

同一份源码编两次 rlib，唯一区别是 `-C metadata`：

```console
$ rustc --edition 2024 --crate-type rlib --crate-name counter -C metadata=alpha counter.rs -o alpha.rlib
$ rustc --edition 2024 --crate-type rlib --crate-name counter -C metadata=beta counter.rs -o beta.rlib
$ llvm-nm --defined-only alpha.rlib | findstr 4hits
00000000 T _RNvMCsdi1eJHOR88G_7counterNtB2_7Counter4hits
$ llvm-nm --defined-only beta.rlib | findstr 4hits
00000000 T _RNvMCsa8Xayzzdo2I_7counterNtB2_7Counter4hits
```

（`llvm-nm` 来自 `rustup component add llvm-tools`，在 `~/.rustup/toolchains/<版本>/lib/rustlib/<宿主三元组>/bin/` 下。）

`alpha` 与 `beta` 里那两串 `Csdi1eJHOR88G` / `Csa8Xayzzdo2I` 就是哈希。**源码一个字没改、编译器还是同一个，符号名就变了**——而链接期认的就是符号名。这就是「Rust ABI 不稳定」的字面含义：它不只是「将来可能变」，而是同一个 crate 换一组编译参数就能变。rustc 手册里 `-C symbol-mangling-version` 的说明也写着，默认的改名方案是「a compiler-chosen default which may change in the future」（由编译器选定的默认值，将来可能改变）。所以 Rust 的符号名与名字修饰**不是接口**，是编译器内部约定。

推论只有一条：**跨语言的缝只能用 C ABI**。`extern "C"`（调用约定）+ `#[repr(C)]`（布局，第 25 章）+ `#[unsafe(no_mangle)]`（名字别改）三件套，缺一不可。Nomicon 对 `no_mangle` 的定位很准确：「The `no_mangle` attribute turns off Rust's name mangling, so that it has a well defined symbol to link to.」

下图把这条缝两侧的名字体系摊开：

<svg viewBox="0 0 720 210" width="100%" role="img" aria-label="同一个 crate 的两套名字：C 侧只看到 no_mangle 导出的稳定符号名与 repr(C) 布局，Rust 内部的名字带哈希且随编译参数变化" xmlns="http://www.w3.org/2000/svg">
<text x="0" y="16" font-size="14" fill="currentColor">同一份代码，两套名字体系</text>
<rect x="0" y="30" width="700" height="44" rx="6" fill="none" stroke="currentColor" stroke-width="1.5"/>
<text x="12" y="50" font-size="12" fill="currentColor">C 侧：只看得到 C 类型</text>
<text x="12" y="68" font-size="12" font-family="monospace" fill="currentColor">extern uint64_t counter_add(void *h, uint64_t n);</text>
<rect x="0" y="86" width="700" height="44" rx="6" fill="none" stroke="var(--links)" stroke-width="2"/>
<text x="12" y="106" font-size="12" fill="var(--links)">C ABI 缝：稳定的一侧</text>
<text x="12" y="124" font-size="12" font-family="monospace" fill="currentColor">counter_add / counter_free / counter_new / summarize + #[repr(C)]</text>
<rect x="0" y="142" width="700" height="44" rx="6" fill="none" stroke="currentColor" stroke-width="1.5" stroke-dasharray="4 3"/>
<text x="12" y="162" font-size="12" fill="currentColor">Rust 内部：不稳定的那一侧</text>
<text x="12" y="180" font-size="12" font-family="monospace" fill="currentColor">_RNvMCsdi1eJHOR88G_7counterNtB2_7Counter4hits</text>
</svg>

`cargo` 里对应的 crate 类型是 `cdylib`（Reference 的说明：「A dynamic system library will be produced. This is used when compiling a dynamic library to be loaded from another language.」）。完整例子在 `snippets/ch40/ffi-lab/`：Rust 侧只暴露 C 能懂的类型，内部状态藏在不透明句柄后面。

```rust
{{#rustdoc_include ../snippets/ch40/ffi-lab/src/lib.rs}}
```

编出来看看它到底导出了什么：

```console
$ cargo build --release
   ...
warning: linker stdout:   正在创建库 ...\pointops.dll.lib 和对象 ...\pointops.dll.exp
    Finished `release` profile [optimized] target(s) in 0.21s
$ llvm-readobj --coff-exports target\release\pointops.dll | findstr Name
  Name: counter_add
  Name: counter_free
  Name: counter_new
  Name: summarize
```

导出的名字只有这四个。四个都是 `#[unsafe(no_mangle)]` 的那几个——`Counter::hits` 这个 Rust 侧方法在导出表里根本不存在，因为 C 拿不到它，也不该拿。

> [!NOTE]
> 上面那条 `linker stdout` 警告不是你的错：MSVC 链接器在创建 `.lib` 导入库时会往 stdout 打一行中文进度，rustc 把「链接器输出了没预期的东西」报成 `linker_messages` 警告。它在 1.98 上是默认提醒，不影响产物。

接着写 C 侧的宿主，链接到刚编出来的 `.dll`（本机装了 MinGW-W64 的 gcc 16.1.0）：

```c
{{#include ../snippets/ch40/ffi-lab/host.c}}
```

```console
$ gcc host.c -o host.exe target\release\pointops.dll
$ copy target\release\pointops.dll .
$ .\host.exe
add(3) = 3
add(4) = 7
sum = 10, count = 4
```

到这里，一个真正的跨语言边界就跑通了：C 建句柄、调方法、释放句柄，Rust 内部用 `Box` 管理内存；`#[repr(C)]` 的 `Summary` 按值来回传。注意 C 侧只写了一个 `void *`——它完全不知道 `Counter` 有几个字段、什么对齐，这正是「不透明句柄」的用法（第 26 章 26.8 的同一思路）。

反过来，如果把 Rust 私有类型放到边界上会怎样？rustc 会在**编译期**警告：

```rust
#[unsafe(no_mangle)]
pub extern "C" fn bad(s: String) -> String {
    s
}
```

```text
warning: `extern` fn uses type `String`, which is not FFI-safe
 --> a.rs:2:26
  |
2 | pub extern "C" fn bad(s: String) -> String {
  |                          ^^^^^^ not FFI-safe
  |
  = help: consider adding a `#[repr(C)]` or `#[repr(transparent)]` attribute to this struct
  = note: this struct has unspecified layout
  = note: `#[warn(improper_ctypes_definitions)]` on by default

warning: `extern` fn uses type `String`, which is not FFI-safe
 --> a.rs:2:37
  |
2 | pub extern "C" fn bad(s: String) -> String {
  |                                     ^^^^^^ not FFI-safe
  |
  = help: consider adding a `#[repr(C)]` or `#[repr(transparent)]` attribute to this struct
  = note: this struct has unspecified layout

warning: 2 warnings emitted
```

注意这两条都只是 **warning**（来往两个方向各一条），并不阻止你编出产物——所以它们必须当门槛看待。`String` 的布局取决于 `std` 的私有决定，一边编、另一边读就回到了本节开头那个禁区。

最后是本节最容易搞反的一句：**Rust 之间共享代码，正确方式是源码依赖（crate/rlib），不是 dylib。** 一个 `.dylib` 只有在「同一轮构建、同一个编译器、同一 edition、同一 panic 策略、同一套 feature」时才有意义，而这些前提一个都不在稳定承诺内。要么 `cargo` 一起编（自动满足），要么把边界收窄到 C ABI（本节的做法）。

### 40.3 交叉编译：三元组、链接器、标准库

交叉编译要同时备齐三样东西：**编译器要支持这个目标、链接器能产出这个目标的产物、标准库要有这个目标的预编译版本**。`rustc --print target-list` 在本机列出 330 个目标；`rustup target add` 装的是后面那两样里的「标准库」部分。

名词先厘清。目标的名字叫**三元组**，习惯写法是 `架构-厂商-系统-ABI`：

<svg viewBox="0 0 720 196" width="100%" role="img" aria-label="目标三元组按架构、厂商、系统、ABI 四段拆开，四段分别决定指令集、名称标记、有没有标准库与链接器、浮点与调用约定" xmlns="http://www.w3.org/2000/svg">
<text x="0" y="16" font-size="14" fill="currentColor">目标三元组：四段，各管一件事</text>
<text x="85" y="46" font-size="12" fill="currentColor" text-anchor="middle">架构</text>
<text x="265" y="46" font-size="12" fill="currentColor" text-anchor="middle">厂商</text>
<text x="445" y="46" font-size="12" fill="currentColor" text-anchor="middle">系统</text>
<text x="625" y="46" font-size="12" fill="currentColor" text-anchor="middle">ABI / 环境</text>
<rect x="0" y="54" width="170" height="30" rx="4" fill="none" stroke="currentColor" stroke-width="1.5"/>
<rect x="180" y="54" width="170" height="30" rx="4" fill="none" stroke="currentColor" stroke-width="1.5"/>
<rect x="360" y="54" width="170" height="30" rx="4" fill="none" stroke="currentColor" stroke-width="1.5"/>
<rect x="540" y="54" width="170" height="30" rx="4" fill="none" stroke="currentColor" stroke-width="1.5"/>
<text x="85" y="74" font-size="12" font-family="monospace" fill="currentColor" text-anchor="middle">x86_64</text>
<text x="265" y="74" font-size="12" font-family="monospace" fill="currentColor" text-anchor="middle">pc</text>
<text x="445" y="74" font-size="12" font-family="monospace" fill="currentColor" text-anchor="middle">windows</text>
<text x="625" y="74" font-size="12" font-family="monospace" fill="currentColor" text-anchor="middle">msvc</text>
<rect x="0" y="90" width="170" height="30" rx="4" fill="none" stroke="var(--links)" stroke-width="1.5"/>
<rect x="180" y="90" width="170" height="30" rx="4" fill="none" stroke="var(--links)" stroke-width="1.5"/>
<rect x="360" y="90" width="170" height="30" rx="4" fill="none" stroke="var(--links)" stroke-width="1.5"/>
<rect x="540" y="90" width="170" height="30" rx="4" fill="none" stroke="var(--links)" stroke-width="1.5"/>
<text x="85" y="110" font-size="12" font-family="monospace" fill="var(--links)" text-anchor="middle">thumbv7em</text>
<text x="265" y="110" font-size="12" font-family="monospace" fill="var(--links)" text-anchor="middle">none</text>
<text x="445" y="110" font-size="12" font-family="monospace" fill="var(--links)" text-anchor="middle">none</text>
<text x="625" y="110" font-size="12" font-family="monospace" fill="var(--links)" text-anchor="middle">eabihf</text>
<text x="85" y="142" font-size="11" fill="currentColor" text-anchor="middle">指令集与指针宽度</text>
<text x="85" y="157" font-size="11" fill="currentColor" text-anchor="middle">（cfg: arm）</text>
<text x="265" y="142" font-size="11" fill="currentColor" text-anchor="middle">只是个名称标记</text>
<text x="265" y="157" font-size="11" fill="currentColor" text-anchor="middle">（cfg: unknown）</text>
<text x="445" y="142" font-size="11" fill="currentColor" text-anchor="middle">有没有 std、</text>
<text x="445" y="157" font-size="11" fill="currentColor" text-anchor="middle">用哪个链接器</text>
<text x="625" y="142" font-size="11" fill="currentColor" text-anchor="middle">浮点与调用约定</text>
<text x="625" y="157" font-size="11" fill="currentColor" text-anchor="middle">（cfg: eabihf）</text>
<text x="0" y="184" font-size="12" fill="currentColor" opacity="0.75">四段里任何一段不对，编出来的目标文件都不是给那台机器准备的。</text>
</svg>

别把名字当字段解析：真正决定编译的是 rustc 里的**目标定义**，名字只是它的简称，所以两者对不上很正常。`rustc --print cfg` 能把实际字段读出来（本机实测，节选）：

```console
$ rustc --print cfg --target thumbv7em-none-eabihf
debug_assertions
panic="abort"
target_abi="eabihf"
target_arch="arm"
target_endian="little"
target_env=""
target_os="none"
target_pointer_width="32"
target_vendor="unknown"
```

对着看：`thumbv7em` 这段带的是 CPU 型号，`target_arch` 其实是 `arm`；名字里写 `none` 的厂商段，实际 `target_vendor` 是 `unknown`。`x86_64-linux-android` 读出来则是 `target_arch="x86_64"`、`target_os="android"`，`panic="unwind"`。

注意最后那行 `panic="abort"` 与安卓的 `panic="unwind"`——**目标自己带默认 panic 策略**，这一点直接决定了下一个实验的成败。

拿一块真实的库来试。`snippets/ch40/fwlib/` 是一个只依赖 `core` 的环形缓冲 + CRC-32 库，编成 `staticlib`（给固件链接用的 `.a`）：

交叉编译用的库不能当 doctest 编译（它声明了 `#![no_std]`），所以这一段只展示、由 `cargo build --target` 与 `cargo test` 把关：

```rust,ignore
{{#include ../snippets/ch40/fwlib/src/lib.rs}}
```

主机上先跑它的测试，确认逻辑没问题：

```console
$ cargo test
running 2 tests
test tests::crc32_matches_known_vector ... ok
test tests::ring_buffer_evicts_oldest ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

（`#![cfg_attr(not(test), no_std)]` 这一行让它在宿主机跑测试时打开 `std`、交叉编译时关掉——嵌入式库的常规写法。）

然后给两个 thumb 目标编：

```console
$ cargo build --target thumbv7m-none-eabi
   Compiling fwlib v0.1.0 (D:\Rust\snippets\ch40\fwlib)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.07s
$ cargo build --target thumbv7em-none-eabihf
   Compiling fwlib v0.1.0 (D:\Rust\snippets\ch40\fwlib)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.10s
```

产物是 `target/thumbv7m-none-eabi/debug/libfwlib.a`（7 998 358 字节）与 `target/thumbv7em-none-eabihf/debug/libfwlib.a`（8 098 996 字节），用 llvm 的工具看一眼文件格式：

```console
$ llvm-readobj --file-headers target\thumbv7m-none-eabi\debug\libfwlib.a | findstr Format
Format: elf32-littlearm
```

（`.a` 是一个装了多个目标文件的归档，所以这条命令对每个成员各打一行、内容全一样；上面只留一行。`thumbv7em-none-eabihf` 那份同样是 `Format: elf32-littlearm`。）

**这就是「真跑通」的样子**：一个 32 位小端 ARM 的 ELF 静态库，可以直接丢给固件的链接器。注意这里没有装任何 ARM 的 C 工具链——因为 `staticlib` 不需要最终链接器，`cargo` 用的是随工具链分发的归档器。

换成 Android 目标，同一个 crate 立刻撞墙：

```console
$ cargo build --target x86_64-linux-android
   Compiling fwlib v0.1.0 (D:\Rust\snippets\ch40\fwlib)
error: unwinding panics are not supported without std
  |
  = help: using nightly cargo, use -Zbuild-std with panic="abort" to avoid unwinding
  = note: since the core library is usually precompiled with panic="unwind", rebuilding your crate with panic="abort" may not be enough to fix the problem

error: could not compile `fwlib` (lib) due to 1 previous error
```

原因在刚才那张 `--print cfg` 表里：Android 的默认策略是 `unwind`，而不带 `std` 的 crate 给不出展开所需的那套运行时（`eh_personality` 之类）。thumb 目标之所以没这个问题，是因为它默认就是 `abort`。

按提示换策略即可（`--config` 是 Cargo 的命令行配置覆盖，等价于临时往 profile 里写一行）：

```console
$ cargo build --target x86_64-linux-android --config 'profile.dev.panic="abort"'
   Compiling fwlib v0.1.0 (D:\Rust\snippets\ch40\fwlib)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.14s
$ llvm-readobj --file-headers target\x86_64-linux-android\debug\libfwlib.a | findstr Format
Format: elf64-x86-64
```

现在再看「库」与「可执行文件」的差别——**只有后者才需要目标平台的链接器**：

```console
$ cargo build --target x86_64-linux-android
   Compiling hellobin v0.1.0 (...\hellobin)
error: linker `cc` not found
  |
  = note: program not found

error: could not compile `hellobin` (bin "hellobin") due to 1 previous error
```

（上例是一个 `cargo new` 出来的普通可执行项目。库不需要链接器，所以 `libfwlib.a` 能直接出来；可执行文件要链接成 ELF，就得指给它一个 Android 工具链里的链接器。）

配置在 `.cargo/config.toml` 里，键名是 `[target.<三元组>]`（下面这段按 Cargo 手册写，本机没装 NDK，未实测）：

```toml
[target.x86_64-linux-android]
linker = "C:\\Android\\ndk\\toolchains\\llvm\\prebuilt\\windows-x86_64\\bin\\clang.exe"
runner = ""   # 想让 `cargo run` 自动跑起来，这里填模拟器/设备上的执行包装
```

在本机没有目标链接器时，能直接用的替代方案有两类：**只编库**（如上）、或者**让链接由目标平台自己做**（先把 `.a` 交出去）。第三类方案正是下一节的 build-std：既然 `std` 的预编译版本不合用，那就自己编。

### 40.4 build-std：自己编标准库（nightly）

**解决的问题**：`rustup target add` 只能拿到官方预编译的那份 `std`/`core`。三种情况下这份不合用——目标根本没有官方 `std`、想让 `std` 跟这次构建用同一套 profile（官方那份的参数是固定的）、想自己挑 `std` 的 feature。build-std 让 Cargo 把标准库当成依赖图里的普通 crate，从源码编一遍。

先看边界：它是 nightly-only。本机只有 stable 1.98，于是命令行直接给了这句话：

```console
$ cargo build -Z build-std --target thumbv7m-none-eabi
error: the `-Z` flag is only accepted on the nightly channel of Cargo, but this is the `stable` channel
See https://doc.rust-lang.org/book/appendix-07-nightly-rust.html for more information about Rust release channels.
```

> [!WARNING]
> `-Z build-std` 是 nightly 专属的不稳定功能（Cargo 手册把它列在 Unstable 一节，跟踪仓库是 `rust-lang/wg-cargo-std-aware`），接口与行为都在变。下面只给文档化的用法，本书示例一律不依赖它。要用就得装 nightly：
> ```console
> $ rustup toolchain install nightly
> $ rustup component add rust-src --toolchain nightly
> ```

按 Cargo 手册，今天的用法是：

```console
$ cargo +nightly build -Z build-std --target thumbv7m-none-eabi
$ cargo +nightly build -Z build-std=core,alloc --target <三元组>   # 只要这两个，比如不带 std 的固件
$ cargo +nightly build -Z build-std-features=panic-unwind --target <三元组>
```

手册列的**三条硬要求**（原文照译）：

- 必须通过 `rustup component add rust-src` 装上标准库源码；
- 必须用 nightly 的 Cargo **和** nightly 的 rustc；
- `-Z build-std` 必须出现在**每一次** `cargo` 调用里。

不传时默认编 `core`、`std`、`alloc`、`proc_macro` 四个（跑 `cargo test` 还会加上 `test`）；`-Z build-std-features` 用来设定编 `std` 时开哪些 feature，默认是 `backtrace` 与 `panic-unwind`。

它和 40.3 的墙正好对上：那句 `= help: using nightly cargo, use -Zbuild-std with panic="abort"` 说的就是「你想要一份按 abort 编的标准库」。代价是每次构建都要连 `core` 一起编，首次构建明显变慢——所以它是「目标平台需要」时的工具，不是日常开关。

### 40.5 供应链：registry 与 vendor

依赖从哪里来，是生产环境里另一个必须能回答的问题。默认答案是 crates.io，但 Cargo 允许换：一个 registry 由**索引（index）**和可选的 **web API** 组成，`[registries]` 表把「名字 → 索引地址」记下来（Cargo 手册的示例）：

```toml
# .cargo/config.toml
[registries]
my-registry = { index = "https://my-intranet:8080/git/index" }
```

名字起好后，依赖里用 `registry` 键指过去：

```toml
[dependencies]
other-crate = { version = "1.0", registry = "my-registry" }
```

没配置就去用它，Cargo 会说得很清楚（本机实测）：

```console
$ cargo publish --registry=my-registry --dry-run
error: registry index was not found in any configuration: `my-registry`
```

同样地，`cargo login --registry=my-registry` 报的是同一句——`login` / `publish` / `install` 这些命令都要先能定位到那个索引。

私有 registry 场景里有三个配置值得记住：

- **默认 registry**：配置里 `[registry] default = "my-registry"` 可以改掉「不写 `--registry` 时用哪个」。
- **发布白名单**：`[package] publish = ["my-registry"]` 限制这个包只允许发到指定 registry；`publish = false` 就是全禁。写错了会得到一个明确拒绝（本机实测）：
  ```console
  $ cargo publish --dry-run
  error: `pubprobe` cannot be published.
  `package.publish` must be set to `true` or a non-empty list in Cargo.toml to publish.
  ```
  这条是防「把内部包误发到 crates.io」的第一道闸：它写在清单里，随代码一起 review。
- **索引协议**：`git` 或 `sparse`。索引 URL 带 `sparse+` 前缀就走 sparse（按需下载单个元数据文件，省带宽）；crates.io 两种都支持，用 `registries.crates-io.protocol` 选。

不能连外网、或者要把依赖钉死在自己的机器上时，用 vendoring：把依赖的源码整份拷进仓库。Cargo 自带 `cargo vendor`（本机实测，`CARGO_NET_OFFLINE=true`，依赖是缓存里的 `cfg-if`）：

```console
$ cargo vendor
     Locking 1 package to latest Rust 1.98.0 compatible version
   Vendoring cfg-if v1.0.5 (C:\Users\tyd27\.cargo\registry\src\index.crates.io-1949cf8c6b5b557f\cfg-if-1.0.5) to vendor\cfg-if
To use vendored sources, add this to your .cargo/config.toml for this project:

[source.crates-io]
replace-with = "vendored-sources"

[source.vendored-sources]
directory = "vendor"
```

把上面那段贴进 `.cargo/config.toml` 之后，离线构建真的能跑起来：

```console
$ cargo build --offline
   Compiling cfg-if v1.0.5
   Compiling vendprobe v0.1.0 (...\vendprobe2)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.14s
```

机制是**源替换（source replacement）**：`replace-with` 把 crates.io 整个换成一个本地目录，于是构建不再碰网络，依赖内容也被你一起提交、一起审计。代价同样明显——`vendor/` 会显著撑大仓库，升级依赖得重新 vendor。适合的场景是「构建必须在无网环境里可复现」，不适合当默认做法。

> [!TIP]
> 供应链还有两个字：**审计**与**漏洞**。前者看的是「这次构建里到底有哪些包」，`cargo tree`、`cargo metadata` 是现成工具；后者看的是「其中哪些有已知漏洞」，社区的做法是订阅 RustSec 的 advisory 数据库，用 `cargo audit` / `cargo deny` 这类工具在 CI 里卡住。它们不是 Cargo 自带，但都是命令行工具，接进 CI 就是一条命令的事。

### 40.6 加固选项速览

交付一条二进制时，还有一类「默认已经开了，但值得确认」的东西——编译产物里的漏洞利用缓解措施（exploit mitigations）。rustc 手册专门有一章列它，适用口径是「Linux 操作系统 + AMD64」（原文限定），本机是 Windows，所以顺手看一眼 PE 头部的等价信息：

```console
$ llvm-readobj --file-headers target\release\panic-lab.exe | findstr IMAGE_DLL_CHARACTERISTICS
    IMAGE_DLL_CHARACTERISTICS_DYNAMIC_BASE (0x40)
    IMAGE_DLL_CHARACTERISTICS_HIGH_ENTROPY_VA (0x20)
    IMAGE_DLL_CHARACTERISTICS_NX_COMPAT (0x100)
    IMAGE_DLL_CHARACTERISTICS_TERMINAL_SERVER_AWARE (0x8000)
```

`DYNAMIC_BASE`（配合 `HIGH_ENTROPY_VA`）= ASLR：每次加载地址随机；`NX_COMPAT` = 数据页不可执行；最后一行是终端服务相关的标记，和加固无关。前两个在 Windows 上是默认开的，和手册里 Linux 侧「PIE 默认开、NX 默认开」对应。

手册那张表的关键信息是**哪些默认开、哪些要 nightly 手动开**：

| 加固 | 默认 | 开关 |
|---|---|---|
| 位置无关可执行文件（PIE，full ASLR） | 是 | 默认 |
| 不可执行内存段（NX） | 是 | 默认 |
| 栈冲突保护（stack probe） | 是 | 默认 |
| 只读重定位与立即绑定（RELRO） | 是 | 默认 |
| 堆损坏检查 | 是 | 由分配器提供 |
| 整数溢出检查 | 随 debug assertions | `overflow-checks`（第 29 章） |
| 栈溢出保护（stack protector） | 否 | `-Z stack-protector`（nightly） |
| 前向边控制流保护（CFI） | 否 | `-Z sanitizer=cfi`（nightly） |
| 后向边控制流保护 | 否 | `-Z sanitizer=shadow-call-stack,safestack`（nightly） |

> [!WARNING]
> 最后三行都是 nightly 专属（`-Z` 开头），本机没有 nightly，未实测。它们对应的 C/C++ 世界里的 `-fstack-protector` / CFI，生产里要开就得走 nightly 构建链。

取舍和 40.4 一样：默认那几条是白送的，nightly 那几条要用就得接受 nightly 构建链。

## 动手看看

本章三个实验都是完整可跑的 crate，位置和跑法：

**（1）panic 策略对照**：`snippets/ch40/panic-lab/`

```console
$ cd snippets/ch40/panic-lab
$ cargo run --release                    # 展开 + 捕获：跑 3 行 drop，退出码 0
$ cargo run --release -- uncaught        # 展开不捕获：跑 3 行 drop，退出码 101
$ cargo run --profile release-abort      # 中止：一行不跑，退出码 0xc0000409
```

**（2）cdylib + C 宿主**：`snippets/ch40/ffi-lab/`

```console
$ cd snippets/ch40/ffi-lab
$ cargo build --release
$ gcc host.c -o host.exe target\release\pointops.dll
$ copy target\release\pointops.dll .
$ .\host.exe
add(3) = 3
add(4) = 7
sum = 10, count = 4
```

**（3）交叉编译**：`snippets/ch40/fwlib/`

```console
$ cd snippets/ch40/fwlib
$ cargo test                                                       # 宿主机逻辑自测
$ cargo build --target thumbv7m-none-eabi                          # 裸机 Cortex-M3
$ cargo build --target thumbv7em-none-eabihf                       # 带硬浮点的 Cortex-M4F/M7
$ cargo build --target x86_64-linux-android                        # 不带 --config 会失败，见 40.3
$ cargo build --target x86_64-linux-android --config 'profile.dev.panic="abort"'
```

一句话收束：**先问「这个产物要在什么上跑」，再问「用什么编」**——目标决定链接器和标准库，标准库决定 panic 策略能不能这么选。

## 报错现场

### 现场一：no_std 静态库缺 panic 处理器

最小复现：把 `snippets/ch40/fwlib/` 里的 `#[panic_handler]` 那段删掉，再给 thumb 编：

```console
$ cargo build --target thumbv7m-none-eabi
   Compiling fwlib v0.1.0 (D:\Rust\snippets\ch40\fwlib)
error: `#[panic_handler]` function required, but not found

error: could not compile `fwlib` (lib) due to 1 previous error
```

定位路径：`staticlib` 是**最终产物**，链接期必须能解析出一个唯一的 panic 处理器；带 `std` 时它由 `std` 提供，`#![no_std]` 就得自己写。它不是「少了点可选的装饰」，而是缺一个必须存在的符号。修法（`fwlib` 里就是这么写的）：

```rust,ignore
#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {
        core::hint::spin_loop();
    }
}
```

（这段只在 `#![no_std]` 的 crate 里成立，放进 doctest 会和 `std` 自带的那个撞车，所以标了 `ignore`；`snippets/ch40/fwlib/src/lib.rs` 里有它的完整上下文。）

配 `#[cfg(not(test))]` 是为了让宿主机的 `cargo test` 仍用测试框架自带的那一个。真实固件不会在这里自旋，而是复位看门狗或点亮错误灯——顺便说明：**这个函数的实现方式，就是 40.1 里 panic 策略在 `no_std` 世界的对应物**。

### 现场二：给裸机目标编一个 std 程序

最小复现：`cargo new hellobin` 之后直接：

```console
$ cargo build --target thumbv7m-none-eabi
   Compiling hellobin v0.1.0 (...\hellobin)
error[E0463]: can't find crate for `std`
  |
  = note: the `thumbv7m-none-eabi` target may not support the standard library
  = note: `std` is required by `hellobin` because it does not declare `#![no_std]`

error: cannot resolve a prelude import

error: cannot find macro `println` in this scope
 --> src\main.rs:2:5
  |
2 |     println!("Hello, world!");
  |     ^^^^^^^

error: `#[panic_handler]` function required, but not found

For more information about this error, try `rustc --explain E0463`.
error: could not compile `hellobin` (bin "hellobin") due to 4 previous errors
```

四条错误是同一件事的连锁反应：这个目标**没有预编译的 `std`**——`rustup target add thumbv7m-none-eabi` 装的那份标准库里只有 `libcore`、`liballoc`、`libcompiler_builtins`，没有 `libstd`。于是 prelude 没了、`println!` 没了，连 panic 处理器也没了（它平时由 `std` 提供，见现场一）。定位路径：**先确认目标有没有 `std`**：看工具链里该目标的 `lib` 目录下有没有 `libstd`，或者用 `rustc --print cfg --target <三元组>` 看 `target_os` / `panic`，最省事的办法是直接试着编一个库。修法有两条——写 `#![no_std]` 的库（`fwlib`），或者用 40.4 的 build-std 自己编一份。

### 现场三：给 Android 编可执行文件

最小复现：同一个 `hellobin`：

```console
$ cargo build --target x86_64-linux-android
   Compiling hellobin v0.1.0 (...\hellobin)
error: linker `cc` not found
  |
  = note: program not found

error: could not compile `hellobin` (bin "hellobin") due to 1 previous error
```

定位路径：`x86_64-linux-android` 的 `std` 是装好的（同一目标编库能过），缺的是**链接器**——rustc 按这个目标的默认设置在找名为 `cc` 的链接器，而这台 Windows 机器上没有叫 `cc` 的程序。修法就是在 `.cargo/config.toml` 的 `[target.x86_64-linux-android]` 里指定 `linker`（40.3 给了配置形状；本机没装 NDK，所以只走到这一步）。顺带一条排错经验：**「编库过、编可执行文件不过」几乎总是链接器问题**，不是代码问题。

## 小结

- 产物层没有稳定性承诺：符号名、ABI、panic 运行时都属于「当前编译器 + 当前参数」，所以工程做法是把不稳定的部分围起来，只留一条窄缝。
- `panic = "abort"` 换来的是更快更小的退出，代价是 **`Drop` 全不跑、`catch_unwind` 全失效**（实测：一行输出都没有，退出码 `0xc0000409`）。库不要替使用者决定；终产物自己决定。
- 一条链里混用 panic 策略是 UB（Reference 明文），不是兼容性问题。
- 「Rust ABI 不稳定」是可观察的事实：换一个 `-C metadata`，同一个 crate 的符号名就变了。跨语言的缝只能是 C ABI（`extern "C"` + `#[repr(C)]` + `no_mangle`），Rust 之间共享用源码依赖。
- 交叉编译 = 目标支持 + 链接器 + 标准库三件事；**库不需要链接器，可执行文件才需要**。目标默认的 panic 策略决定「不带 std 的库能不能这么编」——不合用时用 `--config 'profile.*.panic="abort"'` 或 build-std。
- build-std 解决「预编译 std 不合用」，代价是 nightly +每次都要带 `-Z`。
- 依赖来源可配置：`[registries]` 换索引、`publish` 白名单防误发、`cargo vendor` 换成本地目录做离线可复现构建。

## 练习

**1.** 下面这段代码在 `panic = "unwind"` 与 `panic = "abort"` 下各打印几行到标准输出？退出码分别是多少？

```rust,no_run
struct G(&'static str);
impl Drop for G {
    fn drop(&mut self) { println!("drop {}", self.0); }
}

fn main() {
    let r = std::panic::catch_unwind(|| {
        let _g = G("main");
        panic!("boom");
    });
    println!("caught: {}", r.is_err());
}
```

**2.** 一个服务把任务跑在 worker 池里，靠 `catch_unwind` 把单个任务的 panic 关在池内。上线前为了「更稳」，发布档位加了 `panic = "abort"`。结果一个任务出错就整个进程没了。为什么？两种改法各是什么？

**3.** 有人把团队内部的 Rust 库编成 `.dylib` 发给同事，说这样「升级不用重新编」。为什么这不是 Rust 的库分发方式？如果同事必须拿到二进制，该怎么设计边界？

<details><summary>参考答案</summary>

**1.** 分档位看：

- **unwind**：stdout 两行——`drop main` 与 `caught: true`；stderr 另有默认 panic hook 的三行（`thread 'main' ... panicked at ...`、panic 消息、`note: run with RUST_BACKTRACE=1 ...`）；退出码 **0**，因为 panic 被 `catch_unwind` 接住、`main` 正常返回。
- **abort**：stdout **一行都没有**——`Drop` 不跑，`catch_unwind` 也没机会返回；stderr 那三行照旧；进程以 `0xc0000409` 中止（Linux 上是 `SIGABRT`）。

要点是：`abort` 下 `Drop` 与 `catch_unwind` **同时**失效，不是只失效一个。

**2.** 因为 `catch_unwind` 文档写明「This function only catches unwinding panics, not those that abort the process.」——`panic = "abort"` 让 panic 直接终止进程，`catch_unwind` 在池内根本没有机会收到返回值。两种改法：（a）把发布档位的 panic 策略改回 `unwind`，保留进程内隔离；（b）保留 `abort`，但把隔离层级从「任务」提到「进程」——用多进程或 supervisor 重启来做崩溃隔离，此时单次崩溃的代价是整进程重启。附带一条：即便在 unwind 下，`catch_unwind` 也只能处理 Rust panic，跨 FFI 的部分还是要按第 26 章的三条防线处理。

**3.** 因为 `.dylib` 是「产物层」的东西：符号名带哈希、ABI 与 panic 运行时都跟编译器版本和参数绑定，换一次编译器或换一个 profile 就可能对不上（Reference 的规矩是：用另一个 Rust 运行时实例编出来的 Rust 代码，在这一节的意义上算「外部代码」）。同事链接它，等于要求所有人用同一轮构建的同一套设置——这不是承诺，是巧合。正确做法分两种：**能收源码就收源码**（crate 依赖，semver 管兼容）；**必须给二进制就把边界收窄到 C ABI**——`cdylib` + `extern "C"` + `#[repr(C)]` + 不透明句柄，像 `snippets/ch40/ffi-lab/` 那样，并把稳定性责任明确写在这个 C 接口上（版本号、结构体字段的增删规则）。

</details>

## 延伸阅读

- Rust Reference 的 Panic 与 ABI 相关条款（panic 运行时与 ABI 的对照表、`#![no_std]` 的 panic 处理器）：<https://doc.rust-lang.org/reference/panic.html>
- Rust Reference 的 Linkage（crate 类型、混链与展开的禁止规则、静态/动态 C 运行时）：<https://doc.rust-lang.org/reference/linkage.html>
- Rust Reference 的 Application binary interface（影响产物 ABI 的属性）：<https://doc.rust-lang.org/reference/abi.html>
- The rustc book：Exploit Mitigations（默认开启的缓解措施清单与检查方法）：<https://doc.rust-lang.org/rustc/exploit-mitigations.html>
- The rustc book：Codegen Options（`symbol-mangling-version`、`metadata`、`panic` 等）：<https://doc.rust-lang.org/rustc/codegen-options/index.html>
- The Cargo Book：Registries（`[registries]`、发布白名单、索引协议）：<https://doc.rust-lang.org/cargo/reference/registries.html>
- The Cargo Book：Source Replacement 与 `cargo vendor`（源替换与 vendor 的方向）：<https://doc.rust-lang.org/cargo/reference/source-replacement.html>
- The Cargo Book：Unstable 一节里的 `build-std` / `build-std-features`：<https://doc.rust-lang.org/cargo/reference/unstable.html#build-std>
- The Rustonomicon：FFI（`no_mangle`、`cdylib`、panic 越界与 `catch_unwind`）：<https://doc.rust-lang.org/nomicon/ffi.html>
- RustSec Advisory Database（依赖漏洞数据的官方来源）：<https://github.com/RustSec/advisory-db>
