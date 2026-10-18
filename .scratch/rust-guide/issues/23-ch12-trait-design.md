# 23 第 12 章 trait 设计

Type: task
Status: claimed

## Question

写进阶篇第一章：**trait 作为设计工具**（第 9 章只建立了「行为契约」的直觉）。

- **范围**：关联类型 vs 泛型参数的取舍（何时用哪个，用 `Iterator`/`Add` 做例）；supertrait（`trait A: B`）；sealed trait 模式（`pub trait X: private::Sealed`）；扩展 trait（给外部类型加方法）；coherence 与孤儿规则的细节（第 9 章点到为止处展开）；对象安全只留一句预告（第 13 章）。
- **素材**：`research/02-knowledge-structure.md`（I1 条目与 §5 易踩坑）；官方快照 `reference/items/traits.html`、`reference/items/associated-items.html`、std 的 `Iterator`/`Add` 页。
- **边界**：`dyn`/对象安全 → 13；`impl Trait`/RPIT → 14；GAT → 15；错误处理与 API 设计 → 18。
- 篇幅 4000–5500 汉字；八节模板；练习 2–3 题带答案；nightly-only（如 specialization）只标注不进正文；可选「语言对照角」。
- DoD：按 `WRITING.md` §8 批次协议（独占本章文件 + `snippets/ch12/` + 本票；禁 `git`/禁 build；自验 `mdbook test -c ch12-trait-design.md`；收尾 `claimed` + `## Comments`）。

## Comments

- 2026-10-18 初稿：文件 `src/ch12-trait-design.md`（八节模板齐全，4717 汉字）；片段 `snippets/ch12/assoc_vs_generic.rs`、`supertrait.rs`、`sealed.rs`、`extension_trait.rs`（4 个完整程序，经 `{{#rustdoc_include}}` 进 `mdbook test`，正文各配真实运行输出）。除授权范围外未新增/修改任何文件（未碰 `SUMMARY.md`、`book.toml`）。
- 自验：`mdbook test -c ch12-trait-design.md` → **exit 0**（16 个 doctest：11 passed / 5 compile_fail ok / 4 ignored）。另做门禁反证：临时注入一个坏 `rust` 块后同一命令 exit 101、报 E0308，随即移除（当前为绿）。按 §8 未跑 `mdbook build`/`serve`。
- 实测输出（探针全在 `.scratch/tmp-23/`，收尾已删；`rustc`/`cargo` 均为本机 1.98.0、edition 2024）：
  1. 4 个片段程序的真跑输出，已粘进「动手看看」（`Meters(3.5)/Meters(6.5)/1.50 m/3/1.50 m/1.50 m/1.5`、`area = 3.1416, radius = 1.0000`、`hello`、`Some(20)/固有 len: 3/扩展 len: 999`）。
  2. 报错现场三例的关键行均来自实跑：E0210（`impl<T> Display for T`）、E0119（blanket + 具体类型）、sealed 的 E0277 及其专属 note `"sealed trait"`（单 crate 版 + 跨 crate 版，末尾多一条 `= help: the following type implements the trait: greet::English`）。
  3. 就地报错：E0116（`impl Vec<i32>`）附 `= help: ... newtype wrapper`。
  4. `#[diagnostic::on_unimplemented]` 定制文案实测通过（1.78 稳定；行号 18:25 与正文清单一致）。
  5. 基础类型构造器：`impl Display for Box<Local>`、`impl Display for &Local` 都编译通过；`impl<T> Display for Box<T>` 报 E0210；`type Alias = Vec<i32>` + `impl Display for Alias` 报 E0117（别名不改变本地性）。
  6. 跨 crate（cargo workspace `greet`/`user`）：sealed 报错与单 crate 版同形，路径变 `greet::private::Sealed`。
  7. 跨 crate blanket impl 封死下游：库 `impl<T: Debug> Describe for T` + 下游 `impl Describe for Point` → E0119，note 原文 `conflicting implementation in crate \`blanket\`: - impl<T> Describe for T where T: Debug;`（正文已按节选粘成 `text`）。
  8. 英文引用逐字核对（脚本比对本机快照 HTML / std 源码）：Reference traits、implementations、glossary、Book ch20-02、std 源码 `mod sealed` 注释、std `Add` 文档段——全部命中，含 Book 的 `U+2019` 撇号。
- 遗留：
  - 「语言对照角」**未写**（票里为可选）：跨语言断言必须逐条核对对应语言一手文档，本轮预算优先给 coherence/sealed 的实测证据；若协调者要求补，最省力的对照点是「给已有类型补方法」——Java 只能在声明处 `implements`（要补方法得包适配器）、C++ 自由函数 + ADL、Python 运行时打补丁，正好对应扩展 trait 的存在理由。
  - `Pin` 属于基础类型构造器只按 Reference 术语表转述，未做 `impl Display for Pin<Local>` 的编译验证（`Pin<P>` 的接收者/解引用限制让最小复现要多绕一步）。
  - `unsafe_impl` 无；`const trait`（1.98 std 源码里 `pub const trait Add`）只在 std 快照里见到，判为内部不稳定语法，未进正文。
- 新术语（GLOSSARY 未收，待协调者并表）：
  - associated type | 关联类型
  - associated item | 关联项
  - associated constant | 关联常量（仅出现在延伸阅读小标题）
  - extension trait | 扩展 trait
  - sealed trait | 密封 trait（正文动词用「密封（seal）」）
  - coherence | 一致性
  - inherent method | 固有方法
  - blanket impl | 建议保留英文
  - cover / uncovered | 覆盖 / 未覆盖（孤儿规则语境；注意 GLOSSARY 在「遮蔽」条目下把「覆盖」标为 trait 实现的说法，请裁决是否给「覆盖」加语境限定）
  - fundamental type constructor | 基础类型构造器
  - dyn compatibility | dyn 兼容性（Reference 已把 object safety 改名；GLOSSARY 无 object safety 条目，第 13 章会用到）
- 邻章衔接：第 9 章孤儿规则处写的「第 12 章展开」已兑现（coherence 细节 + newtype 作为「覆盖类型参数」的位置），E0117 指向「第 9 章报错现场」；`dyn`/对象安全只留 NOTE + 一句「upcasting 自 1.86 稳定」→ 第 13 章；`impl Trait`/RPIT → 第 14 章（未展开）；GAT/const 泛型 → 第 15 章（未展开）；API/SemVer → 第 18 章（只提 SemVer 一词）；`Deref` 让 newtype「像内部类型」→ 第 17 章（正文留了一句）。
- 图示需求（本轮未自画 SVG）：
  - `### coherence 与孤儿规则的细节`：现在是一张四行表；若出图，想要 2×2 方格（trait 本地/外部 × 类型本地/外部），四格分别标「允许 / 允许 / E0117 / E0210」，并在角落标出「本地类型由 newtype 提供」。
  - `### sealed trait`：现在是 `text` 树；若出 SVG，建议画模块可见性的三层——外部可命名（`pub trait Greet`）、外部不可命名（`mod private` → `trait Sealed`）、可实现区与不可实现区之间的那道墙。
  - `### 关联类型还是泛型参数`：现在是三行表；可选一张「同一 trait 两种写法 → 一个类型能填几次表」的对照图（左：`type Item` 只有一格；右：`<Rhs>` 带参数，可多份）。
