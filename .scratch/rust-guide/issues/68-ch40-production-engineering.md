# 68 第 40 章 生产工程

Type: task
Status: claimed

## Question

写第六篇第七章：**生产工程**（素材 = D7：rustc `exploit-mitigations`、cargo `registries`、unstable-book `build-std`）。

- **范围**：panic 策略（`panic = "unwind"` vs `"abort"`：真跑两种 release 构建对照）；动态库与 ABI 稳定（`cdylib` 真跑 + 「Rust ABI 不稳定」的工程含义，回指 ch26 结论）；交叉编译（**本机已装** `thumbv7m-none-eabi`/`thumbv7em-none-eabihf`/android 系 target——真跑 `cargo build --target` 粘输出）；`build-std`（nightly 标注，只文档化）；供应链与 registry（`[registries]` 配置、私有 registry 文档化、`cargo vendor` 方向说明）。
- **要求**：真跑与「文档化 + 标注」严格区分；thumb 目标用 `#![no_std]` 库示例（`snippets/ch40/` 独立 crate）。
- **边界**：FFI/ABI → ch26 回指；profile → ch29；workspace/features → ch30。
- **篇幅**：4000–5500 汉字；八节模板；练习 2–3 题带答案。
- **DoD**：§8 —— 独占 `src/ch40-production-engineering.md` + `snippets/ch40/` + 本票；禁 `git`/禁 build；自验 `mdbook test -c ch40-production-engineering.md` exit 0 + 交叉编译真跑；探针 `.scratch/tmp-68/`；收尾 `Status: claimed` + `## Comments`。

## Comments

**做了什么 / 文件**

- 改写 `src/ch40-production-engineering.md`（占位 H1 保留，八节模板齐全；正文约 5 320 汉字，连同被 `#include` 进来的片段中文注释合计 5 851）。结构：40.1 panic 策略 / 40.2 动态库与 ABI / 40.3 交叉编译 / 40.4 build-std / 40.5 供应链与 registry / 40.6 加固速览（D7 的 exploit-mitigations 素材）+ 动手看看 / 报错现场（3 个真实报错）/ 小结 / 练习 3 题带答案 / 延伸阅读（9 条官方一手）。
- 新增 `snippets/ch40/panic-lab/`（`Cargo.toml` 两个 profile + `src/main.rs`）：同一份源码在 unwind / abort 两档下真跑对照。
- 新增 `snippets/ch40/ffi-lab/`（`Cargo.toml` 的 `crate-type = ["cdylib","rlib"]` + `src/lib.rs` + `host.c`）：编出 `pointops.dll`，由 MinGW gcc 编译的 C 宿主真调通。
- 新增 `snippets/ch40/fwlib/`（`Cargo.toml` staticlib + `src/lib.rs`）：`#![cfg_attr(not(test), no_std)]` 的环形缓冲 + CRC-32，用于交叉编译。
- 两处内嵌 SVG：40.2「同一份代码的两套名字体系」、40.3「目标三元组四段拆解」。
- 未碰 `src/SUMMARY.md`、`book.toml`、`WRITING.md`、`GLOSSARY.md`、`README.md`、地图与其它票。探针目录 `.scratch/tmp-68/` 已删除；`snippets/ch40/*/target/`、`ffi-lab/host.exe`、三个 `Cargo.lock` 也已清理（只留 7 个源文件）。

**自验命令与结果（§8：只跑 `mdbook test -c`，未跑 build、未跑 git）**

- `mdbook test -c ch40-production-engineering.md` → **exit 0**（5 passed / 2 ignored）。两个 ignored 是：fwlib 的 `rust,ignore` include（`#![no_std]` 在 doctest 里编不过，改由 `cargo build --target` / `cargo test` 把关）与 `#[panic_handler]` 片段（与 std 的 lang item 冲突）。
- 反向验证门禁确实生效：临时注入一个类型错误的 `rust` 块后 `mdbook test -c` → exit 101、`5 passed; 1 failed; 2 ignored`；随后已还原（当前 exit 0）。
- `cargo test`（`snippets/ch40/fwlib`，宿主机）→ `2 passed`。
- 交叉编译真跑（本机 stable 1.98 / x86_64-pc-windows-msvc）：
  - `cargo build --target thumbv7m-none-eabi` → `libfwlib.a`（7 998 358 B），`llvm-readobj` 报 `Format: elf32-littlearm`；
  - `cargo build --target thumbv7em-none-eabihf` → `libfwlib.a`（8 098 996 B），同为 `elf32-littlearm`；
  - `cargo build --target x86_64-linux-android` → 失败：`error: unwinding panics are not supported without std`（附 `= help: using nightly cargo, use -Zbuild-std with panic="abort"`）；
  - `cargo build --target x86_64-linux-android --config 'profile.dev.panic="abort"'` → 成功，`Format: elf64-x86-64`；
  - std 可执行文件交叉编译的两个真实报错：`thumbv7m-none-eabi` → `error[E0463]: can't find crate for 'std'`；`x86_64-linux-android` → `error: linker 'cc' not found`。

**实测输出清单（正文里的 console/text 块全部真跑真粘，除下述「文档化」项）**

1. panic 对照：unwind+`catch_unwind`（stdout 3 行 `drop` + `catch_unwind 拦住了：true`，exit 0）；unwind 不捕获（同 3 行，exit 101）；abort 两例（stdout 一行都没有，exit `0xc0000409`，cargo 打 `STATUS_STACK_BUFFER_OVERRUN`）。二进制体积 `release` 140 288 B vs `release-abort` 129 024 B。
2. cdylib：`cargo build --release` 产 `pointops.dll` / `pointops.dll.lib` / `libpointops.rlib`（附 `linker_messages` 警告的真实文本）；`llvm-readobj --coff-exports … | findstr Name` 恰好 4 个导出名；`gcc host.c -o host.exe target\release\pointops.dll` 链上，`.\host.exe` 输出 `add(3)=3 / add(4)=7 / sum=10, count=4`。
3. ABI 不稳定：`rustc --crate-type rlib -C metadata=alpha|beta` 编同一份 4 行 `counter.rs` → `_RNvMCsdi1eJHOR88G_7counterNtB2_7Counter4hits` 与 `_RNvMCsa8Xayzzdo2I_7counterNtB2_7Counter4hits`；`String` 跨 C 边界的 `improper_ctypes_definitions` warning 全文（1.98 上是 warning 而非 error，所以正文没用 `compile_fail`）。
4. 目标信息：`rustc --print cfg --target thumbv7m-none-eabi|thumbv7em-none-eabihf|x86_64-linux-android` 的字段对照（两个 thumb 都是 `panic="abort"`、`target_arch="arm"`、`target_vendor="unknown"`，差别在 `target_abi` 的 eabi/eabihf；android 是 `panic="unwind"`）；`rustc --print target-list` 共 330 个目标。
5. build-std：stable 上 `cargo build -Z build-std --target thumbv7m-none-eabi` → `error: the '-Z' flag is only accepted on the nightly channel of Cargo, but this is the 'stable' channel`。
6. 供应链：`cargo publish --registry=my-registry --dry-run` → `error: registry index was not found in any configuration: 'my-registry'`；`publish = false` 的包 `cargo publish --dry-run` → `error: 'pubprobe' cannot be published.` + 那句 `package.publish` 说明；`cargo vendor`（`CARGO_NET_OFFLINE=true`，依赖用缓存里的 `cfg-if 1.0.5`）的真实输出，以及按它给的 `replace-with` 配好后 `cargo build --offline` 成功。
7. 加固：`llvm-readobj --file-headers panic-lab.exe | findstr IMAGE_DLL_CHARACTERISTICS` → DYNAMIC_BASE / HIGH_ENTROPY_VA / NX_COMPAT / TERMINAL_SERVER_AWARE。
8. Reference 的四行表与「未指定哪种析构行为」那条，出处 `reference/items/functions.html` 的 `[items.fn.extern.unwind]`。

**引文来源（逐字核对，均取本机 1.98 官方快照或随附源码）**

- Rust Reference：`linkage.html`（“Rust code compiled or linked with a different instance of the Rust runtime counts as “foreign code”…”、“If a Rust artifact is potentially unwinding…” 两条原文）；`items/functions.html`（`[items.fn.extern.unwind]` 表 + abort 说明）；`panic.html`（`fn(&PanicInfo) -> !` 签名）；`abi.html`（进延伸阅读）。
- rustc book：`exploit-mitigations.html`（默认/非默认列与 nightly 项）；`codegen-options/index.html`（`symbol-mangling-version` 的 “a compiler-chosen default which may change in the future”）。
- The Cargo Book：`reference/registries.html`（`[registries]` 示例、`registry = …` 依赖键、`publish`、git/sparse 协议）；`reference/unstable.html#build-std`（三条硬要求、默认编 core/std/alloc/proc_macro、`build-std-features` 默认 `backtrace` + `panic-unwind`）；`reference/config.html`（`[target.<triple>] linker / runner / rustflags`）。
- Nomicon `ffi.html`（`no_mangle` 那句原文）；std 源码 `sys/pal/windows/mod.rs` 中 `abort_internal` 的注释 “Use `__fastfail` to abort the process”。
- 参照本机 std 目录确认 `thumbv7m-none-eabi` 只有 `libcore`/`liballoc`/`libcompiler_builtins`（无 `libstd`），作为「现场二」的定位依据。

**遗留不确定（需要协调者知悉）**

- 40.3 的 `.cargo/config.toml` 里 `[target.x86_64-linux-android] linker = …` 只做**文档化**（本机没装 NDK，正文已明说未实测）；能实测的是「同一目标编库能过、编可执行文件缺链接器」这一对比。
- 40.4 的 build-std 全部命令行**未实测**（本机仅 stable），正文用 `> [!WARNING]` 标注为 nightly-only；实测到的只有 stable 拒绝 `-Z` 的那句。
- 40.6 三条 nightly 加固开关（`-Z stack-protector`、`-Z sanitizer=cfi`、`-Z sanitizer=shadow-call-stack,safestack`）未实测，只按 rustc book 表格列出；Windows 侧默认开的两条是实测。
- 正文中 console 块里的线程号、时间戳、cargo 的耗时数字每次运行都不同，已在相应位置说明。
- **未跑 `mdbook build`**：请收尾时确认 0 警告、linkcheck2 无断链（重点关注两处内嵌 SVG 与 `[items.fn.extern.unwind]` 这类方括号文本的渲染），并目视确认两张 SVG 在明暗两套主题下的观感。
- 篇幅：全文 CJK 5 851，其中 529 来自被 include 的三个片段的中文注释，章节自撰正文约 5 322 汉字（票要求 4000–5500）。若按「含注释总数」卡 5 500，需要再砍约 350 字，我倾向保留（内容密度已较高、无注水段）。

**新增术语**

无全新术语。若要并表，建议补两条：**目标三元组（target triple）**、**源替换（source replacement）**（其余沿用：展开 unwind、中止 abort、供应链、registry、vendoring）。`GLOSSARY.md` 我未改动。

**衔接提示**

- 40.1 正是第 7 章那句「选择理由属于工程策略（第 40 章展开）」的落点；同时回指第 18 章（库里别 panic）、第 26 章 26.6（三条防线）、第 29 章 29.1（profile 表）。
- 40.2 回指第 25 章（布局）、第 26 章（FFI 三件套）；40.3 与 40.4 互为铺垫（40.3 的报错直接把读者引到 40.4）；40.5 与第 30 章的发布/yank 一节互补（第 30 章讲「怎么发」，本章讲「从哪取、怎么离线」）。
- 第 41 章若要提「产物层没有稳定性承诺」，落点在本章 40.2 开头那三层表。

**RvCh40 审校修复（第 2 轮，逐条）**

- **F5（major，现场二）· 接受并重贴**：用 `cargo new hellobin` 的默认源码（`println!("Hello, world!");`）重跑 `cargo build --target thumbv7m-none-eabi`，实况是**四条**错误——除 E0463、prelude、`println` 之外还有 `error: #[panic_handler] function required, but not found`，末尾为 `For more information about this error, try 'rustc --explain E0463'.` 与 `error: could not compile 'hellobin' (bin "hellobin") due to 4 previous errors`。根因：第一轮探测时我把输出 `head -12` 截断后当全文粘贴，漏了后两条与收尾行。已按实况整块重贴（含 `|     ^^^^^^^` 插入符行），叙述改为「四条错误」，并补上因果「连 panic 处理器也没了（它平时由 `std` 提供，见现场一）」。
- **F1（minor）· 接受**：三处私有探针路径全部截断——vendor 块改为 `(...\vendprobe2)`，现场二/现场三改为 `(...\hellobin)`；`grep '\.scratch' src/ch40-production-engineering.md` 现为 0 命中。
- **F2（minor）· 接受**：NOTE 的四行表补齐第四行（`panic=abort` + 非展开 ABI → 不展开直接中止），现为四行齐全。
- **F3（minor）· 接受**：改为「第 29 章 29.1 表格里 `panic` 那一行」。
- **F4（minor）· 接受**：改按章内那份**四行**源码（`#[unsafe(no_mangle)]` + 三行函数体）重跑，粘出**两条** `improper_ctypes_definitions`（`2:26` 参数、`2:37` 返回值）与结尾 `warning: 2 warnings emitted`，源行不再是单行版的 `{ s }`；紧随叙述改为「这两条都只是 warning（来往两个方向各一条）」。
- **F6（minor）· 接受**：四处 console 块的守尾行补齐——40.3 的 fwlib android 块加 `error: could not compile 'fwlib' (lib) due to 1 previous error`；40.3 的 android bin 块补 `Compiling …` 与 `error: could not compile 'hellobin' (bin "hellobin") due to 1 previous error`；现场二、现场三同上。另按同一标准复查全文其余报错块（现场一、build-std 的 `-Z`、registry 未配置、`publish = false`、panic 三例、FFI 警告）：均在实况下已含收尾行或本就无收尾行。
- **可选项 · 采纳**：按 ch16/22/23 的体例，在「这一章解决什么问题」末尾补了 `> [!NOTE]` 的实测环境 + 节选约定声明（本机 `rustc 1.98.0 (88d9e12ae 2026-08-18)`、edition 2024、x86_64 Windows、只装 stable；报错按「最小复现 + 关键行节选」；线程号与耗时随运行变化）。

**修复后自验**：`mdbook test -c ch40-production-engineering.md` → exit 0（5 passed / 2 ignored）。探针 `.scratch/tmp-68/`、`snippets/ch40/*/target`、`Cargo.lock` 已再次清理（`snippets/ch40/` 只留 7 个源文件）。篇幅复核：全文 CJK 5 954，其中 529 为 include 片段的中文注释，章节正文约 5 425 汉字。
