# 54 第 31 章 测试进阶

Type: task
Status: claimed

## Question

写第五篇第五章：**测试进阶**。

- **范围**：单元/集成/doctest 的分工与陷阱（`#[test]`/`#[cfg(test)]` 不被 rustdoc 编译——书里常见的假绿坑；`mdbook test` 绿 ≠ 测过）；`compile_fail`/`no_run`/`should_panic` 的语义边界；参数化（表驱动 helper 的写法）；属性测试（proptest，说明性外部依赖）；快照测试（insta，说明性）；`cargo nextest`（说明性）；Miri/loom 在 CI 里的位置（nightly、外部 crate，说明性；回指 23/24 章的 Miri 用法）。
- **素材**：`research/02` 的 I21 行（书 ch11、`rustdoc/write-documentation/documentation-tests.html`、`cargo/guide/tests.html`、`cargo/commands/cargo-miri.html`）；本机 1.98 实测（`cargo test` 各能力真跑）。
- **边界**：Miri 细节 → 第 23/24 章（回指）；loom 与并发 → 第 19/20 章（回指）；CI 模板 → 第 30 章（回指）；`#[expect]`/lint → 第 32 章。
- **篇幅**：4000–5500 汉字；八节模板；练习 2–3 题带答案；外部工具只作说明、`ignore`/文字块。
- **DoD**：§8 —— 独占 `src/ch31-advanced-testing.md` + `snippets/ch31/` + 本票；禁 `git`/禁 build；自验 `mdbook test -c ch31-advanced-testing.md` 绿；探针 `.scratch/tmp-54/`（含真跑 `cargo test` 的小 crate）；收尾 `claimed` + `## Comments`。

## Comments

### 文件

- `src/ch31-advanced-testing.md`：全章成稿，八节模板齐全（这一章解决什么问题 / 为什么这样设计 / 机制与规则 / 动手看看 / 报错现场 / 小结 / 练习 3 题带答案 / 延伸阅读），H1 标题保持占位时的原文未动；正文 CJK 约 5490 字（在 4000–5500 内）。
- `snippets/ch31/`：新建可跑 crate，包名 `ch31-demo`，只含 `Cargo.toml` + `src/lib.rs`（无外部依赖、无 `Cargo.lock`、`target/` 已删，与 `snippets/ch11/` 一致）。内容：`LogLevel::from_str`（带 `Result` 错误类型）+ 表驱动测试、`normalize_word` + 表驱动测试、`port_or_panic` + `#[should_panic(expected)]`，以及默认/`no_run`/`should_panic`/`compile_fail` 四种 doctest。

### 自验

- `mdbook test -c ch31-advanced-testing.md` → **exit 0**（章内 5 个受检块：隐藏行默认块、`no_run`、`should_panic`、`compile_fail`；其余是 `ignore`/`text`/`console`）。
- 反向确认章内块真在编（防“绿得空洞”）：临时在章尾注入含 `let x: i32 = "临时类型错误";` 的普通块 → `mdbook test -c ch31-advanced-testing.md` **exit 101**（输出 11 个块、1 failed），随后恢复。
- `cargo test`（在 `snippets/ch31/`）→ 5 个单元测试 + 7 个 doctest 全绿；`cargo test --lib` → 5；`cargo test --doc` → 6 + 1（`compile_fail` 单独一批）。

### 实测过的行为清单（探针 `.scratch/tmp-54/`，收尾已删）

1. 章内 `#[test]` 块写 `let got: i32 = "8080";` + `mdbook test -c ch31-advanced-testing.md` → exit 0（假绿）。
2. 同码进 crate 的 `#[cfg(test)] mod tests` + `cargo test` → E0308，exit 101。
3. `#[test]` 块里放语法错误 + `rustdoc --test` → `error: this file contains an unclosed delimiter`，exit 101（解析先于条件编译）。
4. `no_run` 块里放类型错误 → `- compile ... FAILED`，exit 101。
5. `should_panic` 块不 panic（crate 真跑 `cargo test --doc`）→ `note: test did not panic as expected at src\lib.rs:103:0`，exit 101。
6. 编译失败的块标 `compile_fail,E0277`（真实错误码 E0308），stable → 通过（错误码被忽略）。
7. 能编译的块标 `compile_fail,E0277`，stable → 失败：``Test compiled successfully, but it's marked `compile_fail`.``
8. doctest 顶层 `?` → E0277（包装出的 `fn _doctest_main_…` 返回 `()`）。
9. `examples/broken.rs` 类型错误 + `cargo test --no-run` → `could not compile … (example "broken")`，exit 101。
10. 表驱动断言带输入、实现故意写错 → ``assertion `left == right` failed: 输入 "Two  Words"``，exit 101。

正文粘贴的 `cargo test` / `cargo test --doc` / `cargo test --lib` 输出都是上表 5 与 crate 全绿那次的真实输出（含 1.98 的两批 doctest 与 `merged doctests` 一行）。

### 引文来源（逐字核对）

- rustdoc 书 `write-documentation/documentation-tests.html`（本机 1.98 文档快照）：`ignore`/`no_run`/`should_panic`/`compile_fail` 定义、`?` 的两种解法（自写 `fn main() -> Result` 或 `# Ok::<(), E>(())`）、`merged doctests` / `standalone_crate`。
- rustdoc 书 `unstable-features.html`（快照）原文：“Attempting to use these error numbers on stable will result in the code sample being interpreted as plain text.”
- `rust-lang/miri` README：定位句 “Miri is an Undefined Behavior detection tool for Rust.”、`cfg(miri)`、`MIRIFLAGS` 与默认隔离、Windows 建议 `--target x86_64-unknown-linux-gnu`、`cargo miri nextest run` 与「检测不到共享资源竞争」的差异、CI 片段、soundness 边界（“when Miri does not find UB …”）。
- `tokio-rs/loom` README（docs.rs 0.7.2 源）：定位句、`[target.'cfg(loom)'.dependencies]`、`RUSTFLAGS="--cfg loom" cargo test --release`、`SeqCst` 当 `AcqRel`（误报）与 load buffering 未探索（不 sound）。
- `nexte.st`（官网 + `/docs/design/how-it-works`）：两阶段 list/run、进程隔离、重试/慢测试/压力循环、filterset、CI 归档/分片/JUnit、“Up to 3× faster”，以及 doctest 原文 “Doctests are currently not supported … run doctests in a separate step with `cargo test --doc`.”
- `proptest` README：QuickCheck 家族、per-value 生成与 shrink、`proptest-regressions/` 入版本控制、MSRV 1.88。
- `insta` 官网（quickstart + advanced）：`.snap.new` 与 review 流程、`INSTA_UPDATE` 模式、`CI` 变量下的行为、内联快照 `@"…"`、`glob!`、`with_settings!`。

### 未实跑（正文均已标“说明性”）

- `proptest`、`insta`、`cargo nextest`：未安装、也不在本机装（避免引入网络依赖）；相关代码只出现在 `rust,ignore` 块。
- Miri：本机无 nightly，只给命令与官方事实，并显式标注 nightly-only。
- loom：未实跑，仅引 README。

### 遗留不确定

- **loom 是否需要 nightly**：README quickstart 只给 `RUSTFLAGS="--cfg loom" cargo test --release`，未提 nightly，正文据此写「不要求 nightly 工具链」并括注这是 quickstart 的说法。若协调者要更保守，可删去该括注或改成「以官方文档为准」。
- 「`cargo test` 会编译 `examples/`」是本机 1.98 实测，未逐字核对到官方文档的同一句表述。

### 新增术语（请协调者并表）

`GLOSSARY.md` 现有测试小节只覆盖单元/集成/端到端/断言/测试运行器/测试框架/文档测试。本章新引入：**表驱动（table-driven）**、**属性测试（property-based testing）**、**快照测试（snapshot testing）**、**假绿（false green，非正式用语，正文加粗解释）**。另在 rstest 一句里用了「夹具」（fixture），若不愿引入可改写。

### 图示需求（已内嵌）

两张 SVG：① 同一段测试代码的两条通道（rustdoc 关 `cfg(test)` vs `cargo test` 开）；② CI 四道闸门顺序。均符合 §9：单块无空行、无脚本无外链、`viewBox` + `width="100%"` 无 `height`、只用主题变量、`role="img"` + `aria-label`、图内 `&`/`<`/`>` 无裸用。

### 衔接提示

- 回指：第 11 章（测试基础）、第 19/20 章（loom 与并发）、第 23/24 章（Miri 用法）、第 25 章（属性测试的适合目标）、第 30 章（CI 模板）。
- 未涉及 `#[expect]`/lint（留给第 32 章）。第 30 章若给工作流模板，建议至少含一条 `cargo test --doc`（nextest 不跑 doctest）与可选的 miri job。
- 未改 `src/SUMMARY.md`、`book.toml`、`GLOSSARY.md`；未跑 `mdbook build`、未用 `git`。

### 审校回执修复（RvCh31，5 条；2026-10-19）

- **F-01 已修**（并按可选建议引入实测）：`should_panic` 一句改为「和**不带 `expected` 的** `#[should_panic]` 一样，只断言『panic 了』；只有带 `expected` 的那条属性才做子串匹配」，与第 11 章一致。新增一条 `> [!WARNING]`：fence 写 `should_panic,expected="…"` 无效。**自测证据**（`rustdoc --test --edition 2024`）：打到 stderr 的 `warning: unexpected character '='`（lint `rustdoc::invalid_codeblock_attributes`），同一文件里普通 `should_panic` 块照跑（`running 1 test … should panic ... ok`），而带 `expected=` 的块**根本不出现在测试列表**，exit 0。措辞用「只给一句警告 + 整个块不参与测试」，避开「静默」二字（确有警告）。
- **F-02 已修**：旗舰假绿块去掉 `,ignore`（改普通 `rust`），并在引出处加「故意不加 `ignore`——它要走 rustdoc 通道」。**自测证据**：临时在章尾注入普通坏块逼 mdbook 打印完整列表，该块显示为 `… ::机制与规则::假绿_ (line 55) ... ok`（块内就是那条 `let got: i32 = "8080";` 的 E0308，`cfg(test)` 关着），随后恢复；恢复后单跑 `mdbook test -c ch31-advanced-testing.md` 仍 exit 0。
- **F-03 已修**：改为 edition 口径——「这个 crate 是 edition 2024，rustdoc 会把能合并的 doctest 合并成一个文件编译（输出里的 `merged doctests` 一行；官方说明这条行为从 **2024 edition** 起）」。**自测证据**：同一文件两个可合并块，`rustdoc --test --edition 2021` 无 `merged doctests` 行、`--edition 2024` 有该行。
- **F-04 已修**：改为「块因为拼错函数名、漏一个 `use`，甚至语法错误（本机实测：放一个未闭合的括号照样通过），任何原因挂掉都算通过」。**自测证据**：`compile_fail` 块里写 `fn broken( { }` → `- compile fail ... ok`，exit 0。
- **F-05 已修**：「本机 Windows 10」→「本机 Windows 11」（10.0.26200 属 Windows 11 构建号；系统报的 `win32 10.0.26200` 只是 NT 版本号）。

**修复后复验**：`mdbook test -c ch31-advanced-testing.md` → **exit 0**；正文 CJK **5484**（新增 F-01 警告框，做了等量删减，仍在 4000–5500）；`cargo test`（`snippets/ch31/`）仍 5 单元 + 7 doctest 全绿；探针目录、`target/`、`Cargo.lock` 已再次删除。五条全部接受，无异议。
