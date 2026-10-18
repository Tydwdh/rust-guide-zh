# 63 第 35 章 别名模型与形式化

Type: task
Status: claimed

## Question

写第六篇第二章：**别名模型与形式化**（素材 = D2：UCG 站点、Nomicon `aliasing.html`）。

- **范围**：Stacked Borrows 与 Tree Borrows 的机制对照（栈式借出/回收 vs 树式许可）；UCG 的定位（**进行中的非规范工作**，引用必须标注）；形式化与编译器现状的差距（1.98 是否默认启用任一所列模型——以官方现状核实后写）；provenance 细节深化（承接 ch24：严格/暴露两层结论、`&raw` 的意义）；Miri 的角色与使用方式（命令级文档化；**本机无 nightly，不得伪造输出**，用 `> [!WARNING]` 标注）。
- **要求**：概念引用逐字对官方页（快照/在线），RFC/issue 编号逐条核；所有「实测」必须是 stable 面证据。
- **边界**：Miri 用法细节 → ch23/24 回指；provenance API → ch24 回指（不重讲）；unsafe 基础 → ch23。
- **篇幅**：4000–5500 汉字；八节模板；练习 2–3 题带答案；按 §9 内嵌 1–2 张 SVG（如两种模型对照）。
- **DoD**：§8 —— 独占 `src/ch35-aliasing-models.md` + `snippets/ch35/`（若有可编译演示）+ 本票；禁 `git`/禁 build；自验 `mdbook test -c ch35-aliasing-models.md` exit 0；探针 `.scratch/tmp-63/`；收尾 `Status: claimed` + `## Comments`。

## Comments

**做了什么**

- 成稿 `src/ch35-aliasing-models.md`（八节模板齐全；正文 CJK 5498 字；H1 保持占位原文）。
- 新建 `snippets/ch35/sb_fails.rs`（改编自 Miri 测试 `tests/pass/tree_borrows/sb_fails.rs`，两个代表性用例；经 `{{#rustdoc_include}}` 进正文，被 mdbook test 编译并执行）。
- 新建 `snippets/ch35/ir_probe.rs`（`--crate-type=lib` 的 LLVM IR 探针；**不经 mdbook test**，用 rustc 手动验证，文档化的命令写进正文与该文件头注释）。
- 两张内嵌 SVG（栈 vs 树结构对照；「同一次读，两种后果」），单块无空行、无脚本、无 `height`、颜色只用主题变量。

**自验命令与结果**

- `mdbook test -c ch35-aliasing-models.md` → `INFO Testing chapter '第 35 章 别名模型与形式化'`，exit 0（多次复跑，含收尾一次）。
- `rustc --edition 2024 snippets/ch35/sb_fails.rs -o … && 运行` → 编译 exit 0、运行 exit 0、**无输出**（正文照此写）。
- `rustc --edition 2024 --crate-type=lib -O --emit=llvm-ir snippets/ch35/ir_probe.rs --out-dir target/tmp` + `grep -n "define void @compute"` → 正文那行 IR 原样（第 7 行，`noalias`，`input` 带 `readonly`，两个都有 `dereferenceable(4)`）；正文那段带 `main` 的例子实跑输出 `2`。
- `rustc --edition 2024`（`invalid_reference_casting` 最小复现）→ 正文「报错现场」那段 1.98 输出逐字。
- 未跑 `mdbook build` / `git`（按批次协议）；探针目录 `.scratch/tmp-63/` 与 `target/tmp` 已删除。

**实测输出清单（真跑真粘）**

1. `sb_fails.rs` 运行：无输出、exit 0。
2. `ir_probe.ll` 的 `define void @compute(ptr noalias nofree noundef readonly align 4 captures(none) dereferenceable(4) %input, ptr noalias nofree noundef align 4 captures(none) dereferenceable(4) %output) unnamed_addr #0`。
3. 同 IR 的 `bb3` 里 `%1 = phi i32 [ %0, %bb2.bb3_crit_edge ], [ 2, %start ]`（常量 2 即 Nomicon 描述的折半）。
4. 带 `main` 的 `compute`（x=20、out=3）输出 `2`。
5. `invalid_reference_casting` 报错：`error: assigning to \`&T\` is undefined behavior, consider using an \`UnsafeCell\``，并带 `= note: \`#[deny(invalid_reference_casting)]\` on by default`。
6. 1.98 快照检索：`Stacked Borrows`/`Tree Borrows` 出现在 `unstable-book/compiler-flags/llvm-writable.html`、`unstable-book/print.html`、`src/alloc/sync.rs.html`、`src/core/slice/sort/unstable/quicksort.rs.html`，**Reference 里没有**。

**引文来源清单（逐字核对）**

本机快照 `…/1.98.0-x86_64-pc-windows-msvc/share/doc/rust/html/`：

- `reference/behavior-considered-undefined.html`：UB 列表的别名段（与在线版同一措辞，两处核对）。
- `cargo/commands/cargo-miri.html`："This command is only available on the nightly channel."
- `unstable-book/compiler-flags/codegen-emit-retag.html`："emit retag function calls in generated code" / "This feature has no tracking issue, and is therefore likely internal to the compiler, not being intended for general use."
- `unstable-book/compiler-flags/llvm-writable.html`："The Miri tool can be used to detect some problematic cases. However, note that when using Tree Borrows, you must set -Zmiri-tree-borrows-implicit-writes to ensure that the UB arising from these implicit writes is detected."（另有 `#[rustc_no_writable]` 字样）
- `nomicon/aliasing.html`：`compute` 例子与优化动机。
- `src/alloc/sync.rs.html`（Arc）："we don't shrink the range of bytes the ptr is allowed to access under Stacked Borrows"、"（Note that `NonNull::from(&STATIC_INNER_SLICE.inner)` is fine under Tree Borrows.）"。
- `src/core/slice/sort/unstable/quicksort.rs.html`："This is only an issue under Stacked Borrows, Tree Borrows accepts the intuitive code using GapGuard as valid."

在线官方源（写书时抓取）：

- UCG 首页：定位三段（"a past effort"、"It is largely abandoned right now. However, the glossary is actively maintained."、"mostly a "recommendation" and still subject to change"）。
- UCG 词汇表：`AbstractByte<Provenance>` 定义、Pointer Provenance 段（含 "The exact form of provenance in Rust is unclear."）、Aliasing 段（"The most developed potential aliasing model so far is Stacked Borrows."）。
- UCG 仓库 README：定位与 "Current consensus is documented in t-opsem FCPs and the Rust Language Reference."；`wip/stacked-borrows.md` 头部（"not normative nor endorsed by the UCG WG"）、读写访问规则段、Biggest conceptual issues 段（raw cast 派生新 tag、读不守栈纪律、`copy_nonoverlapping` 例子）；`wip/tree-borrows.md` 头部、"Differences"（raw retag 是 NOP、无 subobject provenance）。
- Miri README（master，提交 `446c2bac99b7`，2026-10-18）：SB/TB 描述、"even more experimental" 警告、`-Zmiri-disable-stacked-borrows`、`-Zmiri-strict-provenance`、`-Zmiri-permissive-provenance`、`-Zmiri-tree-borrows`、`-Zmiri-track-pointer-tag`、"Miri does **not catch every violation of the Rust specification** … not least because there is no such specification."
- Miri 源码：`src/borrow_tracker/mod.rs`（默认 Stacked Borrows、两种 AllocState）；`tree_borrows/perms.rs`（权限状态表：Cell / ReservedFrz / ReservedIM / Unique / Frozen / Disabled）；测试 `tests/pass/tree_borrows/sb_fails.rs`（"These tests fail Stacked Borrows, but pass Tree Borrows."）、`tests/pass/tree_borrows/reborrow-is-read.rs`（"a reborrow is counted as a Read access"）、`tests/fail/stacked_borrows/pass_invalid_mut.stderr`（正文引用的报错，**非本机运行**，已就地标注）。
- PLDI'25 Tree Borrows 论文页（plf.inf.ethz.ch）：摘要 "rejects 54% fewer test cases"、"Distinguished Paper Award at PLDI'25"。
- `rust-lang/opsem-team` 的 `fcps.md`：FCP 清单头部说明 + 清单中无别名模型条目。
- UCG issue 编号逐条核：**#584**（标题点名 LLVM 的 `writable` 属性，open）；**#585**（别名模型与并发模型如何交互，open）。

**遗留不确定（交给协调者）**

- SVG 只做了静态自查（单块无空行、无 `height`、无脚本、实体转义），**渲染目视留给收尾 build**。
- Miri 相关结论无量测：本机无 nightly。凡涉 Miri 输出处均标注来源与提交号；`-Zmiri-*` 命令块在 WARNING 盒里明确「未跑过」。
- UCG 的 #584 标题含反引号，正文按 §7 规避 `\`` 转义，改为叙述 + 编号（标题原文记在本票）。
- 「报错现场」第一段的文件名为本机探针名 `cast_lint.rs`（真实输出逐字），读者换文件名后路径行会不同。

**新增术语（待并表；GLOSSARY 现无）**

重借（reborrow）、借用栈（borrow stack）、许可树 / 权限树（permission tree）、保护符（protector）、tag（标记，行文保留英文）、retag（重打标记）、子对象来历（subobject provenance）、t-opsem / FCP（最终评论期）、阴影状态（shadow state）。

**衔接提示**

- 回指：unsafe 五项能力 / Miri 安装与 CI → 第 23 章；`addr`/`with_addr`/`expose_provenance`/`&raw`/`MaybeUninit` → 第 24 章；布局与 `repr` → 第 25 章。
- 本章口径（UCG 非规范、模型只活在 Miri、stable 不启用任何模型）可供后续章节引用。

## 审校修复（RvCh35 回执，1 blocker + 3 major + 4 minor）

**F1 · blocker · 已确认并全章修正。** 以 Miri `src/borrow_tracker/tree_borrows/perms.rs`（master，提交 `446c2bac99b7`）的转移表为准：`foreign_read` 里 `ReservedFrz{..}|ReservedIM`（未受保护）返回原值，注释逐字 "Before activation and without protectors, foreign reads are fine. That's the entire point of 2-phase borrows."；`Unique` 未受保护 → `Frozen`，受保护 → `Disabled`；`foreign_write` 只放过 `Cell` 与未受保护的 `ReservedIM`。改动位置：章首摘要、TB 权限表（改成「外来读 / 外来写」两列 + 子访问一句）、两条关键差别、`reborrow-is-read` 那条、正文「同一段代码，两个结论」的解释、两张 SVG（SVG2 右半重画为「Reserved 无影响 / Unique → Frozen」）、小结、练习 1 答案。

**F2 · major · 已补。** 本机 1.98 实跑 `transmute::<&usize, &mut usize>`：`error: transmuting &T to &mut T is undefined behavior, even if the reference is unused, consider instead using an UnsafeCell` + `= note: #[deny(mutable_transmutes)] on by default`（按 `src/main.rs` 跑，报 `src/main.rs:3:34`）。已写进「形式化到什么程度」层一，并把「报错现场」扩成「rustc 自己就能拦下的两类」，两类各带完整真实输出。

**F3 · major · 已按实情改写。** `snippets/ch35/sb_fails.rs` 的 `main` 改为 `pass_invalid_mut` 在前（文件头与正文都注明「与上游顺序的唯一差别」），因此「SB 第一处报 UB 在 `pass_invalid_mut`」对本章片段成立；表格同时注明上游文件第一处是 `fnentry_invalidation`。

**F4 · major · 已消解不一致。** 片段补齐上游全部五处用例（`fnentry_invalidation`、`pass_invalid_mut`、`return_invalid_mut`、`static_memory_modification`、`interior_mut_reborrow`），对照表五行的用例名现在都能在片段里找到；表头注明「都取自 Miri `tests/pass/tree_borrows/sb_fails.rs`」。

**F5 · minor · 已改。** stderr 引文明确标「节选」并点名略去的两行（`note: some details are omitted …`、`error: aborting …`）；两处 lint 输出改为完整、不再删行。

**F6 · minor · 已改。** 报错文件名统一为 `src/main.rs`（用该路径实跑得到所引输出）；IR 探针命令按 ch09/ch13 口径去掉 `--out-dir`：`rustc --edition 2024 -O --emit=llvm-ir snippets/ch35/ir_probe.rs`，重跑确认 `ir_probe.ll` 第 7 行输出不变；`snippets/ch35/ir_probe.rs` 头注释同步。

**F8 · minor · 已改。** 练习 1 答案里「Imprecisions」的归属改正：该节列的是 TB **比 SB 更宽松**、有人主张收紧的几处（无子对象来历、`&mut` 初始不可写），不包括「读只冻结」。

**自查补的一处同类问题**：`reborrow-is-read` 那条原写「绝不会判死」过强，已改为「（受保护时才会直接判死）」，与 `perms.rs` 的 `Unique if protected => Disabled` 对齐。

**F7**（GLOSSARY 术语并表）与「text 块不被门禁校验」：记录在案，属协调者/批次侧。

**本次自验**：`mdbook test -c ch35-aliasing-models.md` → exit 0；`rustc --edition 2024 snippets/ch35/sb_fails.rs` → 无警告、运行 exit 0、无输出；IR 探针按新命令重跑，`define void @compute …` 仍为 `.ll` 第 7 行；正文 CJK 5495（回到 4000–5500）。探针目录 `.scratch/tmp-63b/` 已删。

**遗留**：两张 SVG 只做静态自查（单块无空行、无 `height`、无脚本、主题变量），渲染目视留收尾 build；SVG2 右半为本次重画。
