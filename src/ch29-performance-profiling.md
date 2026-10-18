# 第 29 章 性能与剖析

## 这一章解决什么问题

Rust 的性能名声来自两句话：零成本抽象（zero-cost abstraction）和「没有运行时」。这两句描述的是**上限**，不是你手上这份代码的现状。程序慢下来，原因通常很朴素：一份在 dev 档测出来的成绩、一个在循环里反复分配的字符串、一个被整份复制十万次的大结构体。

本章只做一件事：把性能从感觉变成可以查证的账。读完你应当能：

- 说清 `dev` 与 `release` 两档的差别，以及 `opt-level`、`lto`、`codegen-units` 各自换走了什么；
- 用 `cargo build --timings` 看出编译时间花在哪个 crate 上；
- 用 `Instant` 加 `black_box` 写出不会被优化器删掉的测量程序，并知道 `#[bench]` 从 1.88 起在 stable 上已不能用；
- 知道剖析（profiling）工具各占地图的哪一格，并跑通 PGO 与覆盖率（instrumentation-based coverage）这两条「用运行数据回喂编译器」的流程；
- 用分配次数这类确定性指标判断 `Vec` 增长、`clone`、`Arc::clone` 的真实成本。

**划两条边界。** 本章所有耗时都随机器浮动，每个数字都标「本机」，你要自己重复一遍才算数；本书不写「必须小于 X ms」这类断言。并发正确性（数据竞争、`loom`、Miri）在第 31 章；字段填充与胖指针这些字节账在第 25 章；编译器内部（MIR、单态化、`-C` 全表）在第 37 章。

## 为什么这样设计

一个程序的速度可以拆成三层账，本章就按这个顺序走。

**第一层：编出来的机器码是什么。** Cargo 默认把未优化档给开发命令，这不是疏忽，而是取舍：调试信息、溢出检查、更快的增量编译，换的是「改一行、跑一下」这个循环的长度。于是第一条规矩很硬：**任何关于运行速度的结论，都必须在优化档下取得**（本机 Windows x86-64、Rust 1.98，下同：同一份求和程序在 dev 档是 3.44 ns/元素，release 档 0.09）。

**第二层：你测的数字可不可信。** 测量程序本身会成为优化的对象：编译器发现你算了半天、结果没人用，就把整段删掉——release 档下 1000 万次加法的循环，结果不消费时耗时 0 到 100 ns，每一步都喂 `black_box` 时耗时 1.8 ms。此外还有噪声（别的进程、调度、CPU 降频）与档位两个坑，方法学就是对付这三件事的一套规矩。

**第三层：运行时的钱花在哪。** 对多数 Rust 程序，热点就三类：分配（allocation）、拷贝（copy）、算法本身。算法要你自己看；分配与拷贝能用尺子量——**分配次数是确定性数字，与机器快慢无关**，这是最好用的一手证据。

顺带把「零成本抽象」补完（第 9、10 章给过它，这里加上测量条件）：迭代器、泛型、闭包不额外收费，靠的是内联与单态化把它们消解掉；消解没发生（跨 crate 又没开 LTO，或者你在 dev 档），抽象就收费了——实测里两种求和写法在 dev 档都掉到几纳秒，且**迭代器反而更快**（1.91 对 3.44），因为 std 里那段代码是预编译好的优化版本。

## 机制与规则

### 29.1 构建档位：profile 怎么选，每项换走了什么

Cargo 有四个内置档位（profile）：`dev`、`release`、`test`、`bench`。不指定时按命令自动选：

| 命令 | 默认档位 |
|---|---|
| `cargo build`、`cargo run`、`cargo check`、`cargo rustc` | `dev` |
| `cargo test` | `test`（继承 `dev`） |
| `cargo bench` | `bench`（继承 `release`） |
| `cargo install` | `release` |

`--release` 是 `--profile=release` 的简写。想同时保留好几套配置，用 `inherits` 定义自定义档位：

```toml
[profile.release-lto]
inherits = "release"
lto = "thin"
codegen-units = 1
```

```console
$ cargo build --profile release-lto
```

产物进 `target/release-lto/`，与 `target/release/` 互不干扰。

两档的默认值如下（Cargo 手册的 `dev`/`release` 表，本机 1.98 复核）：

| 设置 | `dev` | `release` | 它管什么 |
|---|---|---|---|
| `opt-level` | 0 | 3 | 优化等级 |
| `debug` | true（完整） | false | 调试信息量 |
| `debug-assertions` | true | false | `debug_assert!`、`cfg(debug_assertions)` |
| `overflow-checks` | true | false | 整数溢出是 panic 还是回绕 |
| `incremental` | true | false | 增量编译 |
| `codegen-units` | 256 | 16 | 一个 crate 切成多少代码生成单元 |
| `lto` | false | false | 链接期优化 |
| `panic` | unwind | unwind | panic 后展开还是中止 |
| `strip` | none | none | 是否剥掉符号/调试信息 |

逐项看清楚它们换的是什么：

| 设置 | 它管什么 | 要记住的 |
|---|---|---|
| `opt-level` | `0` 到 `3` 越高压得越狠；`"s"`/`"z"` 优化体积，`"z"` 另关循环向量化 | 3 档不保证快过 2 档，得自己试 |
| `debug` | 调试信息量；`"line-tables-only"` 只留文件名与行号 | 把开发档里依赖的调试信息关掉（`[profile.dev.package."*"] debug = false`）能加快链接、缩小 `target/` |
| `debug-assertions` | `debug_assert!` 是否生效 | dev 开、release 关 |
| `overflow-checks` | 溢出 panic 还是回绕 | 同上；第 3 章那个整数溢出示例的两种行为就来自这里 |
| `lto` | 链接期优化的范围 | `true`/`"fat"` 覆盖整个依赖图，`"thin"` 快得多而效果接近，`false` 只在本 crate 的 codegen unit 之间做，`"off"` 才彻底关——「`lto = false` 等于没有 LTO」是错的 |
| `codegen-units` | 并行度与代码质量 | 默认增量 256、非增量 16；设 1 更慢但代码更好 |
| `panic` | 展开还是中止 | 测试、基准、build script、过程宏会忽略它 |
| `incremental` | 重编速度 | rustc 手册：它会抑制优化，不推荐用于 release |
| `strip` | 是否删符号 | 默认 `"none"`；删了符号，剖析器与回溯只剩地址 |

profile 从哪读：**Cargo 只看 workspace 根那份 `Cargo.toml` 的设置，依赖里写的会被忽略**；而 `.cargo/config.toml` 里的 profile 反过来可以覆盖 manifest。另外 **build script、过程宏及其依赖默认不优化**（`opt-level = 0`、`codegen-units = 256`）——很重的 build script 在 release 构建里跑的仍是未优化版本，这是「明明 `--release` 了、构建还是慢」的常见答案。

> [!TIP]
> 开发期的编译速度，第一手段是少编译：改完先 `cargo check`，要跑才 `cargo build`；依赖的 `debug` 信息按上面关掉。

### 29.2 编译时间花在哪：`cargo build --timings`

运行速度之前先说编译速度，因为它是你每天都要等的那笔账。手册对 `--timings` 的说明是：给出每次编译耗时的信息，并跟踪一段时间内的并发情况。

```console
$ cargo build --timings
   Compiling dep_a v0.1.0 (C:\work\demo\dep_a)
   Compiling dep_b v0.1.0 (C:\work\demo\dep_b)
   Compiling probe v0.1.0 (C:\work\demo)
      Timing report saved to C:\work\demo\target\cargo-timings\cargo-timing-20261018T184604873Z-44896fca9a86c57a.html
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.34s
```

（项目目录以 `C:\work\demo` 为例；这次运行在一个三 crate 的小项目上，所以有三行 `Compiling`。）报告是 HTML，各一份：`target/cargo-timings/cargo-timing.html` 是覆盖写的最新一份，带时间戳的那份留着对比历史运行。

报告里有两张表、两张图：**单元图**里一个「单元」是一次编译器调用，每段代表它的耗时，单元完成时会画出因此解锁的新单元，代码生成阶段与 `build.rs` 各用一种颜色标出；**并发图**的三条线分别是等 CPU 槽位的、等依赖完成的、正在跑的，手册提醒它不反映编译器内部的并发；最后一张表列出每个单元的总耗时、代码生成耗时与启用的 feature。

手册给的排查顺序可以直接当清单：先找慢依赖（能不能关 feature、能不能整个去掉），再找同一 crate 被多版本重复编译，然后考虑拆小过大的 crate，最后看是不是一堆 crate 都堵在同一个 crate 上——那种瓶颈优先优化它一个。

> [!NOTE]
> `--timings` 治的是「编译慢」，不是「跑得慢」。两者常被混为一谈：构建图上的热点和运行时热点多半不是同一批代码。

### 29.3 测量方法学：`Instant` 与 `black_box`

现在回到运行速度。标准库的计时工具是 `std::time::Instant`，它的文档把语义写得很克制：测的是一个单调不减的时钟（除非平台有 bug，后取的时刻不会早于先取的），但它**不保证 steady**——每个 tick 的长度可能不同、可能时间膨胀，只是永远不会倒退；它是**不透明类型**，只能与另一个 `Instant` 比较；系统挂起算不算时间，**由平台和 Rust 版本决定**。

还有一条容易踩的：`duration_since`、`elapsed`、`Sub` 在顺序反过来时会**饱和到 0**（早期版本是 panic），别拿它们当逻辑判断，要检测异常顺序用 `checked_duration_since`。

计时简单，难的是让测出来的东西有意义。四条规矩：**在优化档下测**；**预热，然后多轮取最小值**（第一轮含分配、缺页、分支预测填充的代价；噪声只会让某轮变慢，所以用最小值而非平均值）；**输入与结果都要过 `black_box`**；**一次只改一个变量，两版对照跑**。

`std::hint::black_box` 是这里的关键工具。文档对它的定义是（译自 std 文档）「一个恒等函数，提示编译器对 `black_box` 可能做什么保持最大程度的悲观」；注意它**只对输出有效**：`black_box(5 * 10)` 里的 `5 * 10` 照样会被算成 50，要拦住得写 `black_box(5) * black_box(10)`。它还有一条必须说的限定：程序**不能**依赖 `black_box` 保证正确性，它除了表现为恒等函数之外没有任何承诺，也不能用于密码学或安全目的。它是测量工具，不是安全机制。1.66 起稳定。

动手看看的第一个程序把它用上了，结构可以直接抄：

```rust
{{#rustdoc_include ../snippets/ch29/bench_instant.rs}}
```

那段「结果不消费」的对照说明一件事：不喂 `black_box`，编译器的输出是 0 到 100 ns，喂了就变成 1.8 ms——差了四个数量级，而且慢的那个才是真的。

**`#[bench]` 的事得单独说。** 老教程里的写法是：

```rust,ignore
#![feature(test)]
extern crate test;

#[bench]
fn bench_add(b: &mut test::Bencher) {
    b.iter(|| 1u64 + 1);
}
```

这段在 stable 上编译不过：1.88 的 release notes 写明完全取消 `#[bench]` 的稳定资格（此前它至少已是 deny 级的 future-incompatibility 警告，1.88 起升为硬错误）；`test` crate 也只是 nightly 测试框架的内部件。所以上面那个块是**只能看、不能在 stable 跑**的形态。stable 上的替代只有两条：自己用 `Instant` 写，或者用 `criterion` 这类外部框架。

用 `criterion` 时的骨架长这样（外部依赖，需要自己加进 `Cargo.toml`）：

```toml
[[bench]]
name = "add"
harness = false
```

```rust,ignore
use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;

fn bench_add(c: &mut Criterion) {
    c.bench_function("add", |b| b.iter(|| black_box(1u64) + black_box(1)));
}

criterion_group!(benches, bench_add);
criterion_main!(benches);
```

（`black_box` 就用标准库那个。）

`harness = false` 让 Cargo 别用 libtest 的基准框架，把控制权交给 `criterion_main!` 生成的 `main`。它替你做的事正是上一节那四条规矩的自动化：多次采样、给出统计区间、与上次结果比较、噪声过大时提醒你。它不在标准库里，用法以它自己的文档为准。

> [!WARNING]
> 别在没有剖析数据的情况下写基准。基准测试适合**验证**「这个改动是否真的更快」，不适合**发现**「哪里慢」——后者是剖析器的工作，猜热点十次错九次。

### 29.4 剖析工具版图

剖析按「采样还是插桩」分成两条路线，工具版图就按它划；另外还有一条专门看内存与分配的线。

**采样剖析（sampling）** 按固定频率打断程序、记录调用栈，再按栈聚合；开销小、能在接近真实负载下跑，是找热点的首选。Linux 上的标准答案是 `perf`（`perf record -g ./程序` 采样、`perf report` 看结果），火焰图工具（FlameGraph、`cargo flamegraph` 这类包装）都建立在它之上。采样质量取决于两样东西：**符号**（地址能不能还原成函数名）与**调用栈信息**（没有帧指针就得靠 DWARF 展开，更慢也更脆）。

Rust 侧的准备就三条：release 档默认不 strip（见 29.1，符号还在）；想要行号就把 `debug` 设成 `"line-tables-only"`；要强制留帧指针用 `-C force-frame-pointers=yes`（默认行为取决于目标平台）。另外一条近年变化值得知道：从 1.97 起 v0 符号修饰成为默认，旧一点的 demangle 工具认不出来，火焰图上会直接显示 `_RNv...` 这样的原始名字——那不是你的代码有问题，是工具该升级了。

**Windows 与内存。** Windows 没有 `perf` 那样开箱即用的命令行采样器；系统级的采样设施是 ETW，读它的工具（例如 Windows Performance Analyzer）与 IDE 自带的性能探查器是这条路线上的两个方向——只给方向不给命令，因为具体工具与选项随版本变动，写死在书里很快过期。更省事的组合往往是：先用 `Instant` 分段量出「哪一段慢」，再用分配计数（29.6）看是否堆在分配上，真正的采样留给 Linux 上的 CI。要专门的堆剖析器，Linux 上有 `valgrind --tool=massif`、`heaptrack` 这类工具；没有它们时，29.6 那个自定义分配器记账的做法就是最快的一手数据：几十行代码，不需要外部工具，结果确定。

> [!NOTE]
> 采样剖析器看到的是**这一次运行**的热点，输入换一批热点可能整个搬家，所以结论要写清前提。

### 29.5 PGO 与覆盖率：把运行数据喂回去

这一节两条流程骨架相同：先用插桩版收集数据，再让数据参与编译；都属于「知道怎么跑就行」，要不要用是工程决定。

**PGO（profile-guided optimization，剖析引导优化）** 用真实运行的执行轨迹指导内联、机器码排布与寄存器分配。rustc 手册解释了它为什么选插桩而不靠 `perf` 采样：插桩数据更准。四步流程就是下面这张图：

<svg viewBox="0 0 760 170" width="100%" role="img" aria-label="PGO 的四步流程：用 -Cprofile-generate 编译插桩版，运行典型负载产生 default_*.profraw，用 llvm-profdata merge 合并成 merged.profdata，再用 -Cprofile-use 带上该数据重新编译" xmlns="http://www.w3.org/2000/svg">
<text x="0" y="20" font-size="14" fill="currentColor">PGO 四步：插桩、跑负载、合并、用数据重编（LLVM 的流程，rustc 把它接了出来）</text>
<rect x="4" y="46" width="160" height="58" rx="6" fill="none" stroke="currentColor" stroke-width="1.5" />
<text x="84" y="68" font-size="12" font-family="monospace" fill="currentColor" text-anchor="middle">① -Cprofile-generate</text>
<text x="84" y="90" font-size="12" fill="currentColor" text-anchor="middle">编译出插桩版</text>
<rect x="196" y="46" width="160" height="58" rx="6" fill="none" stroke="currentColor" stroke-width="1.5" />
<text x="276" y="68" font-size="12" fill="currentColor" text-anchor="middle">② 跑典型负载</text>
<text x="276" y="90" font-size="12" font-family="monospace" fill="var(--links)" text-anchor="middle">default_*.profraw</text>
<rect x="388" y="46" width="160" height="58" rx="6" fill="none" stroke="var(--links)" stroke-width="1.5" />
<text x="468" y="68" font-size="12" font-family="monospace" fill="currentColor" text-anchor="middle">③ llvm-profdata merge</text>
<text x="468" y="90" font-size="12" font-family="monospace" fill="var(--links)" text-anchor="middle">merged.profdata</text>
<rect x="580" y="46" width="160" height="58" rx="6" fill="none" stroke="currentColor" stroke-width="1.5" />
<text x="660" y="68" font-size="12" font-family="monospace" fill="currentColor" text-anchor="middle">④ -Cprofile-use</text>
<text x="660" y="90" font-size="12" fill="currentColor" text-anchor="middle">用数据重新编译</text>
<line x1="164" y1="75" x2="192" y2="75" stroke="currentColor" stroke-width="1.5" />
<line x1="184" y1="69" x2="192" y2="75" stroke="currentColor" stroke-width="1.5" />
<line x1="184" y1="81" x2="192" y2="75" stroke="currentColor" stroke-width="1.5" />
<line x1="356" y1="75" x2="384" y2="75" stroke="currentColor" stroke-width="1.5" />
<line x1="376" y1="69" x2="384" y2="75" stroke="currentColor" stroke-width="1.5" />
<line x1="376" y1="81" x2="384" y2="75" stroke="currentColor" stroke-width="1.5" />
<line x1="548" y1="75" x2="576" y2="75" stroke="currentColor" stroke-width="1.5" />
<line x1="568" y1="69" x2="576" y2="75" stroke="currentColor" stroke-width="1.5" />
<line x1="568" y1="81" x2="576" y2="75" stroke="currentColor" stroke-width="1.5" />
<text x="0" y="134" font-size="12" fill="currentColor">两个中间产物别混：.profraw 是每次运行的原始计数，合并成 .profdata 才能喂回编译器。</text>
<text x="0" y="156" font-size="12" fill="currentColor">第 ①④ 步用同一份源码，只是编译标志不同；第 ② 步要跑「有代表性的」输入。</text>
</svg>

本机实跑（Windows，把路径换成绝对路径是手册的建议——Cargo 调用 rustc 时工作目录会变）：

```console
$ RUSTFLAGS="-Cprofile-generate=C:/work/demo/pgo-data" cargo run --release --bin capacity
$ llvm-profdata merge -o pgo-data/merged.profdata pgo-data
$ RUSTFLAGS="-Cprofile-use=C:/work/demo/pgo-data/merged.profdata -Cllvm-args=-pgo-warn-missing-function" cargo build --release
warning: alloc_count.9d96ee3c24c02612-cgu.0: no profile data available for function _RNvXsa_…（节选）
warning: `probe` (bin "alloc_count") generated 16 warnings
warning: `probe` (bin "bench_instant") generated 10 warnings
    Finished `release` profile [optimized] target(s) in 1.67s
```

四步都能跑通，那堆警告正是 `-Cllvm-args=-pgo-warn-missing-function` 的用处：我只跑了 `capacity` 一个负载，另外两个二进制没有画像数据，LLVM 默认不吭声，加了这个参数才逐条告诉你。手册另外两条建议照抄即可：给 Cargo 加 `--target`（免得 `RUSTFLAGS` 传给 build script，让它们也生成一堆 `.profraw`）；开工前清掉上一次的画像目录。`llvm-profdata` 来自 rustup 的 `llvm-tools-preview` 组件（`rustup component add llvm-tools-preview`），它**不进 PATH**，要在工具链的 `lib/rustlib/<目标三元组>/bin/` 里找。值不值得：负载稳定、热点集中、二进制长期服役时有意义，一次性脚本不值得维护两套构建。

**覆盖率** 走同一套插桩机制，目标换成「哪些行被执行过」。`-C instrument-coverage`（稳定选项）编译后，运行时把计数写进按 `%m`/`%p` 等模式命名的 `.profraw`，再用 `llvm-profdata merge -sparse` 合并、`llvm-cov report`/`show` 出报告：

```console
$ RUSTFLAGS="-C instrument-coverage" LLVM_PROFILE_FILE="cov-%m_%p.profraw" cargo run --bin capacity
$ llvm-profdata merge -sparse cov-*.profraw -o capacity.profdata
$ llvm-cov report --instr-profile=capacity.profdata --object target/debug/capacity.exe --ignore-filename-regex='registry|\.rustup'
Filename                                                     Regions    Missed Regions     Cover   Functions  Missed Functions  Executed       Lines      Missed Lines     Cover    Branches   Missed Branches     Cover
D:\Rust\snippets\ch29\capacity.rs                                 77                 0   100.00%           2                 0   100.00%          37                 0   100.00%           0                 0         -
TOTAL                                                             77                 0   100.00%           2                 0   100.00%          37                 0   100.00%           0                 0         -
```

（`llvm-cov` 的输出还有 `---` 分隔行与更宽的列，这里为排版删去并压紧，数字未改。）

这就是本机 stable 1.98 上真实跑出来的结果（探针把要看的源码换成了本章的 `capacity.rs`）。`cargo test` 上的用法是 `RUSTFLAGS="-C instrument-coverage" cargo test --tests`，默认输出文件名从 1.65 起是 `default_%m_%p.profraw`：`%m` 是**插桩二进制的签名**（每个二进制各不相同），`%p` 是进程号，`%h` 才是主机名——所以多个测试二进制各写各的文件，不会互相覆盖。`--ignore-filename-regex` 用来排除依赖源码。

两条提醒：覆盖率统计的是「跑过的行」，不是「测好的代码」，行覆盖高与测试质量高是两件事；另外 rustc 手册写明，`-C instrument-coverage` 虽已稳定，但**它产出的画像数据格式可能变化，可能只与编译器随附的工具兼容**，别把它写成长期管道的硬依赖。

### 29.6 分配与克隆的成本

最后一节把最常见的三类开销量出来。前提先记住：**拷贝一个值要搬多少字节，由 `size_of` 决定**（第 25 章）。`Vec<u64>` 复制一次搬 8 × 元素数个字节，`Arc<Vec<u64>>` 复制一次只动一个指针——这是「clone 贵不贵」的第一层答案。

第二层是 `Vec` 的增长方式：容量满了再 `push`，它会申请一块更大的、把旧的搬过去。本机实测的台阶：

```text
Vec<u8>    size_of=1   容量台阶：8@len1 16@len9 (len=12 capacity=16)
Vec<u16>   size_of=2   容量台阶：4@len1 8@len5 16@len9 (len=12 capacity=16)
Vec<u32>   size_of=4   容量台阶：4@len1 8@len5 16@len9 (len=12 capacity=16)
Vec<u64>   size_of=8   容量台阶：4@len1 8@len5 16@len9 (len=12 capacity=16)
with_capacity(100) 后 push 50 个：len=50 capacity=100
shrink_to_fit 之后：len=50 capacity=50
String 容量台阶（每次 push 1 字节）：8@len1 16@len9 (len=16 capacity=16)
```

读法：第一次分配给的非零容量取决于元素大小（本机：1 字节的元素给 8 个，其余小元素给 4 个），之后翻倍。**这是实现细节，不是接口**——`Vec` 只承诺摊销 O(1) 的增长，没承诺具体台阶；但它解释了「预先知道数量就用 `with_capacity`」为什么有效：它把若干次「申请 + 搬运」压成一次。`shrink_to_fit` 反过来把容量收回长度（示例里 100 收到 50）。

第三层、也是最可靠的一把尺子：**数分配次数**。给程序换一个会记账的全局分配器，就能把「这段代码分配了几次、多少字节」变成确定性的输出：

```text
  9 次分配 /    8176 字节   Vec::new + push 1000 个 u32
  1 次分配 /    4000 字节   Vec::with_capacity(1000) + push
  8 次分配 /    2040 字节   String::new + push 1000 个字符
  2 次分配 /      24 字节   format! 一次
  1 次分配 /    8000 字节   Vec<u64> clone（1000 个元素）
  0 次分配 /       0 字节   Arc<Vec<u64>> clone
  1 次分配 /      12 字节   Box<str> from &str（含拷贝）
```

这张表里最该记住的三行：往里塞 1000 个 `u32`，不预留是 **9 次分配、8176 字节**，`with_capacity(1000)` 是 **1 次、4000 字节**；`clone` 一个 1000 元素的 `Vec<u64>` 是**一次分配加 8000 字节**的搬运；`Arc::clone` 是**零次分配**——只把一个计数加一。

最后补上时间对照（release 档、本机，20 轮取最小值）：clone 一个 16 元素的 `Vec<u64>` 是 21.65 ns/次，`Arc<Vec<u64>>` 的 clone 是 7.22 ns/次，两者都含结果被丢弃时的那次释放。所以「用 `Arc` 共享就不用拷数据」是真的，但它**不免费**：`Arc::clone` 是一次原子读-改-写（第 20 章），跨核竞争时比看上去贵。判断该用哪个，看共享的数据有多大、clone 落在多热的路径上。

## 动手看看

三个程序都在本机跑过（Windows x86-64、Rust 1.98）。**下面的数字只属于这台机器**：再跑一次量级不变、具体数会动——我写下的两轮 `push` 数据，第一次是 2.06/1.16 ns，第二次就是 2.04/1.36 ns，末位百分之十几的漂移是常态。

第一个就是 29.3 那个手写 micro-bench。release 档：

```console
$ cargo run --release --bin bench_instant
每次操作的纳秒数（20 轮取最小值；耗时会随机器浮动）
    0.09 ns/op  索引循环求和
    0.09 ns/op  迭代器求和
    2.06 ns/op  push 无容量预留
    1.16 ns/op  push 有 with_capacity
   21.65 ns/op  Vec<u64> clone（16 个元素）
    7.22 ns/op  Arc<Vec<u64>> clone
10000000 次加法：结果不消费 100 ns；每次结果走 black_box 1810900 ns
```

同一份程序换成 dev 档（不加 `--release`）：

```console
$ cargo run --bin bench_instant
每次操作的纳秒数（20 轮取最小值；耗时会随机器浮动）
    3.44 ns/op  索引循环求和
    1.91 ns/op  迭代器求和
    7.04 ns/op  push 无容量预留
    6.19 ns/op  push 有 with_capacity
   54.62 ns/op  Vec<u64> clone（16 个元素）
   12.89 ns/op  Arc<Vec<u64>> clone
10000000 次加法：结果不消费 33292700 ns；每次结果走 black_box 40870000 ns
```

两组数字放在一起读，能读出三件事。（一）索引循环与迭代器在 release 档同为 0.09 ns/元素，零成本抽象在这里是实测事实。（二）dev 档下两者都慢了二三十倍，且迭代器反而更快——档位的影响盖过了写法，可见「在 dev 档比较写法」没有意义。（三）`push` 的容量预留差距在 release 档是 1.8 倍、dev 档几乎看不出来（7.04 对 6.19），而那个「结果不消费」的循环在 release 档被优化掉（100 ns）、在 dev 档没被优化（33 ms）——被删掉的和被拖慢的都不算测量。

第二个程序数分配次数：它换了一个自定义全局分配器，只做两件事——记账，然后转交系统分配器（输出即 29.6 那张分配表）：

```rust
{{#rustdoc_include ../snippets/ch29/alloc_count.rs}}
```

第三个程序量容量台阶，它只依赖标准库实现，换台机器结果也一样（输出是 29.6 的第一张表）：

```rust
{{#rustdoc_include ../snippets/ch29/capacity.rs}}
```

> [!NOTE]
> 三个程序各对应一层账：`bench_instant` 是测量方法学，`alloc_count` 是确定性指标，`capacity` 是标准库行为。它们都不依赖外部 crate，可以直接搬进你的项目当模板。

## 报错现场

第一个报错来自 `#[bench]`。在 stable 上写老教程里的写法：

```rust,compile_fail
#[bench]
fn bench_add(b: &mut test::Bencher) {
    b.iter(|| 1u64 + 1);
}
```

以 Rust 1.98（stable）为例，`cargo bench` 的关键几行：

```text
error[E0658]: use of unstable library feature `test`: `bench` is a part of custom test frameworks which are unstable
 --> src\lib.rs:1:3
  |
1 | #[bench]
  |   ^^^^^
  |
  = note: see issue #50297 <https://github.com/rust-lang/rust/issues/50297> for more information

error[E0433]: cannot find module or crate `test` in this scope
 --> src\lib.rs:2:22
  |
2 | fn bench_add(b: &mut test::Bencher) {
  |                      ^^^^ use of unresolved module or unlinked crate `test`

error: could not compile `bench-err` (lib test) due to 2 previous errors
```

定位路径分两步。第一层，`#[bench]` 属于「自定义测试框架」这条不稳定通道，stable 上直接 E0658——1.88 起它不再只是警告。第二层，`test::Bencher` 里的 `test` 是 nightly 自带的测试框架内部 crate，stable 上根本不存在，于是又一条 E0433。修法不是加 `#![feature(test)]`（那需要 nightly），而是换工具：要长期维护的基准用 `criterion` 之类的框架，只想知道「一次多少纳秒」就用 29.3 那套 `Instant` + `black_box`，二十行以内。

第二个报错来自 profile 表写错了值：

```toml
[profile.release]
opt-level = "fast"
```

```text
error: must be `0`, `1`, `2`, `3`, `s` or `z`, but found the string: "fast"
 --> Cargo.toml:7:13
  |
7 | opt-level = "fast"
  |             ^^^^^^
```

修法就写在报错里：合法值是 `0`、`1`、`2`、`3`、`"s"`、`"z"`。注意 `"s"` 与 `"z"` 是**带引号的字符串**，`0` 到 `3` 是数字。

顺带把 `lto` 说清，它和 `opt-level` 不是同一类：`lto` 的字符串值 Cargo 基本原样透传给 `-C lto`，所以 `lto = "yes"` 也会构建成功（本机 1.98 实测：透传成 `-C lto=yes`，退出码 0、无警告），rustc 的 `-C lto` 本身就收这类布尔写法。真正写错时（`-C lto=bogus`）报错是这样的：

```text
error: incorrect value `bogus` for codegen option `lto` - either a boolean (`yes`, `no`, `on`, `off`, etc), `thin`, `fat`, or omitted was expected
```

手册给的规范写法仍是 `true`/`"fat"`/`"thin"`/`false`/`"off"`，照手册写不会踩到差异。

## 小结

- 性能有三层账：**档位**（编译出什么）、**方法学**（数字可不可信）、**运行时开销**（分配、拷贝、算法）。任何一层的结论都不能拿到另一层用。
- Cargo 默认给开发命令用未优化的 `dev` 档；`release` 档才开 `opt-level = 3`、关掉溢出检查与增量编译。`lto`、`codegen-units`、`debug`、`strip` 是还能继续调的四个旋钮，每个都在「编译时间/体积」与「运行速度」之间换。profile 只看 workspace 根的 `Cargo.toml`，build script 与过程宏默认**不优化**。
- `--timings` 管编译时间：给出单元图、并发图与逐单元耗时表，排查顺序是慢依赖 → 重复版本 → 拆 crate → 解并行瓶颈。
- 测量要满足四条：优化档、预热加多轮取最小、输入与结果都过 `black_box`、一次只改一个变量；`#[bench]` 从 1.88 起在 stable 上是硬错误，替代品是自己写 `Instant` 计时或引入 `criterion`。
- PGO 与覆盖率共用同一套插桩机制，都是四步：插桩编译 → 跑有代表性的负载 → 把 `.profraw` 合并成 `.profdata` → 带着数据重编（PGO）或出报告（覆盖率）。
- 分配次数是确定性指标：`Vec::with_capacity` 把 9 次分配压成 1 次；`Arc::clone` 零分配但不免费（一次原子读-改-写）。

## 练习

**一、解释。** 同一份读数的两个版本：dev 档下「`push` 无容量预留」是 7.04 ns/操作、「`push` 有 `with_capacity`」是 6.19 ns；release 档下是 2.06 与 1.16。为什么这个差距在 release 档才明显？

**二、找错。** 有人这样测一段解析函数的性能（`parse` 是他自己写的函数，这里省略）：

```rust,ignore
fn main() {
    let input = std::fs::read_to_string("big.txt").unwrap();
    let t = std::time::Instant::now();
    for _ in 0..100 {
        parse(&input);
    }
    println!("{:?}", t.elapsed());
}
```

指出至少两处会让结论不可信的做法。

**三、预测。** 一个项目的 `Cargo.toml` 里写了：

```toml
[profile.release]
lto = false
codegen-units = 1
```

（a）`lto = false` 意味着「不做任何 LTO」吗？（b）把 `codegen-units` 从默认的 16 改成 1，换来的是什么、付出的是什么？

<details><summary>参考答案</summary>

**一、** 因为两版的差别只在「增长时要不要重新分配并搬运」，而 dev 档下这段用户代码的每一步都额外付出未优化代码的代价：`push` 不内联、每步都过边界检查。分配与搬运本身发生在标准库里，而标准库是预编译好的优化代码——**dev 档让「你的代码」慢，却没让「库里的代码」慢**。于是 dev 档测到的是「未优化的循环开销 + 基本不变的分配开销」，两者比例被搅乱，容量预留的收益被压进噪声；release 档把你的循环也优化了，分配与搬运的占比回到真实水平，差距才显出来。结论：比较「哪个写法更快」必须在优化档下做。

**二、** 至少有三处问题：

1. **没有预热、没有多轮取最小**：只跑一遍 100 次，第一轮含着首次缺页、分支预测填充的冷启动代价，而单次结果又是平均值，会被冷启动拖高。
2. **没有消费结果**：`parse(&input)` 的返回值被丢掉；`parse` 若是纯函数，release 档下整段循环会被优化掉，测出来接近 0。返回值得送进 `black_box`。
3. **没有说档位**：默认是 dev 档，测出的数字与用户实际跑的 release 版本无关（见第一题）。

（再补两点也不为过：输入是同一份，循环里的公共子表达式可能被提起；没有对照组，读到一个数字也判断不出快慢。）

**三、** （a）不是。`lto = false` 是「thin local LTO」——在本 crate 内部的 codegen unit 之间仍会做一遍；只有 `lto = "off"` 才彻底关掉，而 `opt-level = 0` 或 `codegen-units = 1` 时连这一步也没有。（b）换来代码质量：单元越少，编译器能看到的上下文越完整，跨单元内联与优化机会越多，通常出更快的代码；付出的是编译时间与并行度——crate 不能再切成很多块并行做代码生成。典型的「用编译时间换运行速度」，换不换取决于你构建的频率与这个二进制是否长期热跑。

</details>

## 延伸阅读

- Cargo 手册的档位参考，本章 29.1 的表格与各选项语义都出自它：[Profiles](https://doc.rust-lang.org/cargo/reference/profiles.html)
- 编译耗时报告（单元图、并发图、排查清单）：[Reporting build timings](https://doc.rust-lang.org/cargo/reference/timings.html)
- 官方给出的整套编译提速手段（`cargo check`、调试信息、链接器、并行前端）：[Optimizing Build Performance](https://doc.rust-lang.org/cargo/guide/build-performance.html)
- 计时与提示两个模块：[std::time::Instant](https://doc.rust-lang.org/std/time/struct.Instant.html)、[std::hint](https://doc.rust-lang.org/std/hint/index.html)（`black_box` 见 [std::hint::black_box](https://doc.rust-lang.org/std/hint/fn.black_box.html)）
- PGO 的完整流程与排错建议：[Profile-guided Optimization](https://doc.rust-lang.org/rustc/profile-guided-optimization.html)
- 覆盖率：流程、`llvm-profdata`/`llvm-cov` 用法与限制：[Instrumentation-based Code Coverage](https://doc.rust-lang.org/rustc/instrument-coverage.html)
- 继续往下挖编译器旋钮（`-C` 全表、`opt-level` 与 LTO 的实现侧说明）：[Codegen Options](https://doc.rust-lang.org/rustc/codegen-options/index.html)
- 基准框架（外部依赖，API 以它自己的文档为准）：[criterion.rs](https://github.com/bheisler/criterion.rs)
