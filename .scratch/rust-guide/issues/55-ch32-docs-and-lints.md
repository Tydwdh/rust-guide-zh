# 55 第 32 章 文档与 lint 工程

Type: task
Status: resolved

## Question

写第五篇第六章：**文档与 lint 工程**。

- **范围**：rustdoc（文档注释、`#![warn(missing_docs)]`、doctest 的隐藏行 `#`、`compile_fail` 标注、文档里的 `should_panic` 关联）；intra-doc link（`[Type]`/`[fn]` 写法与 `-D rustdoc::broken_intra_doc_links`）；`#[doc(cfg)]`（nightly）与 `#[cfg_attr(docsrs, doc(cfg(...)))]` 技巧（docs.rs 场景，说明性）；clippy 分组（correctness/style/complexity/perf）与常用 lint 取舍；`-D warnings` 的策略与**禁止 `deny(warnings)` 进库**的理由（下游编译被卡）；`#[expect]`（核对稳定版本）与 `[lints]` 表（衔接第 30 章）。
- **素材**：`research/02` 的 I22 行（`rustdoc/`、`clippy/`、`rustc/lints/index.html`）；本机 1.98 实测（探针 crate 真跑 `cargo doc` + `RUSTDOCFLAGS="-D warnings"` 与 `cargo clippy`）。
- **边界**：`[lints]` 表/workspace 继承 → 第 30 章（回指）；Edition 迁移 lint 组 → 第 33 章；测试面 → 第 31 章。
- **篇幅**：4000–5500 汉字；八节模板；练习 2–3 题带答案。
- **DoD**：§8 —— 独占 `src/ch32-docs-and-lints.md` + `snippets/ch32/` + 本票；禁 `git`/禁 build；自验 `mdbook test -c ch32-docs-and-lints.md` 绿；探针 `.scratch/tmp-55/`（探针 crate 里跑 doc/clippy 取真实输出）；收尾 `claimed` + `## Comments`。

## Comments

（票 55 写作者；下面每条 = 结论 / 证据命令+结果 / 引文来源 / 遗留）

**结论**：第 32 章成稿，八节模板齐全，篇幅 5498 汉字（`python` 计 `[\u4e00-\u9fff]`）；自验 `mdbook test -c ch32-docs-and-lints.md` **EXIT=0（绿）**；章内 13 个可编译块全过、2 个 `rust,ignore` 块按设计跳过。正文所有实机输出都来自本轮探针真跑。

**文件**
- `src/ch32-docs-and-lints.md`（新写；H1 保持占位标题「第 32 章 文档与 lint 工程」）
- `snippets/ch32/doc_lib.rs`（`{{#rustdoc_include}}`；演示三种注释位置 + intra-doc link）
- `snippets/ch32/gated.rs`（`cfg_attr(docsrs, doc(cfg(...)))`；stable 上可编译）
- 本票（未碰 `SUMMARY.md` / `book.toml` / `WRITING.md` / `GLOSSARY.md` / 其它票）

**自验命令与结果**
- `mdbook test -c ch32-docs-and-lints.md` → `INFO Testing chapter '第 32 章 文档与 lint 工程': "ch32-docs-and-lints.md"`，EXIT=0
- 门禁反证：临时把章内隐藏行 `# fn helper() -> i32 { 41 }` 改成 `"41"` 再跑同一命令 → `test result: FAILED. 12 passed; 1 failed; 2 ignored`、EXIT=101；还原后复跑 EXIT=0。证明章内块确实被 rustdoc 编译，且**隐藏行也在编译范围内**（与正文 32.2 的断言一致）。

**实测输出清单**（探针目录 `.scratch/tmp-55/`；rustc/cargo/clippy 1.98 stable，Windows 10）
1. `docprobe`（`[lints.rust] missing_docs = "warn"` + `[lints.rustdoc] broken_intra_doc_links = "warn"`）
   - `cargo doc --no-deps` → 2 warnings（missing docs / unresolved link），EXIT=0
   - `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` → 2 errors，EXIT=101（正文 32.4 与「动手看看」贴了关键行）
   - `cargo build` → 末行 `= note: requested on the command line with \`-W missing-docs\``（证明 `[lints]` 表被转成 CLI 级别、且对 rustdoc 那次调用同样生效）
2. `lintprobe`（每群组埋一条）
   - `cargo clippy` → `error: equal expressions as operands to \`==\``（`#[deny(clippy::eq_op)]` on by default）+ 4 warnings（single_char_add_str / needless_range_loop / identity_op / useless_vec），EXIT=101
   - `cargo clippy -- -W clippy::pedantic` → 11 条诊断（多出 must_use_candidate / cast_possible_truncation / missing_panics_doc）
   - `cargo clippy -- -W clippy::restriction` → 首条 `warning: clippy::restriction is not meant to be enabled as a group`，help `enable the restriction lints you need individually`
   - `[lints.clippy] pedantic = "warn"` 同样能打开 pedantic（实测出现 casting / must_use / missing_panics_doc）
3. `doctestprobe`：`cargo test --doc` → 隐藏行块 `ok`、`# fn main() -> Result<...>` 配 `?` 块 `ok`、```should_panic``` 报 `- should panic ... ok`、```compile_fail``` 报 `- compile fail ... ok`；把隐藏行改成 `# let n: i32 = "..."` 立刻 E0308 失败
4. `expectprobe`：`#[expect(unused_variables, reason = "...")]` 未兑现 → `warning: this lint expectation is unfulfilled`，`reason` 原文作为 note 打出；同 crate 另一处已兑现的 `#[expect]` 完全沉默
5. `docsrsprobe`：裸写 `#[doc(cfg(...))]` → `error[E0658]: \`#[doc(cfg)]\` is experimental`（issue #43781）；`#![feature(doc_cfg)]` → `error[E0554]`；`RUSTDOCFLAGS="--cfg docsrs" cargo doc` → E0658；不带 `docsrs` 时 `cfg_attr(docsrs, doc(cfg(...)))` 编译通过
6. `cfgdoc`：`#[cfg(doc)]` 在 stable 1.98 直接可用（无需 feature 开关）
7. `incstr`：`#![doc = include_str!("../README.md")]` 编译通过；README 里写错的断言让 `cargo test --doc` 失败，改对后 `ok`
8. `wscap`（「下游被卡」的实测）：`dep` 带 `#![deny(warnings)]` + 一处 `unused_variables`；下游以 path 依赖（同一 workspace）→ `cargo build` EXIT=101，`error: could not compile \`dep\` (lib) due to 1 previous error`（正文贴了完整关键段）；`cargo build -v` 的 rustc 命令行里**没有** `--cap-lints`
9. `capcheck`（registry 依赖对照）：`cargo build -v --offline` 中 `cfg-if` 的 rustc 命令行有 `--cap-lints allow`（1 次）；加上 `RUSTFLAGS="-D warnings"` 后仍然有 → CI 的 `-D warnings` 不会被依赖拖爆

**引文来源**（正文引用的英文原句逐字核对；本地快照 `C:\Users\tyd27\.rustup\toolchains\1.98.0-x86_64-pc-windows-msvc\share\doc\rust\html\`；docs.rs 两页取自官方在线页）
- The rustdoc book：`write-documentation/documentation-tests.html`（`#` 隐藏行原话、预处理算法、`compile_fail` 断言会随版本失效的提醒）、`write-documentation/linking-to-items-by-name.html`（intra-doc link 定义、作用域、不「像链接」就忽略）、`lints.html`（`broken_intra_doc_links` 默认 warn；`missing_docs` 默认 allow 且是 rustc 侧唯一可用者）、`unstable-features.html`（`doc(cfg)`）
- The rustc book：`lints/levels.html`（`--cap-lints` 与 Cargo 的关系）
- Clippy 文档：`index.html`（`clippy::all` 定义与各组一行说明）、`lints.html`（各组取舍）、`configuration.html`（`clippy::pedantic` 误报提醒）、`continuous_integration/index.html`（CI 推荐 `-Dwarnings`）、`continuous_integration/github_actions.html`（`cargo clippy --all-targets --all-features`）
- The Cargo Book：`reference/manifest.html`（`[lints]` 表规则、只作用于当前包）、`reference/semver.html`（`deny(warnings)` 的兼容性风险与 `-Adeprecated` 建议）
- Rust Reference：`attributes/diagnostics.html`（`#[expect]` 语义与 `unfulfilled_lint_expectations`）
- `releases.md`（1.98 快照内）：`#[expect]` 稳定于 1.81.0（Stabilize `#[expect]` for lints (RFC 2383)）
- docs.rs 官方：`/about/builds`（nightly 构建；`docsrs` cfg 只作用于最终 rustdoc 调用）、`/about/metadata`（`[package.metadata.docs.rs]`）

**遗留不确定**
- `--cap-lints allow` 是否同样覆盖 **git 依赖**未实测（本机无网络、且本票禁 `git`）；正文只写了实测覆盖的 registry / path / workspace 三种。
- `#[doc(cfg)]` 的 nightly 侧（`#![feature(doc_cfg)]` 的真实展开、`doc(auto_cfg)`）只依据 unstable book 与 docs.rs 说明，**未真跑**；正文已按 WRITING §2 标注 nightly-only，并给出 stable 上的实测 E0658/E0554 证据。
- 章内 2 个 `rust,ignore` 块按设计不参与编译：练习 1（库根 `#![deny(warnings)]` + 真实无用变量）、练习 2（文档注释里嵌 doctest，含嵌套栅栏，mdBook 无法在单个块内显示内层 ```）。两者的真实行为都在探针里真跑过（见上 8 与 3）。
- `mdbook test -c` 成功时不打印逐条用例；13 passed / 2 ignored 的计数取自上述反证那次的输出，未再单独确认。

**新增术语（建议协调者并表 GLOSSARY）**
- **rustdoc**：Rust 文档工具（保留英文，同 `lint`）；**clippy**：官方 lint 工具（保留英文）
- **intra-doc link（文档内链接）**：文档注释里用项路径写的相对链接（`` `[`Type`]` ``、`` `[text](path)` ``）
- **隐藏行（hidden line）**：doctest/代码块里以 `#` 开头、渲染收起但参与编译的行
- **文档门禁**：`RUSTDOCFLAGS="-D warnings" cargo doc` 这类把文档诊断变硬失败的 CI 步骤

**衔接提示**
- 32.7 只讲 `[lints]` 的规则与作用域，workspace 级继承（`[workspace.lints]`、`priority`、`[lints] workspace = true`）明确回指第 30 章；第 30 章定稿后请核对示例键名是否需要对齐。
- doctest 与测试组织、`should_panic` 的 libtest 形态 → 第 31 章；Edition 迁移牵动的 lint 组 → 第 33 章。
- 本章「动手看看」的两个探针 crate 都在 `.scratch/tmp-55/`，按 §8 收尾已删除；正文保留的是真跑输出。

**图示需求**：已加 1 张内嵌 SVG（doctest 流程：源码注释 → rustdoc 抽取 → 补 allow/包 `fn main` → 编译运行 → ok/FAILED）。按 §9：单块无空行、`viewBox` + `width="100%"` 且根元素无 `height`、只用主题变量（`currentColor`/`var(--links)`/`var(--quote-border)`）、`role="img"` + `aria-label`；XML 解析通过。lint 政策作用范围改用表格表达，未画图。

## Comments（第二轮：RvCh32 审校回执处理）

**结论**：1 major + 5 minor + 1 存疑 + F9（批次卫生）**全部接受并已修**；只动 `src/ch32-docs-and-lints.md` + 本票（`snippets/ch32/` 未改：两处片段本身无问题）。修完复跑 `mdbook test -c ch32-docs-and-lints.md` → **EXIT=0**；汉字数 5495（仍在 4000–5500）；栅栏 54 行成对、SVG 重新 XML 解析通过、八节标题齐全；已 grep 确认章内不再出现 `tmp-55` / `.scratch` / `D:\Rust` 字样；探针 `.scratch/tmp-55/` 已再次删除。

- **F1（major）· 接受方案②**。章内 clippy 探针块补上真正产出这两条诊断的两个函数（`to_i32`：`x as i32`；`parse`：`s.parse().unwrap()`），数字改为与代码块一致的实测值。重建探针复测（`.scratch/tmp-55/lintprobe`，Rust 1.98）：`cargo clippy` → `generated 4 warnings` + `1 previous error`（`#[deny(clippy::eq_op)]`），EXIT=101；`cargo clippy -- -W clippy::pedantic` → `generated 11 warnings`，按 lint 名统计为 single_char_add_str / needless_range_loop / identity_op / useless_vec 各 1 + `must_use_candidate` ×5 + `cast_possible_truncation` 1 + `missing_panics_doc` 1 = 11。正文口径随之改为「11 条警告——默认那 4 条之外多出 `must_use_candidate` ×5、`cast_possible_truncation`、`missing_panics_doc`」。
- **F2（minor）· 接受**。32.2 改为「rustdoc 包出来的 `fn main()` 返回 `()`」；第二种解法写明 `# Ok::<(), 你的错误类型>(())`——类型必须写全、`(())` 连写。新增实测反证：`rustdoc --test` 跑只写 `# Ok(())` 的块 → `error[E0282]: cannot infer type of the type parameter \`E\``（0 passed; 1 failed）。
- **F3（minor）· 接受**。32.5 的 E0554 行与 32.6 的 `clippy::restriction` 警告行改为双反引号整行包裹（` ``…`` `），与本章报错现场的写法统一。
- **F4（minor）· 接受**。改为「官方 CI 示例给的是 `cargo clippy --all-targets --all-features`，再配 `RUSTFLAGS="-Dwarnings"`（或 `-- -D warnings`）」。依据：clippy 书 `continuous_integration/github_actions.html` 的示例命令只有 `cargo clippy --all-targets --all-features`；`-Dwarnings`/`RUSTFLAGS` 的推荐出自 `continuous_integration/index.html`，两者是两页的内容，此前被合并成一句。
- **F5（minor）· 接受**。终端记录不再有未标注省略：32.3 在命令后与末尾各加 `…`；32.4 补回 ` Documenting docprobe v0.1.0 (…)` 行并在末尾加 `…`；「动手看看」默认那条补回 ` Documenting` / `Finished` / `Generated` 三行、并把 `-->` 缩进修正为真实的一空格；`-D warnings` 那条补成完整输出（第二条 error、两条 `= help: to override …`、末行 `error: could not document` 与 `$ echo $?` → 101）；32.7 `#[expect]` 那条末尾加 `…`。
- **F6（minor）· 接受**。术语对照前移到首现处：24 行改「直接读源码里的文档注释（doc comment）」，32.1 节首处改为普通「文档注释」。
- **F9（协调者加派）· 接受**。正文三处内部路径改中性路径：` Documenting` 与 `Generated` 两行用 `C:\work\docprobe`，deny(warnings) 实测输出两行用 `C:\work\wscap\dep\src\lib.rs`；叙述（「探针里被依赖的库写了 …」）无需改，已保持一致。
- **存疑项 · 接受软化**。32.5 元数据段改为「Builds 页只讲了『`docsrs` cfg 只作用于最终那次 rustdoc 调用』；本节据此推断多数 crate 今天不必自己传（官方 Metadata 页没有对应文字）。」——删掉了「现在 docs.rs 会自己加」这一无一手出处的断言。
- 无反驳项；无新增遗留。

## Comments（第三轮：协调者 build 实测 · `:134` 行内代码配对）

**结论**：`src/ch32-docs-and-lints.md:134` 的 `\`` 转义已去掉，改为双反引号整行包裹；`mdbook test -c ch32-docs-and-lints.md` → EXIT=0；章内 `\`` 序列计数 0；汉字数 5495。只动该章 + 本票。

- **旧 → 新**：`会报 ` + 单反引号 + `E0282: … \`E\`` → 双反引号整行包裹，收尾 `` `` `` 前留一个半角空格。
- **与协调者给的字符串有一处微调，理由如下**：本票最终采用 ` `` E0282: cannot infer type of the type parameter `E` `` `（在 ` `` ` 后多一个前导半角空格），而不是 ` ``E0282: …`` `。CommonMark 只有在代码段内容**同时**以空格开头和结尾时才各剥掉一个；协调者给的形式只满足「结尾有空格」，于是那个空格留在代码段内部，渲染成 `<code>…`E` </code>`——中文逗号前会多一个可见间隙。补一个前导空格后，两个空格成对剥掉，输出干净。协调者约束的实质（收尾双反引号前必须有半角空格）仍然满足。
- **渲染自查**（私有探针书，未碰仓库 `book/`）：把该行原样贴进 `.scratch/tmp-55/probebook/src/probe.md`（自带 book.toml，输出在探针目录内）后 `mdbook build`，两种形式对比：
  - 字面形式 ` ``E0282: … `E` `` ` → `<code>E0282: cannot infer type of the type parameter `E` </code>`
  - 本票采用 ` `` E0282: … `E` `` ` → `<code>E0282: cannot infer type of the type parameter `E`</code>`
  - 两者均为**单个** `<code>`、反引号按字面显示、无反斜杠；差别只在尾空格。
- 同章 `:200/:461/:491` 等围栏内的 `\[` 未动（协调者确认不受影响）。
- 探针目录 `.scratch/tmp-55/` 已删除（含 probebook）。

## Answer

2026-10-19 批次收尾（用户验收通过）。

- 交付：`src/ch32-docs-and-lints.md` + `snippets/ch32/`（doc_lib.rs/gated.rs，均经 `{{#rustdoc_include}}` 受测）。
- 自验：`mdbook test -c` exit 0（双注入反证）；docprobe/wscap/lintprobe 三个探针 crate 真跑 `cargo doc`/`-D warnings`/clippy 分组/`#[expect]`/cap-lints，逐字核对。
- 交叉审校 RvCh32（1 major + 5 minor + 1 存疑）全修：clippy 探针补两个真产诊断的函数（11 条口径落实）、`?` 写 turbofish、双反引号整行引诊断、clippy CI 示例分页归位、终端记录省略标注、术语首现、docs.rs 句软化；协调者加派三处探针路径中性化 + `:134` 行内代码配对修正。
- 术语并表（协调者）：rustdoc/clippy/intra-doc link/隐藏行入 GLOSSARY。
- 全量链（批次级）全绿。
- 遗留：nightly 侧（`doc(cfg)` 真渲染）未跑；git 依赖 `--cap-lints` 未实测。

