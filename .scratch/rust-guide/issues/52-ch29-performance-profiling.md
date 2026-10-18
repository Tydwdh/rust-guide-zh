# 52 第 29 章 性能与剖析

Type: task
Status: claimed

## Question

写第五篇第三章：**性能与剖析**。

- **范围**：构建档位与优化（dev/release、`opt-level`/`lto`/`codegen-units`、`profile` 表）；`cargo build --timings`；测量方法学（`Instant` 手写 micro-bench 与 `black_box`；**`#[bench]` 已属 nightly**；criterion 仅作说明性对照、外部依赖 `ignore`）；剖析工具版图（Linux `perf`/flamegraph；Windows 上的等价做法只给方向、不虚构）；PGO（`rustc` 文档流程，说明性）；覆盖率（`-C instrument-coverage`，说明性）；分配与克隆成本（`size_of`、`Vec` 容量增长实测、`clone` vs `Arc::clone`）。
- **素材**：`research/02` 的 I19 行（`cargo/guide/build-performance.html`、`cargo/reference/profiles.html`、`rustc/profile-guided-optimization.html`、`rustc/instrument-coverage.html`、`std/hint/index.html`）；本机 1.98 实测。
- **边界**：并发性能（`loom`/Miri）→ 第 31 章；内存布局 → 第 25 章（回指）；PGO/覆盖率只到「怎么跑」。
- **要求**：所有耗时数字随机器浮动——正文必须写明，不写「必须小于 X ms」式断言；可复现测量放 `snippets/ch29/`（真跑一次粘输出并注明浮动）。
- **篇幅**：4000–5500 汉字；八节模板；练习 2–3 题带答案。
- **DoD**：§8 —— 独占 `src/ch29-performance-profiling.md` + `snippets/ch29/` + 本票；禁 `git`/禁 build；自验 `mdbook test -c ch29-performance-profiling.md` 绿；探针 `.scratch/tmp-52/`；收尾 `claimed` + `## Comments`。

## Comments

**文件（本票独占，均已交付）**
- `src/ch29-performance-profiling.md`：成稿（H1 保持原名），八节模板齐全，内嵌 1 张 SVG（PGO 四步；按 §9：单块、无空行、无脚本、只用主题变量、`width="100%"`、无 `height`、`role="img"` + `aria-label`）。
- `snippets/ch29/bench_instant.rs`：`Instant` + `black_box` 手写 micro-bench，含「结果不消费」对照。
- `snippets/ch29/alloc_count.rs`：自定义全局分配器，数分配次数与字节数。
- `snippets/ch29/capacity.rs`：`Vec`/`String` 容量台阶。
- 未碰 `src/SUMMARY.md`、`book.toml`、`WRITING.md`、`GLOSSARY.md`、地图与其他票。

**自验（§3 + §8；只跑本章，未跑 `mdbook build`，未用 `git`）**
- `mdbook test -c ch29-performance-profiling.md` → 输出 `INFO Testing chapter '第 29 章 性能与剖析'`，**退出码 0**，无 ERROR。
- include 确被测到（防「假绿」）：把 capacity 那个围栏临时改成 `rust,should_panic` 后，`mdbook test` 报 `test did not panic as expected ... FAILED`（EXIT=101），证明 3 个 `{{#rustdoc_include}}` 都进测试集并被执行；随后已还原为 `rust` 并复跑通过。
- 块类型盘点（§1）：`toml`/`console`/`text` 输出块若干；`rust,ignore` 两处（`#[bench]` 的 nightly 形态、criterion 骨架）；`rust,compile_fail` 一处（`#[bench]` 在 stable 的 E0658）；练习二的残缺代码块标 `rust,ignore`（`parse` 故意未定义）。
- `grep -o '[一-鿿]' src/ch29-performance-profiling.md | wc -l` → **5749 汉字**（票面 4000–5500，超上限约 4.5%，见「遗留」）。

**实测过的输出清单**（探针 `.scratch/tmp-52/probe/`，已按要求删除；探针的三个 bin 直接指向 `snippets/ch29/*.rs`，正文数字即这三份源码的输出）
1. micro-bench，release（`cargo run --release --bin bench_instant`）：索引循环 0.09 / 迭代器 0.09 / push 无预留 2.06 / push 有 `with_capacity` 1.16 / `Vec<u64>` clone 21.65 / `Arc` clone 7.22 ns/op；1000 万次加法：结果不消费 **100 ns**、每步走 `black_box` **1 810 900 ns**。
2. micro-bench，dev（同程序）：3.44 / 1.91 / 7.04 / 6.19 / 54.62 / 12.89 ns/op；1000 万次加法 33 292 700 ns vs 40 870 000 ns。
3. release 再跑一轮（浮动证据）：push 2.04 / 1.36 ns，死代码 0 ns——正文写明了这次漂移。
4. `alloc_count`：9 次 / 8176 B（无预留）；1 次 / 4000 B（`with_capacity(1000)`）；8 次 / 2040 B（String 逐字符 push）；2 次 / 24 B（一次 `format!`）；1 次 / 8000 B（`Vec<u64>` clone 1000 元素）；**0 次 / 0 B**（`Arc<Vec<u64>>` clone）；1 次 / 12 B（`Box<str>`）。
5. `capacity`：`Vec<u8>` 8→16 台阶，`Vec<u16>/<u32>/<u64>` 4→8→16；`with_capacity(100)` 后 capacity=100；`shrink_to_fit` 后 =50；`String` 逐字节 8→16。
6. `cargo build --timings`（3 crate 探针，1.98）：控制台末行 `Timing report saved to …\target\cargo-timings\cargo-timing-20261018T180937527Z-44896fca9a86c57a.html`；该目录下确有 `cargo-timing.html` 与带时间戳副本（与手册「两份」一致）。
7. PGO 全流程实跑（stable 1.98，Windows）：`-Cprofile-generate=<abs>` 编译 → 运行产出 `default_*.profraw` → `llvm-profdata merge` 出 `merged.profdata` → `-Cprofile-use` 重编成功；带 `-Cllvm-args=-pgo-warn-missing-function` 时出现「no profile data available for function」16 + 10 条（因为只跑了一个负载）——正文如实解释。
8. 覆盖率实跑（stable 1.98）：`RUSTFLAGS="-C instrument-coverage"` + `LLVM_PROFILE_FILE` → 生成 `cov-<...>.profraw` → `llvm-profdata merge -sparse` → `llvm-cov report`：`capacity.rs` 77 regions / 37 lines / 100%。**本机 stable 上可用**（无需 nightly）。
9. `#[bench]` 在 stable 1.98 的报错（真实跑的是 `cargo bench`）：`E0658: use of unstable library feature 'test': 'bench' is a part of custom test frameworks which are unstable` + `E0433: cannot find module or crate 'test'`；正文 `compile_fail` 块与输出块逐字来自这次运行。
10. profile 取值写错：`opt-level = "fast"` → cargo 原始输出 `error: must be '0', '1', '2', '3', 's' or 'z', but found the string: "fast"`（粘在 `text` 块）。

**引文来源（一手；括号内为核对位置）**
- Cargo Profiles 手册（本机 1.98 快照 `…/share/doc/rust/html/cargo/reference/profiles.html`）：内置档位与按命令选档表、`dev`/`release` 默认值表、`opt-level` 六个取值与「3 档可能比 2 档慢，`"s"`/`"z"` 不一定更小」、`lto` 四值（`false` = thin local LTO，codegen-units=1 或 opt-level=0 时无 LTO）、`codegen-units` 默认 256/16、`panic` 设置被测试/基准/build script/过程宏忽略、`incremental`「抑制优化、不推荐 release」、`strip` 默认 `"none"`、「只看 workspace 根那份 `Cargo.toml`、依赖里的被忽略」、`inherits` 自定义档位与 `target/<profile>/`、build-override 默认 `opt-level = 0`。
- Cargo `timings` 手册：`--timings` 的说明、两份 HTML、单元图/并发图/末表的读法、四条排查建议（慢依赖 / 重复版本 / 拆 crate / 解并行瓶颈）。
- Cargo 构建性能指南（`cargo/guide/build-performance.html`）：`[profile.dev.package."*"] debug = false` 与 `debug = "line-tables-only"` 那条建议。
- rustc 手册 `profile-guided-optimization.html`：四步流程、绝对路径的理由、`--target` 隔离 build script、`-Cllvm-args=-pgo-warn-missing-function`、`llvm-tools-preview` 组件与工具不在 PATH。
- rustc 手册 `instrument-coverage.html`：`-C instrument-coverage`、`LLVM_PROFILE_FILE` 与 `%m`/`%p`、默认名 `default_%m_%p.profraw`（1.65 起）、`llvm-profdata merge -sparse`、`llvm-cov report/show`、`--ignore-filename-regex`、以及「数据格式可能变、可能只与编译器随附的工具兼容」。
- rustc 手册 `codegen-options/index.html`：`-C force-frame-pointers` 的取值、`-C incremental` 不推荐用于 release、`-C instrument-coverage` 本身稳定但格式可能变。
- std 文档：`std/time/struct.Instant.html`（单调不减、非 steady、不透明、系统挂起是否计入由平台与版本决定、`duration_since`/`elapsed`/`Sub` 饱和为 0）、`std/hint/fn.black_box.html`（identity 函数、对 `black_box` 可能做什么保持最大悲观、只对输出有效、不得依赖它保证正确性、不提供密码学/安全保证），稳定版本 1.66。
- `releases.md`（1.98 快照）：1.88「Fully de-stabilize the `#[bench]` attribute … will now become a hard error」（1.77 起为 deny 级 future-incompatibility lint）；1.97 v0 symbol mangling 成为默认（剖析器 demangle 相关）。
- 版本口径：全部按 stable 1.98（`rustc 1.98.0 (88d9e12ae 2026-08-18)`）核实；nightly-only 内容（`#![feature(test)]` 的 `#[bench]`）只以 `rust,ignore` 展示并在正文说明，未实跑。

**新增术语（供协调者并表）**
- 「构建档位（profile）」：正文首现括注英文，之后简称「档位」。GLOSSARY 无此条目，请裁决用词。
- 「剖析（profiling）」「采样剖析（sampling）」「基于插桩的覆盖率（instrumentation-based coverage）」「手写 micro-bench」「优化等级（正文保留配置键原文 `opt-level`）」——同上，请并表或确认不收录。

**衔接提示（给协调者/相邻票）**
- 已在正文回指：第 25 章（`size_of` 与布局）、第 20 章（原子读-改-写）、第 9/10 章（零成本抽象）、第 3 章（溢出在 dev/release 的两种行为）、第 31 章（Miri/loom，声明为本章边界之外）、第 37 章（MIR/单态化/`-C` 全表的深水，尚未成稿，作为前向引用）。
- 第 30 章若写 features/workspace/CI，可回指本章 `[profile]` 与 `--timings`；本章只在 29.2 提了一句「关 feature 可能减编译时间」的方向，未展开 features。
- `SUMMARY.md` 中第 29 章条目已是链接，无需改动。

**遗留不确定 / 请协调者裁决**
1. **篇幅 5749 汉字**，超出票面 4000–5500 上限约 4.5%。已砍两轮（7056 → 5749，纯删冗句、未删票面要求的内容）；再压就要动 `black_box` 语义限定、PGO/覆盖率细节或练习三题等票面明列内容。同类已定稿章 ch19=6534、ch20=5819 也超 5500，倾向判断上限偏紧；若必须 ≤5500，请指示允许删哪一类。
2. `lto = "yes"` 一类未知字符串在 1.98 上**未报错**（cargo 静默接受，探针实测后已还原），因此正文没有把它写成「会报错」，只在报错现场陈述合法取值。若要点名这一点，需先确认 cargo 对未知 `lto` 字符串的实际语义。
3. Windows 采样路线按票面要求「只给方向」：提到 ETW（WPA 之类工具）与 IDE 自带性能探查器，**未给命令、未在本机做采样实测**。
4. PGO 的真实收益未量化（只验证流程跑通）；「同机 PGO 前后对比」需另开探针才能给数字。
5. `-C instrument-coverage` 本机 stable 1.98 实测可用（已贴报告）；但 rustc 手册只承诺 nightly 发行版默认带 profiler runtime，正文据此写成「本机 stable 实测可用」，未推广到所有 stable 平台/目标。

**审校回执处理（RvCh29；F1–F9 全部接受，逐条如下）**

- **F1（blocker · `lto`）**：接受。复核证据：`lto = "yes"` → `cargo build --release -v` 显示透传 `-C lto=yes`，EXIT=0、零警告；`-C lto=bogus` 的报文自列合法值。已删掉「最常遇到的错误是把 `lto` 写成 `"yes"`」这句，改为事实版：Cargo 对 `lto` 字符串基本原样透传、`lto = "yes"` 能编过、`-C lto` 本身就收 `yes`/`no`/`on`/`off`，并附 `-C lto=bogus` 的**逐字**报文（放在 `text` 块里，保留原反引号）。
- **F2（major · `%m`）**：接受。查 rustc 手册 `instrument-coverage.html` 的 `LLVM_PROFILE_FILE` 模式表：`%p` = 进程号、`%h` = 主机名、`%m` = **插桩二进制的签名**（`%Nm` 是签名对应的 N 个 raw profile 池）。已改为「`%m` 是**插桩二进制的签名**（每个二进制各不相同），`%p` 是进程号，`%h` 才是主机名」。
- **F3（minor · 量级）**：接受。实测比值 38×（3.44/0.09）与 21×（1.91/0.09），不是两个数量级。已改「dev 档下两者都慢了二三十倍」。
- **F4（minor · 行号）**：接受。用与正文完全相同的那 3 行做最小复现重跑 `cargo bench`，实得 `1:3` 与 `2:22`（旧粘贴来自带首行注释的探针文件）。已改为 `--> src\lib.rs:1:3` / `1 | #[bench]` / `--> src\lib.rs:2:22` / `2 | fn bench_add…`。
- **F5（minor · 覆盖率表）**：接受，取你给的第二条路（注明压缩）。重跑确认原输出为 5 行：表头 / `---`（168 个）/ 数据行 / `---` / TOTAL，行宽 227。已在表后加披露句「（`llvm-cov` 的输出还有 `---` 分隔行与更宽的列，这里为排版删去并压紧，数字未改。）」；表内数字（77 / 2 / 37 / 0、100%）与原输出逐格一致。若你要逐字长行版，说一声我换。
- **F6（minor · criterion::black_box）**：接受。骨架改为 `use std::hint::black_box;`，并在正文点一句「（`black_box` 就用标准库那个。）」。未在正文断言它「已废弃」——那属于外部 crate 的版本事实，本轮没核对，不写进书。
- **F7（minor · 译自）**：接受。改为「文档对它的定义是（译自 std 文档）「一个恒等函数，……最大程度的悲观」」。
- **F8（minor · Windows 自相矛盾）**：接受。改为「Windows 没有 `perf` 那样开箱即用的命令行采样器；系统级的采样设施是 ETW，读它的工具（例如 Windows Performance Analyzer）与 IDE 自带的性能探查器是这条路线上的两个方向」。
- **F9（minor · 探针路径）**：接受。正文粘贴里的 `D:\Rust\.scratch\tmp-52\probe…` 已全部换成中性示例路径：`--timings` 块用 `C:\work\demo`（4 处），PGO 块用 `C:/work/demo/pgo-data`（2 处），并保留「项目目录以 `C:\work\demo` 为例」。覆盖率块的 `D:\Rust\snippets\ch29\capacity.rs` 按你的意见保留。为保证替换后数字仍是真跑结果，我在新探针里**重跑**了 `--timings`、覆盖率与 PGO 三步（结果与旧输出一致，只有时间戳/符号哈希更新），正文用的是这一轮输出：`--timings` 3 行 `Compiling` + `Timing report saved to …`；覆盖率 77/37/100%；PGO 16+10 条 missing-profile 警告。

- **复验**：`mdbook test -c ch29-performance-profiling.md` → `INFO Testing chapter '第 29 章 性能与剖析'`，**exit 0**。最新字数 `grep -o '[一-鿿]' | wc -l` = **5882 汉字**（审校前 5749；增量来自 F1/F2 的补充说明与 F5 披露句；仍超票面上限约 7%，见上面「遗留」第 1 条）。探针 `.scratch/tmp-52/` 已再次删除。
