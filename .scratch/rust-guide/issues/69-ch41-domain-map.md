# 69 第 41 章 领域地图：从这儿往哪走

Type: task
Status: claimed

## Question

写全书收尾章：**领域地图**（承接 `research/04` 的定位决定：领域内容进补充章节）。

- **范围**：每个领域一小节（2–4 段 + 官方入口链接 + 「先读本书哪几章」+ 最小示例或最小起步命令）：嵌入式/no_std（**真跑**：`#![no_std]` 库交叉编译 `thumbv7m-none-eabi`——本机已装）、WASM（`wasm32-unknown-unknown` 未装：文档化命令并明说未跑）、GUI（egui/tauri 等入口）、CLI 与服务端（回指 ch10/11 与项目 1）、游戏（bevy 等）、数据/科学计算（polars/ndarray 等）、异步网络（tokio，回指第三篇）；最后收束「从这本书到你的第一个真实发布」清单（回指 ch30/32/33 与项目 5）。
- **要求**：外链必须是官方一手入口（逐一可达性核对）；未跑的一律标注；全书回指只写章号；不承诺生态内部细节。
- **边界**：不教程化（每领域入口级）；机制问题回指对应章。
- **篇幅**：4000–5500 汉字；八节模板（「报错现场」用 no_std 交叉编译的典型报错，如缺 `panic_handler`）。
- **DoD**：§8 —— 独占 `src/ch41-domain-map.md` + `snippets/ch41/`（no_std 示例 crate）+ 本票；禁 `git`/禁 build；自验 `mdbook test -c ch41-domain-map.md` exit 0；探针 `.scratch/tmp-69/`；收尾 `Status: claimed` + `## Comments`。

## Comments

### 结论

成稿落盘：`src/ch41-domain-map.md`（占位 H1 保留，正文按八节模板重写）+ 新建 `snippets/ch41/`（`Cargo.toml` + `src/lib.rs`：`#![no_std]` 库，`RingBuffer<const N>` / `millivolts` + 3 个单测）+ 内嵌 SVG 1 张（41.8 发布可逆/不可逆边界）。八节结构齐全：领域小节 41.1–41.7 + 41.8「第一个真实发布」清单（10 项，回指第 30/32/33 章与项目 5）。

篇幅：**汉字 4818**（口径 = 去掉围栏代码块后的 CJK 字符数，与批次其它票同法）。

### 自验命令与结果

| 命令 | 结果 |
|---|---|
| `mdbook test -c ch41-domain-map.md` | **exit 0**（7 个 rust 块：1 个真跑 doctest、6 个 `ignore`；text/console/toml 块不参与） |
| 注入反证：把可跑片段的 `millivolts(2048,3300,12)` 期望改成 1600 | **exit 101**（`assertion left: 1650 right: 1600`）→ 证明门禁真跑；改回后 exit 0 |
| `cd snippets/ch41 && cargo test` | 3 passed; 0 failed（`ring_keeps_last_n` / `mean_is_integer` / `millivolts_covers_full_range`） |
| `cargo build --target thumbv7m-none-eabi`（debug）+ `--release` | 全绿，产物 `target/thumbv7m-none-eabi/{debug,release}/libch41_no_std.rlib` |
| `cargo build --release --target thumbv7em-none-eabihf` | 全绿 |
| 外链可达性（脚本并发 GET，全章 54 条唯一 URL） | **54/54 → 200，0 bad**（首轮发现的死链已替换：rustup 书 → `rust-lang.github.io/rustup/cross-compilation.html`；wasm-bindgen → `rustwasm.github.io/docs/wasm-bindgen/`；`bevyengine.org/learn` → `bevy.org/learn`；cortex-m-quickstart 文档路径 404 → 改用官方教材现推的 app-template） |
| 结构自查 | 栅栏 44（偶）；全文无 `\``（反斜杠+反引号坑）；SVG 单块 0 空行 / XML 可解析 / 无 `height` / 无写死颜色 / 无 script·外链；方括号文本只在代码块内 |

### 实测输出清单（正文粘贴来源，均真跑）

1. `cargo test` 3 passed（`snippets/ch41`）——动手看看第一步。
2. `cargo build --target thumbv7m-none-eabi` → `Finished dev … in 0.11s`；`--release --target thumbv7em-none-eabihf` → `Finished release … in 0.10s`。
3. `rustup target list --installed` 全量 7 行（aarch64/armv7/i686/x86_64-linux-android、thumbv7m、thumbv7em、x86_64-pc-windows-msvc；**无任何 wasm32**）。
4. 报错现场一（目标没装）：`error[E0463]: can't find crate for core` + `note: the thumbv6m-none-eabi target may not be installed` + `help: consider downloading the target with rustup target add thumbv6m-none-eabi`（探针 = `snippets/ch41` 加本机未装的 `thumbv6m-none-eabi`）。
5. 报错现场二（std crate 编裸机）：E0463 `can't find crate for std` + `note: … may not support the standard library` + ``note: `std` is required by `std_probe` because it does not declare `#![no_std]` `` + prelude/`println!`/panic_handler 三条连带 + footer（探针 `.scratch/tmp-69/std-probe`）。
6. 报错现场三（缺 panic handler）：`error: #[panic_handler] function required, but not found`（探针 `.scratch/tmp-69/bin-probe`，`#![no_std] #![no_main]` bin）。
7. 练习一答案用：`error: no global memory allocator found but one is required; link to std or add #[global_allocator] to a static item that implements the GlobalAlloc trait`（探针 `.scratch/tmp-69/alloc-probe`）。
8. 练习二答案用：`error[E0433]: cannot find module or crate std in this scope`；连带实测 `extern crate alloc;` 缺失时同一 E0433（`help: add extern crate alloc to use the alloc crate`）（探针 `.scratch/tmp-69/stduse-probe`）。
9. doctest 反证输出（见上表第 2 行）。

探针 `.scratch/tmp-69/`（4 个探针 crate + SVG 渲染件）**已删除**；`snippets/ch41/target` 与 `Cargo.lock` 保留（与 ch34/ch38/ch39 同批做法一致，`target/` 已 gitignore）。

### 引文来源核对（逐字取官方一手）

- 嵌入式书 `intro/no-std.html`：`std` 还提供运行时（栈溢出保护、命令行参数、主线程启动）与 no_std 特性对照表；表中 heap 需 `alloc` + 合适分配器、collections 需全局分配器、**HashMap/HashSet 因缺安全随机源不可用**。
- 嵌入式书 start 章：`#[panic_handler]`「必须在其依赖图里恰好出现一次」、签名 `fn(&PanicInfo) -> !`；模板 `use panic_halt as _;` + `use cortex_m_rt::entry;`；生成命令 `cargo install cargo-generate` + `cargo generate --git https://github.com/knurling-rs/app-template`（**官方教材现推 knurling app-template，不是 cortex-m-quickstart**，正文按前者写）。
- rustc 目标页：`wasm32-unknown-unknown`「不为标准库向宿主导入任何函数」、`println!` 什么也不做 / `std::fs` 永远返回错误；`wasm32-wasip1` 假设 WASIp1 syscalls 可用、明确支持与 C/C++ 互操作、syscalls 已冻结不再维护。
- Cargo 手册 `publishing.html`：已发布版本「永远不能被覆盖，代码也不能删除」；yank 的效果是「不能再创建新依赖」。
- unstable book `alloc_error_handler`（跟踪 issue #51540）；Cargo unstable 手册 `build-std`。

### 未跑 / 未实测（正文已逐条标注）

- 41.2 的两条 WASM 命令与 `.wasm` 产物路径：本机无 wasm32 目标，正文用 `> [!WARNING]` 标注「来自官方文档，没有在本机跑过」。
- 41.1 的 `cargo install cargo-generate` / `cargo generate --git …`：本机未装 `cargo-generate`，正文括注「来自官方教材，未跑」。
- nightly-only 两处（`-Z build-std`、`#![feature(alloc_error_handler)]`）：只用 `> [!WARNING]` 点名 + 给文档化位置，**未演示、未伪造输出**。

### 新增术语候选（请裁定并表）

- **交叉编译（cross-compilation）**：41.1 首现给英文，全文用中文。
- **目标三元组（target triple）**：同上（第 29/30 章已用中文词，本章补英文首现）。
- `no_std`、`panic handler`、`alloc`：保留英文（GLOSSARY 目前都没有这三条；若要并表，`panic handler` 建议「panic 处理器（panic handler）」或直接保留英文）。

### 衔接提示

- `src/ch26-ffi-abi.md:15` 承诺「`no_std` 与嵌入式场景的组合在第 41 章」→ 41.1 已兑现（运行时契约、panic_handler、全局分配器、目标三元组）。
- 41.8 回指第 30 章（features/workspace/MSRV/发布与 yank）、第 32 章（文档与 lint）、第 33 章（edition）、项目 5；41.5 回指第 19–22、38、40 章与项目 3；41.4 回指项目 1 与第 7、10、11、18 章。
- 正文只写章号回指，未点名任何现有中文资源；外链全部是官方一手入口（含各项目自己的官方站点/仓库）。

### 图示

1 张内嵌 SVG（41.8，发布可逆/不可逆边界；viewBox 720×200、`width="100%"`、无 `height`、颜色只用 `currentColor`/`var(--links)`/`var(--warning-border)`、`role="img"` + `aria-label`）。已把主题变量替换成具体色后独立栅格化目视：五格流程、四条箭头、虚线边界、两行区域标注与底部说明**均无重叠/裁切**。**待收尾 build 时确认 HTML 主题下 `var(--links)`/`var(--warning-border)` 的实际呈现。**

### 遗留不确定

1. **动手看看的主片段标 `rust,ignore` 是必然选择**：实测把含 `#![no_std]` 的文件当 doctest 编译会失败（`#![no_std]` 被提升为 crate 属性 → `error: #[panic_handler] function required, but not found` + `error[E0601]: main function not found in crate rust_out`）；`{{#rustdoc_include}}` 的行范围会把范围外行以 `#` 隐藏但仍参与编译，躲不开。故该 crate 的门禁 = `cargo test`（宿主）+ `cargo build --target thumbv7m-none-eabi`（目标），正文 NOTE 写明；另配 1 个真跑 doctest（整型换算片段），保证 `mdbook test` 不空跑。
2. 41.2 的产物目录 `target/wasm32-unknown-unknown/release/` 按官方文档写，本机未验证。
3. 41.8 提到项目 5 时 `src/project5-engineering.md` 仍是占位，故只写「把其中若干项串成了一个完整演练（工程化改造加发布演练）」——若项目 5 最终内容与此不符，请回改这一句。
4. 篇幅（修复前 4818 汉字）落在票面区间内偏中；如要求贴近上界可扩写 41.3–41.7，但会增加过期风险，建议保持现稿。

### 审校修复（RvCh41：1 blocker + 2 major + 7 minor，逐条处理）

| # | 位置 | 处理 |
|---|---|---|
| F1 | 41.5「第 40 章…日志与可观测性」 | **blocker，已改**：按 ch40 实况的小节重写为「panic 策略（unwind 还是 abort）、动态库与 ABI 的稳定缝、交叉编译与 build-std、供应链与加固选项」（对照 40.1–40.6）。 |
| F2 | 41.6 第 25 章括注「AoS/SoA」 | **major，已改**：ch25 无该词，改为「`repr` 与布局：组件就地存放还是装箱、填充与对齐怎么算」（ch25 实有内容：对齐/填充、`repr(Rust)`/`repr(C)`/`repr(transparent)`、判别式与 niche、DST 胖指针）。 |
| F3 | 41.7「Arrow 是这些库共享的格式」 | **major，已改**：「列式这一支（Polars、DataFusion 这类）共享的内存格式是 Arrow 的 Rust 实现」——不再把 ndarray/Rayon 说成共享它。 |
| F4 | 小结 `#[no_main]` | **minor，已改** → `#![no_main]`。 |
| F5 | 41.1「裸借用」 | **minor，已改** → 「裸借用运算符」（GLOSSARY 定名）。 |
| F6 | 41.1/41.2「第 30 章（按目标的条件依赖）」 | **minor，已改**：胸注收窄为「第 30 章（features 与依赖解析 / 其中的目标差异）」，`[target]` 表写法另给一处新外链 [Cargo 手册 · Platform specific dependencies](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#platform-specific-dependencies)（新增链，已验 200）。 |
| F7 | 41.8→动手看看的免责声明 | **minor，已改**：放宽为「只保留有用行：`Compiling`/`Finished`、`Running unittests` 的路径行、`Doc-tests` 段与行尾的 `finished in …` 都按需省略」。报错现场那句**保留不改**——它只省了 `Compiling`，与实况相符。 |
| F8 | 41.1「项目 4 是这套东西的演练」 | **minor，已改** → 「项目 4 是第 23–26 章那套系统层机制的合成演练」。 |
| F9 | 41.3 两处「异步命令」 | **minor，已改** → 「异步任务与取消」（正文那一句 + 该节「先读哪几章」各一）。 |
| F10 | 术语并表（交叉编译 / 目标三元组 / panic handler 等） | 归协调者，未动。 |
| — | 票内「保留 `snippets/ch41/target` + `Cargo.lock`」 | 该句**已过时**（协调者按 snippets 惯例清理）；本次**未动**、不补回。 |

**复核**：修后 `mdbook test -c ch41-domain-map.md` **exit 0**；全章 55 条唯一外链重新核过 —— 53 条 200，2 条 `github.com` 本轮超时，但对照实验证明是本机到 github.com 的网络问题（同期 `github.com/rust-lang/rust` 同样超时，而 `api.github.com`、`raw.githubusercontent.com` 均 200），两条链接经 GitHub API 确认仓库存在且未归档（`rust-embedded/awesome-embedded-rust` 200 / `huggingface/candle` 200）；正文 **4904 汉字**（口径同前），栅栏 46（偶）、无 `\``、SVG 仍单块 0 空行。
