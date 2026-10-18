# 第 41 章 领域地图：从这儿往哪走

## 这一章解决什么问题

前四十章把语言机制、类型系统、并发与异步、系统层和工程实践都走了一遍。到这里，拦住你的通常不再是「这个语法怎么写」，而是「我下一个要做的项目属于哪个领域，该从哪份文档的第一页开始」。

这一章是一张地图，不是教程。每个领域只给三样东西：

1. **官方一手入口**：这个领域权威的那本书或那份参考的首页。
2. **先读本书哪几章**：这个领域会逼你用到的那些机制，它们各在哪一章。
3. **最小起步**：能敲的第一条命令，或一个能编过的最小示例。

读完这一章，你应该能对任何新领域回答三个问题：我还缺哪几章的知识、去哪个官方站点、第一条命令敲什么。章末还有一份清单：把手上一个真实 crate 发到 crates.io 之前，值得先过一遍的门禁。

## 为什么这样设计

**地基与生态分层。** 领域里的东西（某个 GUI 框架的 API、某个运行时的调度策略、某个数据库驱动的 trait 形状）以「月」为单位变，写进书里会迅速过期；而语言与工具链的规则（所有权、`Send`/`Sync`、`repr`、Cargo 的解析语义）以「年」为单位变，可以放心写死。本书只负责地基，生态的入口交还给官方文档——这也是领域内容被放在补充章节、而不铺进正文的原因：领域是地图上的路标，不是我们要重写一遍的世界。

**为什么只认官方一手来源。** 二手中文材料的问题不在质量，而在三条：它不告诉你版本、你没法核对它说的对不对、它过期之后没人负责。本书「延伸阅读」一路只收官方来源（The Book、Reference、Nomicon、rustc/Cargo/rustdoc 手册，以及各项目自己的 book），这一章延续同一条标准。

**怎么用这张地图。** 三步：先读下面属于你领域的那一小节，照着「先读哪几章」回去补机制，再打开官方入口从它的第一章动手。它是索引，不是课程表——不要指望它替代官方文档的第一章。

## 机制与规则

### 41.1 嵌入式与 no_std：先把标准库摘掉

`no_std` 不是「少用一个库」，而是换一套运行时契约。官方教材把 `std` 提供、而裸机上没有的东西列得很清楚：

- **堆**：`Vec`、`String`、`Box` 这些依赖动态分配的类型；
- **运行前的初始化**：栈溢出保护、命令行参数、主线程启动——这些发生在你的 `main` 之前；
- **平台集成**：文件、线程、时间、网络。

裸机环境里没有操作系统替你提供任何一条。摘下 `std` 后只剩 `core`（无分配、无 OS，但原子操作、切片、浮点这些语言原语都在），想要堆再加 `alloc`（有容器，分配器由你提供）。编译目标写成**目标三元组（target triple）**，形如 `thumbv7m-none-eabi`：架构 `thumbv7m`（Cortex-M3）、厂商 `none`、系统 `eabi`。本机装了 `thumbv7m-none-eabi` 与 `thumbv7em-none-eabihf`，本章的交叉编译就在它们上真跑。

**交叉编译（cross-compilation）**是在一种平台上产出另一种平台的代码；`--target` 指定的是产物平台，宿主机不变。第一步是给工具链补上目标：

```console
$ rustup target add thumbv7m-none-eabi
```

库 crate 只要不碰 `std` 就能直接编过去，**不需要** panic handler——rlib 只是中间产物。真正需要的是**最终产物**（bin、`cdylib`/`staticlib`）：panic 总得有个落地点，而 `core` 不提供。官方教材的说法是，这个函数必须在其依赖图里恰好出现一次，签名是 `fn(&PanicInfo) -> !`。

`alloc` 是另一条线：它不在 prelude 里，必须显式 `extern crate alloc;`（本机实测，不写会报 E0433，`help` 一行直接写着 add `extern crate alloc`）；用了它，最终链接时还需要 `#[global_allocator]`。另外 `HashMap`/`HashSet` 在 `no_std` 下拿不到——标准库版本依赖随机种子，官方对照表直接把它们列为不可用；替代是 `alloc` 的 `BTreeMap`（已排序、不需要哈希），或引入 `hashbrown` 自己配哈希器。

这个领域最常见的取舍之一是把浮点换成整数运算：没有硬件 FPU 的核上，一次 `f32` 除法会拖进整套软浮点例程。下面这段能直接跑（本章唯一的可跑片段，同样的写法也在 `snippets/ch41` 里）：

```rust
/// ADC 原始读数 → 毫伏：`raw * v_ref_mv / (2^bits - 1)`，全程整数。
fn millivolts(raw: u16, v_ref_mv: u16, bits: u8) -> u16 {
    let full_scale = if bits == 0 { 1 } else { (1u32 << bits) - 1 };
    ((raw as u32 * v_ref_mv as u32) / full_scale) as u16
}

fn main() {
    assert_eq!(millivolts(0, 3300, 12), 0);
    assert_eq!(millivolts(2048, 3300, 12), 1650);
    assert_eq!(millivolts(4095, 3300, 12), 3300);
}
```

> [!WARNING]
> 本章不涉及 nightly-only 内容。只提醒两处仍然只在 nightly 上做的事：给上游没有预编译 `core` 的目标编标准库要用 `-Z build-std`（Cargo 手册的 unstable 章节），以及覆写分配失败的默认行为要用 `#![feature(alloc_error_handler)]`（unstable book 仍有这一页，跟踪 issue #51540）。stable 1.98 上不要照抄。

**先读哪几章。** 第 23 章（unsafe 与 UB 的边界）、第 24 章（指针与来历：裸机上你会大量用 `MaybeUninit` 与裸借用运算符）、第 25 章（`repr` 与布局：寄存器映射的结构体怎么写）、第 26 章（FFI/ABI：和 C 世界对接口）、第 15 章（const 泛型：定长缓冲这类形状）、第 30 章（features 与依赖解析）。按目标写条件依赖的写法见 [Cargo 手册的平台特定依赖](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#platform-specific-dependencies)一节。项目 4 是第 23–26 章那套系统层机制的合成演练。

**官方入口。**

- [The Embedded Rust Book](https://docs.rust-embedded.org/book/)——嵌入式工作组维护的主线教材：`no_std` 环境、内存映射寄存器、中断、`embedded-hal`。
- [Discovery Book](https://docs.rust-embedded.org/discovery/microbit/)——手上有一块 micro:bit 就从这本开始，边做边学。
- [The Embedonomicon](https://docs.rust-embedded.org/embedonomicon/)——讲清 `#![no_main]`、链接脚本、`#[panic_handler]`、启动代码是怎么拼起来的，也就是「`std` 曾经替你做的事」。
- [Rust 平台支持清单](https://doc.rust-lang.org/rustc/platform-support.html)与 [rustup 交叉编译指南](https://rust-lang.github.io/rustup/cross-compilation.html)——判断某个目标是否有人维护、怎么装。
- 生态里的基础 crate：官方 [`cortex-m`](https://docs.rs/cortex-m/latest/cortex_m/)、[`embedded-hal`](https://docs.rs/embedded-hal/latest/embedded_hal/)（跨芯片抽象）、[`heapless`](https://docs.rs/heapless/latest/heapless/)（不用堆的容器）、[`spin`](https://docs.rs/spin/latest/spin/)、[`defmt`](https://docs.rs/defmt/latest/defmt/)（日志）、[`panic-halt`](https://docs.rs/panic-halt/latest/panic_halt/)（最短的 panic handler）。清单见 [awesome-embedded-rust](https://github.com/rust-embedded/awesome-embedded-rust)。

### 41.2 WASM：编到浏览器与 WASI（本节未在本机跑）

两条路线，目标是分开的：

- **`wasm32-unknown-unknown`**：官方目标页的说法是它**不为标准库向宿主导入任何函数**，所以是对宿主环境假设最少的一个目标，常用于编译到 Web/JavaScript。`std` 有支持，但很多部分不工作、只返回错误：同一页点名的例子是 `println!` 什么也不做、`std::fs` 永远返回错误。
- **`wasm32-wasip1`**：假设宿主提供 WASIp1（WASI preview1）那套「系统调用」，`std` 的文件/网络抽象在宿主里有对应实现，官方并明确它支持与 C/C++ 这类非 Rust 代码互操作。WASIp1 已经冻结，不再新增 API；新的方向是组件模型。

有一条规则要记牢：**两个方向的正确性都落在 ABI 边界上**。`wasm-bindgen` 生成的是胶水代码，不是魔法：导出/导入的函数签名、字符串与结构体怎么过边界，本质还是第 26 章那套问题（谁分配、谁释放、什么布局），只是对面换成了 JS 或宿主。`extern "C"` 与 `repr(C)` 分别在第 26、25 章。

> [!WARNING]
> 本机没有安装任何 `wasm32-*` 目标，下面的命令与产物路径**来自官方文档，没有在本机跑过**。要真跑，先装目标（命令与 41.1 同形），再按 wasm-bindgen 手册生成绑定：
>
> ```console
> $ rustup target add wasm32-unknown-unknown
> $ cargo build --release --target wasm32-unknown-unknown
> ```
>
> 产物会落在 `target/wasm32-unknown-unknown/release/` 下，是一个 `.wasm` 模块——它是给浏览器/JS 加载的，不是可执行文件。

**先读哪几章。** 第 26 章（ABI 与 `extern`）、第 25 章（布局与 `repr(C)`）、第 23 章（边界上的 unsafe 责任）、第 30 章（features 与依赖解析里的目标差异）。`[target]` 表的写法见 [Cargo 手册的平台特定依赖](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#platform-specific-dependencies)一节。

**官方入口。**

- [Rust and WebAssembly](https://rustwasm.github.io/docs/book/)——从「为什么是 Rust + wasm」讲到完整的浏览器小游戏。
- [wasm-bindgen 手册](https://rustwasm.github.io/docs/wasm-bindgen/)——绑定生成器本身：类型映射、`JsValue`、`#[wasm_bindgen]` 的全部用法。
- rustc 的两个目标页：[`wasm32-unknown-unknown`](https://doc.rust-lang.org/rustc/platform-support/wasm32-unknown-unknown.html)、[`wasm32-wasip1`](https://doc.rust-lang.org/rustc/platform-support/wasm32-wasip1.html)——各自的支持程度与限制就写在这里。
- [WASI](https://wasi.dev/)与[组件模型](https://component-model.bytecodealliance.org/)——WASI 的现状与「组件」这条路的走法。
- [Rust 官方对 WebAssembly 的说明](https://rust-lang.org/what/wasm/)——面向决策的总览。

### 41.3 GUI 与桌面

GUI 的难点通常不在画控件，而在**所有权怎么套进 UI 树**：控件要互相引用、回调要改状态，而借用规则不接受「谁都持有谁的可变引用」。主流方案给出的答案是不同的形状：

- **即时模式（immediate mode）**：每帧重建界面，状态放在一个普通结构体里，[egui](https://egui.rs/) 是代表。它绕开了控件树的长期持有问题，代价是每帧重绘。
- **系统 WebView + 前端**：Rust 只做后端与命令，界面交给 HTML/CSS/JS，[Tauri](https://tauri.app/) 是代表。它把「UI 状态」整体挪出了 Rust。
- **声明式/Elm 式**：状态变化产生消息，框架负责更新视图，[Iced](https://iced.rs/) 与 [Slint](https://slint.dev/) 属于这一类。这套形状与第 17 章的 `Rc<RefCell<…>>`、第 21 与 22 章的异步任务与取消天然合得来。

第四条路是原生控件绑定：[gtk-rs](https://gtk-rs.org/)（GTK 的官方 Rust 绑定）。

**先读哪几章。** 第 17 章（`Rc`/`RefCell`/`Arc<Mutex>`：共享可变的三种形状）、第 19 章（`Send`/`Sync`：什么能跨线程）、第 12、13 章（trait 设计与 trait 对象：框架要的抽象形状）、第 14 章（`impl Trait`：闭包回调的签名怎么写）、第 21、22 章（异步任务与取消）。

### 41.4 CLI 与命令行工具

这是最容易出成果的领域：一个 crate，`cargo install --path .` 给自己用，`cargo publish` 给别人用。项目 1（命令行小工具）就是按真实工程写的：参数解析、错误类型、单元与集成测试、退出码。机制上你已经有的东西：第 7 章（错误处理）、第 18 章（错误类型与 API 取舍）、第 10 章（迭代器：文本处理的主力）、第 11 章（测试）、第 30 与 32 章（发布与文档）。

**官方入口。**

- [Command Line Applications in Rust](https://rust-cli.github.io/book/index.html)——参数、着色输出、进度条、打包发布一条线。
- [clap 的 derive 教程](https://docs.rs/clap/latest/clap/_derive/_tutorial/index.html)——参数解析的事实标准。
- [serde 文档](https://docs.rs/serde/latest/serde/)——配置文件与 JSON/TOML 的读写。

### 41.5 服务端与异步网络

服务端的形状是「一个运行时 + 若干协议实现」：`tokio` 提供执行器与 I/O 驱动（第 21 章讲的就是这套分工），HTTP 框架在其上做协议，数据库层再叠一层。选型时先回答三个问题：运行时用哪个（第 21 章末的比较）、并发模型是「每连接一个任务」还是「共享状态 + 锁」（第 19、20 章）、超时与取消怎么表达（第 22 章：future 被析构就是取消）。

第三篇（第 19–22 章）与项目 3（任务队列）覆盖的就是这套地基。第 38 章自己实现极简运行时，是为了让你看清 `Waker` 与 `poll` 之后不再被任何运行时唬住；第 40 章接着讲生产里的事：panic 策略（unwind 还是 abort）、动态库与 ABI 的稳定缝、交叉编译与 build-std、供应链与加固选项。**先读哪几章**：第 19–22 章、第 38 章、第 40 章；项目 3。

**官方入口。**

- [Tokio 官方教程](https://tokio.rs/tokio/tutorial)与 [`tokio` API 文档](https://docs.rs/tokio/latest/tokio/)。
- [axum](https://axum.rs/)（[API 索引](https://docs.rs/axum/latest/axum/)）、[actix-web](https://actix.rs/)——两个主流 HTTP 框架。
- [`sqlx`](https://docs.rs/sqlx/latest/sqlx/)——在编译期校验 SQL 的数据库层。
- [Asynchronous Programming in Rust](https://rust-lang.github.io/async-book/) 与 [`futures`](https://docs.rs/futures/latest/futures/)——机制部分本书第 21、22 章已经讲过，官方这本的价值在 `Stream` 与 `select!` 的完整例子。

### 41.6 游戏

游戏把本书后半段的机制一次用全：实体与组件要共享可变（第 17 章）、帧循环要并行（第 19、20 章）、布局与缓存友好直接决定帧率（第 25、29 章）。主流路线是 **ECS（entity–component system）**，[Bevy](https://bevy.org/learn/) 是纯 Rust 的代表：它的 `Query`/`System` 抽象把借用检查搬进了框架层，系统之间能不能并行调度靠的是 trait 与类型信息（第 12、13、14 章）。想要更薄的一层，[macroquad](https://macroquad.rs/)（2D、样板极少）与 [godot-rust](https://godot-rust.github.io/)（接 Godot 引擎）是两个常见起点。

**先读哪几章。** 第 17、19、20 章（共享状态与并行）、第 25 章（`repr` 与布局：组件就地存放还是装箱、填充与对齐怎么算）、第 29 章（先量再改：游戏是最需要证据的领域）。

### 41.7 数据与科学计算

这一领域和服务端相反：不等 I/O，而是把 CPU 吃满，所以主角是**列式数据与并行迭代**。第 6、10 章（集合与迭代器）是日常工具；[Polars](https://docs.pola.rs/) 用列式内存加惰性查询、[ndarray](https://docs.rs/ndarray/latest/ndarray/) 是 N 维数组、[Rayon](https://docs.rs/rayon/latest/rayon/) 把普通迭代器并行化（与第 19、20 章的 `Send`/`Sync` 直接相关），而列式这一支（Polars、DataFusion 这类）共享的内存格式是 [Arrow 的 Rust 实现](https://arrow.apache.org/rust/)。机器学习侧有 [Burn](https://burn.dev/) 与 [Candle](https://github.com/huggingface/candle)。

**先读哪几章。** 第 6、10 章（数据处理的日常）、第 29 章（剖析：先量再优化）、第 30 章（workspace：数据项目往往要拆成多个 crate）、第 13 章（trait 对象：把算法与数据后端解耦）。

### 41.8 清单：从这本书到你的第一个真实发布

把「写完了」变成「发出去」，要按顺序过一遍门禁：

```text
1. 真问题与最小范围      一个能一句话说清用途的 crate，从 0.1.0 开始
2. 仓库骨架              第 30 章：features、workspace、[lints]、rust-version
3. 测试与文档            `cargo test` 全绿；文档注释 + doctest + intra-doc link（第 32 章）
4. lint 门禁             clippy 与警告的处理策略放在哪（第 32 章）
5. edition 与 MSRV       第 33、30 章：edition 2024、rust-version 写清并当真
6. 语义版本              Cargo 手册的 semver 章节：哪些改动算破坏性
7. 打包演练              `cargo package --list` 看清打包内容，`cargo publish --dry-run` 干跑
8. 元数据与文档站点      description/license/repository；docs.rs 的构建行为
9. 安全                  RustSec 公告库；发布后持续做依赖审计
10. CI 门禁              fmt --check、clippy、test --locked、MSRV 一条线（第 30 章）
```

项目 5 把其中若干项串成了一个完整演练（工程化改造加发布演练），值得对着上面十行逐条比。

这份清单里只有一处是真正的一次性动作，下图把边界摊开：

<svg viewBox="0 0 720 200" width="100%" role="img" aria-label="从写代码到发布的流程：cargo publish 是一条不可逆的边界，左边每一步都能重来，右边只能 yank 或发新版本" xmlns="http://www.w3.org/2000/svg">
<text x="215" y="20" font-size="13" text-anchor="middle" fill="var(--links)">可逆区：错了就改</text>
<text x="585" y="20" font-size="13" text-anchor="middle" fill="var(--warning-border)">不可逆区</text>
<line x1="440" y1="14" x2="440" y2="186" stroke="var(--warning-border)" stroke-width="1.5" stroke-dasharray="6 5"/>
<rect x="10" y="40" width="120" height="66" rx="6" fill="none" stroke="currentColor" stroke-width="1.5"/>
<text x="70" y="64" font-size="13" text-anchor="middle" fill="currentColor">写代码</text>
<text x="70" y="86" font-size="11" text-anchor="middle" font-family="monospace" fill="currentColor">0.1.0</text>
<text x="70" y="126" font-size="11" text-anchor="middle" fill="currentColor">第 30 章骨架</text>
<line x1="130" y1="73" x2="144" y2="73" stroke="currentColor" stroke-width="1.5"/>
<polygon points="144,68 154,73 144,78" fill="currentColor"/>
<rect x="154" y="40" width="120" height="66" rx="6" fill="none" stroke="currentColor" stroke-width="1.5"/>
<text x="214" y="64" font-size="13" text-anchor="middle" fill="currentColor">本地门禁</text>
<text x="214" y="86" font-size="11" text-anchor="middle" font-family="monospace" fill="currentColor">fmt clippy test</text>
<text x="214" y="126" font-size="11" text-anchor="middle" fill="currentColor">第 32 章</text>
<line x1="274" y1="73" x2="288" y2="73" stroke="currentColor" stroke-width="1.5"/>
<polygon points="288,68 298,73 288,78" fill="currentColor"/>
<rect x="298" y="40" width="122" height="66" rx="6" fill="none" stroke="currentColor" stroke-width="1.5"/>
<text x="359" y="64" font-size="13" text-anchor="middle" fill="currentColor">打包演练</text>
<text x="359" y="86" font-size="11" text-anchor="middle" font-family="monospace" fill="currentColor">package --list</text>
<text x="359" y="126" font-size="11" text-anchor="middle" fill="currentColor">publish --dry-run</text>
<line x1="420" y1="73" x2="446" y2="73" stroke="currentColor" stroke-width="1.5"/>
<polygon points="446,68 456,73 446,78" fill="currentColor"/>
<rect x="460" y="40" width="120" height="66" rx="6" fill="none" stroke="currentColor" stroke-width="1.5"/>
<text x="520" y="64" font-size="13" text-anchor="middle" fill="currentColor">发布</text>
<text x="520" y="86" font-size="11" text-anchor="middle" font-family="monospace" fill="currentColor">cargo publish</text>
<text x="520" y="126" font-size="11" text-anchor="middle" fill="currentColor">第 30 章</text>
<line x1="580" y1="73" x2="594" y2="73" stroke="currentColor" stroke-width="1.5"/>
<polygon points="594,68 604,73 594,78" fill="currentColor"/>
<rect x="604" y="40" width="106" height="66" rx="6" fill="none" stroke="currentColor" stroke-width="1.5"/>
<text x="657" y="64" font-size="13" text-anchor="middle" fill="currentColor">之后</text>
<text x="657" y="86" font-size="11" text-anchor="middle" fill="currentColor">yank</text>
<text x="657" y="126" font-size="11" text-anchor="middle" fill="currentColor">或发新版本</text>
<text x="360" y="166" font-size="12" text-anchor="middle" fill="currentColor">跨过虚线之前每一步都能推倒重来；跨过之后只能撤销可见性，不能撤销版本</text>
</svg>

crates.io 上的版本一旦发布就永远不能被覆盖，代码也不能删除；能做的只有 yank——阻止新的依赖解析选到它，已有的锁文件不受影响。所以第 7 条的干跑不是走过场，是把「不可逆」变成「可逆」的唯一机会（第 30 章讲过 yank 的确切语义）。

## 动手看看

本节配一个真的 `no_std` crate：`snippets/ch41/`。它只用 `core`，在宿主上跑测试，在裸机目标上交叉编译——两种校验都得过。（下面终端记录只保留有用行：cargo 自己的 `Compiling` / `Finished`、`Running unittests` 的路径行、`Doc-tests` 段与行尾的 `finished in …` 都按需省略。）

```text
snippets/ch41/
├── Cargo.toml
└── src/
    └── lib.rs
```

源码前半部分（文件后半是同文件的单元测试）：

```rust,ignore
{{#rustdoc_include ../snippets/ch41/src/lib.rs:1:62}}
```

> [!NOTE]
> 这个块标记成 `ignore` 是有意的：它带 `#![no_std]` 这个 crate 级属性，而 `mdbook test` 会把每个 rust 块当宿主上的独立程序编译（本书写作时用 mdBook 0.5.4 实测过）。`#![no_std]` 一旦提升为 crate 属性，rustdoc 就会要求它自带入口与 panic handler，编译必然失败——这正是 41.1 说的那条规则。这个 crate 的真实校验是下面的交叉编译，不是 `mdbook test`。

**第一步：宿主上跑单元测试。**

```console
$ cargo test
running 3 tests
test tests::mean_is_integer ... ok
test tests::millivolts_covers_full_range ... ok
test tests::ring_keeps_last_n ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

**第二步：交叉编译。** 本机装的目标如下（Android 与 Windows 的也在里面，这里只关心两个 Cortex-M）：

```console
$ rustup target list --installed
aarch64-linux-android
armv7-linux-androideabi
i686-linux-android
thumbv7em-none-eabihf
thumbv7m-none-eabi
x86_64-linux-android
x86_64-pc-windows-msvc
```

```console
$ cargo build --target thumbv7m-none-eabi
   Compiling ch41-no-std v0.1.0 (D:\Rust\snippets\ch41)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.11s
```

产物按目标分目录，与宿主的 `target/debug/` 互不覆盖（同目录另有构建中间产物，已略）：

```text
snippets/ch41/target/thumbv7m-none-eabi/
├── debug/libch41_no_std.rlib
└── release/libch41_no_std.rlib
```

换成带硬件浮点的核只是换个 `--target`：

```console
$ cargo build --release --target thumbv7em-none-eabihf
    Finished `release` profile [optimized] target(s) in 0.10s
```

**第三步：接进真实工程。** `.rlib` 只是中间产物，要烧进芯片还得有启动代码与链接脚本。官方教材的做法是用 `cargo-generate` 起一个模板工程（以下两条命令来自官方教材，本机没装 `cargo-generate`，未跑）：

```console
$ cargo install cargo-generate
$ cargo generate --git https://github.com/knurling-rs/app-template
```

模板起来之后，把本节的库加进它的 `[dependencies]` 即可。模板自带的 `src/main.rs` 结构如下（照官方教材排的版；`use panic_halt as _;` 这一行提供的就是 panic handler）：

```rust,ignore
#![no_std]
#![no_main]

use panic_halt as _;
use cortex_m_rt::entry;

#[entry]
fn main() -> ! {
    loop {
        // 你的代码
    }
}
```

真实工程一般在 `.cargo/config.toml` 里把默认目标钉死，这样就不必每次带 `--target`（[Cargo 配置手册的 `build.target`](https://doc.rust-lang.org/cargo/reference/config.html)）：

```toml
[build]
target = "thumbv7m-none-eabi"
```

## 报错现场

三个现场，按读者真实的遇错顺序排列。三段记录都按本机实况粘贴，只省掉 cargo 自己的 `Compiling` 行。

**现场一：目标没装。**

```console
$ cargo build --target thumbv6m-none-eabi
error[E0463]: can't find crate for `core`
  |
  = note: the `thumbv6m-none-eabi` target may not be installed
  = help: consider downloading the target with `rustup target add thumbv6m-none-eabi`

For more information about this error, try `rustc --explain E0463`.
error: could not compile `ch41-no-std` (lib) due to 1 previous error
```

（以 Rust 1.98 为例；本机没装 `thumbv6m-none-eabi`，所以拿它当反例。）

定位路径：E0463 说「找不到 crate」，但 `note` 已经把话说白——这个目标可能没装，`help` 甚至给出确切命令。**这不是代码问题**，别去改源码。修法就是它建议的那条：`rustup target add thumbv6m-none-eabi`。

**现场二：拿 `std` 的 crate 去编裸机目标。**

```rust,ignore
fn main() {
    println!("hello from std");
}
```

```console
$ cargo build --target thumbv7m-none-eabi
error[E0463]: can't find crate for `std`
  |
  = note: the `thumbv7m-none-eabi` target may not support the standard library
  = note: `std` is required by `std_probe` because it does not declare `#![no_std]`

error: cannot resolve a prelude import

error: cannot find macro `println` in this scope
 --> src\main.rs:2:5
  |
2 |     println!("hello from std");
  |     ^^^^^^^

error: `#[panic_handler]` function required, but not found

For more information about this error, try `rustc --explain E0463`.
error: could not compile `std-probe` (bin "std-probe") due to 4 previous errors
```

定位路径：第一条 `note` 就把根因写全了——这个目标不支持标准库，而这个 crate 没声明 `#![no_std]`。后面三条（prelude、`println!`、panic handler）全是「`std` 没了」的连带，不要逐条去修。修法：让这个 crate 变成 `#![no_std]` 并按 41.1 补齐它缺的东西，或者承认它本来就不该编给这个目标。

**现场三：`no_std` 的 bin 忘了 panic handler。**

```rust,ignore
#![no_std]
#![no_main]

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    loop {}
}
```

```console
$ cargo build --target thumbv7m-none-eabi
error: `#[panic_handler]` function required, but not found

error: could not compile `bin-probe` (bin "bin-probe") due to 1 previous error
```

定位路径：这是**产物形态**的问题，不是语法问题——库编成 rlib 不需要 panic handler（本章的 `snippets/ch41` 就是这么编过去的），而 bin 是最终产物，必须有一个。这个函数必须恰好在依赖图里出现一次，所以**库不该自己写**（两个库各写一个，链接时就撞了）。修法是最小实现一个，或者依赖现成的：

```rust,ignore
use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
```

`loop {}` 只是把设备挂住，方便调试时看到现场；真要上线就换成记录信息再复位的实现，或者直接用 `panic-halt`、配合 `defmt` 打印。

## 小结

- **地图不是教程**：这一章每个领域只给官方入口、该先读的章、第一条命令；领域细节去官方文档，本书负责的是不会过期的地基（第 23–26、30 章的那些规则）。
- **`no_std` 是换运行时契约**：`core` 无堆无 OS，`alloc` 要你自己提供 `#[global_allocator]`（且要显式 `extern crate alloc;`），`HashMap` 这类依赖随机种子的容器在其中不可用。
- **panic handler 属于最终产物**：库不需要它，bin/`cdylib`/`staticlib` 必须恰好有一个；同理，`#![no_main]` 之后入口点归你。
- **交叉编译只是多一个 `--target`**：先 `rustup target add` 再 `cargo build --target`，产物按目标分目录；E0463 的两条常见成因是「目标没装」与「crate 没声明 `no_std`」，`note` 会直接告诉你哪一条。
- **WASM 的难点在 ABI 边界**：`unknown-unknown` 里标准库不向宿主导入任何函数（`std` 大面积返回错误），`wasip1` 假设宿主提供 WASIp1；两者都要按第 25、26 章的规则想清楚数据怎么过边界。
- **发布不可逆**：`cargo package --list` 与 `cargo publish --dry-run` 是发布前唯一的「可逆机会」（第 30 章）。

## 练习

**一、解释。** 为什么 `#![no_std]` 的**库**不该写 `#[panic_handler]`，而 bin 必须写？如果一个 bin 还要用 `alloc`（`Vec`、`String`），除了 panic handler 还缺什么、报错长什么样？

**二、预测。** 一个 `#![no_std]` 的库里有这一行：

```rust,ignore
use std::collections::HashMap;
```

它编译时报什么？三种修法各是什么？

**三、判断。** 你刚 `cargo publish` 了一个 crate，随后发现某个公开类型的名字起得很糟，而且 `docs.rs` 上的文档里有一处写错的说明。这两样分别能改吗？如果不能，正确的处理流程是什么？

<details><summary>参考答案</summary>

**一、** panic handler 是**最终产物的运行时**的一部分：panic 发生时由它决定怎么办。官方教材的说法是它「必须在其依赖图里恰好出现一次」——库只是会被链接进别人的产物的代码，自带一个就会和调用方或另一个库撞成重复语言项；编成 rlib 时也用不到它。bin、`cdylib`、`staticlib` 这类最终产物则必须提供。用了 `alloc` 之后还缺 `#[global_allocator]`：`core`/`alloc` 不含分配器实现，必须由程序提供，实测（Rust 1.98，`thumbv7m-none-eabi`）报错是：

```text
error: no global memory allocator found but one is required; link to std or add `#[global_allocator]` to a static item that implements the GlobalAlloc trait
```

要注意的是：库用 `alloc`（比如返回 `Vec`）本身能编过，缺分配器要到**最终链接**才暴露。另外分配失败默认走 panic，想自定义得用 nightly 的 `#![feature(alloc_error_handler)]`（41.1 的 WARNING 里提过）。

**二、** 报错是 E0433，关键是它找不到 `std` 这个 crate（而不是「`HashMap` 不存在」）：

```text
error[E0433]: cannot find module or crate `std` in this scope
 --> src\lib.rs:4:13
  |
4 |     let v = std::vec::Vec::from(samples);
  |             ^^^ use of unresolved module or unlinked crate `std`
```

三种修法按代价排序：（1）不用哈希表——`alloc::collections::BTreeMap` 只要 `alloc`，不需要随机种子；（2）引入 `hashbrown`，自己提供哈希器（它不需要 `std` 的随机状态）；（3）如果这个 crate 其实不打算给裸机用，那就不是 `no_std` 的活儿，把 `#![no_std]` 去掉。另外别忘了解析 `alloc` 时要写 `extern crate alloc;`——少了它报的是同一个 E0433，只是名字变成 `alloc`。

**三、** 两样都不能在原版本上改。crates.io 上的版本一旦发布就**永远不能被覆盖，代码也不能删除**；能做的只有 yank，而 yank 的作用仅仅是「不让新的依赖解析选到它」（已有 `Cargo.lock` 的项目不受影响，代码依然在）。`docs.rs` 的文档是从已发布版本构建的，改不了已发布的那一份——重新发一版才会重新构建。正确流程：在本地改好、跑一遍门禁，`cargo package --list` 确认打包内容，`cargo publish --dry-run` 干跑通过后，发 **0.1.1**（或按 Cargo 手册 semver 章节判断该跳哪一位）；如果在本地发现的是**破坏性改动**，就得考虑 `1.0.0` 这类版本跨度。yank 只用在「这一版有严重问题、不该再被新项目装上」的场合（第 30 章）。

</details>

## 延伸阅读

按领域列出，都是上面正文里给过的官方一手入口，这里集中一遍。

**嵌入式与 no_std**

- [The Embedded Rust Book](https://docs.rust-embedded.org/book/)——`no_std` 环境、内存映射寄存器、中断与 `embedded-hal`。
- [The Embedonomicon](https://docs.rust-embedded.org/embedonomicon/)——启动代码、链接脚本、`#[panic_handler]` 是怎么拼起来的。
- [Discovery Book](https://docs.rust-embedded.org/discovery/microbit/)——有硬件时的动手路线。
- [Rust 平台支持清单](https://doc.rust-lang.org/rustc/platform-support.html)、[rustup 交叉编译指南](https://rust-lang.github.io/rustup/cross-compilation.html)、[`core` 文档](https://doc.rust-lang.org/core/)、[Cargo 配置（`build.target`）](https://doc.rust-lang.org/cargo/reference/config.html)。

**WASM**

- [Rust and WebAssembly](https://rustwasm.github.io/docs/book/)、[wasm-bindgen 手册](https://rustwasm.github.io/docs/wasm-bindgen/)。
- 目标页：[`wasm32-unknown-unknown`](https://doc.rust-lang.org/rustc/platform-support/wasm32-unknown-unknown.html)、[`wasm32-wasip1`](https://doc.rust-lang.org/rustc/platform-support/wasm32-wasip1.html)；[WASI](https://wasi.dev/)、[组件模型](https://component-model.bytecodealliance.org/)。

**GUI / CLI / 服务端 / 游戏 / 数据**

- [egui](https://egui.rs/)、[Tauri](https://tauri.app/)、[Iced](https://iced.rs/)、[Slint](https://slint.dev/)、[gtk-rs](https://gtk-rs.org/)。
- [Command Line Applications in Rust](https://rust-cli.github.io/book/index.html)、[clap derive 教程](https://docs.rs/clap/latest/clap/_derive/_tutorial/index.html)、[serde](https://docs.rs/serde/latest/serde/)。
- [Tokio 教程](https://tokio.rs/tokio/tutorial)、[axum](https://axum.rs/)、[actix-web](https://actix.rs/)、[`sqlx`](https://docs.rs/sqlx/latest/sqlx/)、[async-book](https://rust-lang.github.io/async-book/)。
- [Bevy](https://bevy.org/learn/)、[macroquad](https://macroquad.rs/)、[godot-rust](https://godot-rust.github.io/)。
- [Polars](https://docs.pola.rs/)、[ndarray](https://docs.rs/ndarray/latest/ndarray/)、[Rayon](https://docs.rs/rayon/latest/rayon/)、[Arrow](https://arrow.apache.org/rust/)、[Burn](https://burn.dev/)、[Candle](https://github.com/huggingface/candle)。

**发布**

- [Publishing on crates.io](https://doc.rust-lang.org/cargo/reference/publishing.html)、[SemVer Compatibility](https://doc.rust-lang.org/cargo/reference/semver.html)、[Registry 与替代 registry](https://doc.rust-lang.org/cargo/reference/registries.html)。
- [rustdoc 手册](https://doc.rust-lang.org/rustdoc/what-is-rustdoc.html)、[Clippy 文档](https://doc.rust-lang.org/clippy/)、[docs.rs 的构建说明](https://docs.rs/about)。
- [RustSec 公告库](https://rustsec.org/)、[Cargo 手册的 CI 一节](https://doc.rust-lang.org/cargo/guide/continuous-integration.html)。
