# 26 第 15 章 泛型的边界：const 泛型与 GAT

Type: task
Status: claimed

## Question

写第 15 章：**泛型的两个「边界外」工具** —— const 泛型与 GAT。

- **范围**：const 泛型（`const N: usize`，`[T; N]` 的泛型化；与「类型参数」的心智模型对照）；为什么 **const 泛型算术仍不稳定**（`generic_const_exprs` / `min_generic_const_args` 的现状按 1.98 写，nightly 标注）；GAT（`type Item<'a>` 语法、解决什么问题 —— 例如 `LendingIterator`/`Iterator` 的借用迭代；与关联类型基础（第 12 章）的衔接）；两者各自「什么时候值得用」的判断清单与替代方案。
- **素材**：`research/02`（I4/I5 + §4.3 不稳定清单）；reference 的 const-generics / associated-items 页；std 现有 GAT 用例（如 `std::iter::Iterator` 之外的 `LendingIterator` 讨论按官方 issue/RFC 现状写）。
- **边界**：关联类型基础 → 12；async 迭代 → 21/22；手写集合（GAT 实战）→ 34。
- 篇幅 4000–5500 汉字；八节模板；练习 2–3 题带答案；nightly-only 必须 `> [!WARNING]` 标注。
- DoD：按 `WRITING.md` §8 批次协议（自验 `mdbook test -c ch15-generics-limits.md`）。

## Comments

- 2026-10-18 初稿：文件 `src/ch15-generics-limits.md`（640 行 / 4507 汉字，八节模板齐全，练习 3 题带答案）+ 片段 `snippets/ch15/fixed_size.rs`（const 泛型：转置 + 点积）、`snippets/ch15/lending_lines.rs`（GAT：自己持有缓冲区的按行借用迭代器 + `for<'a>` 消费函数）。
- 自验：`mdbook test -c ch15-generics-limits.md` → **exit 0**（23 个 doctest：11 个可编译块全过、11 个 `compile_fail` 全按预期失败、1 个 `rust,ignore`）；片段按书中命令真跑：`cd snippets/ch15 && rustc --edition 2024 fixed_size.rs -o fixed_size.exe && .\fixed_size.exe` → `转置 = [[1, 4], [2, 5], [3, 6]]`、`点积 = 32`；`lending_lines.rs` 同法 → `[alpha]`/`[be]`/`[gamma]`/`最长一行 5 个字符`（输出已粘进正文）。探针跑完已删（`.scratch/tmp-26/`）。
- 实测输出清单（rustc/cargo 1.98.0、edition 2024、本机 Windows；探针编号为写作期临时文件，均已删除）：
  - const 泛型基础/默认值/`_` 推断/`char`·`bool` 参数 → 编译通过并运行；`struct Buf<const N: usize = 4>`、`enum`、`type Alias` 上默认值合法，**函数与 impl 上非法**（`defaults for generic parameters are not allowed here`）。
  - `fn bad<const N: usize>() -> [u8; N + 1]` → `error: generic parameters may not be used in const operations` + `help: const parameters may only be used as standalone arguments here, i.e. 'N'`（进正文）。
  - `#![feature(generic_const_exprs)]` → `error[E0554]: #![feature] may not be used on the stable release channel`。
  - `f::<1 + 2>()` → `error: expressions must be enclosed in braces to be used as const generic arguments`（进正文）。
  - `Foo<N>`（`type N = u32` 同名）→ `error[E0747]: type provided when a constant was expected` + 花括号 help（进正文）；**无同名类型时 `A<N>` 反而编译通过**（练习 1 答案的依据，实测）。
  - `_` 推不出来 → `error[E0284]: type annotations needed for '[u8; _]'`。
  - `impl<const N: usize> Unconstrained {}` → `error[E0207]`（note: `expressions using a const parameter must map each value to a distinct output value`）。进正文。
  - `bool` const 参数的穷尽性 → `error[E0277]: the trait bound 'Flag<B>: Enabled' is not satisfied`（进正文）。
  - `const { assert!(N > 0, "数组不能为空") }`：合法实例通过；`first::<0>([])` → `error[E0080]: evaluation panicked: 数组不能为空`（进正文）。
  - `trait LendingIterator { type Item<'a> where Self: 'a; ... }` → 稳定编译；**漏 `where Self: 'a`** → `error: missing required bound on 'Item'` + `help: add the required where clause`；impl 漏抄 → `error[E0477]` + `help: copy the 'where' clause predicates from the trait`（均进正文）；`dyn LendingIterator` → `error[E0038]: the trait 'LendingIterator' is not dyn compatible ... because it contains generic associated type 'Item'`（进正文）。
  - `impl Iterator for Lines { type Item = &str; }` → `error: associated type 'Iterator::Item' is declared without lifetime parameters…` + 官方 note「另造一个借用数据源的类型」（进正文，逐字）。
  - 切片窗口若只借外部切片：用普通 `Iterator` + `mem::replace`（std `ChunksMut` 的路线）**可以编译**；不 `mem::replace` 直接 `self.slice.split_at_mut` → `error: lifetime may not live long enough … method was supposed to return data with lifetime 's but it is returning data with lifetime '1`（这条促成下面的取舍）。
  - `transpose<const R, const C>` + `std::array::from_fn` → 通过，输出 `[[1, 4], [2, 5], [3, 6]]`。
- 一手来源逐条核对（未凭记忆）：reference `items/generics.html`（const 参数允许类型清单原文、standalone 限制原文、实参语法与 `_` 推断、类型/const 歧义规则）、`types/array.html`（`The size is a constant expression that evaluates to a usize.`）、`items/associated-items.html`（GAT 声明形式、必需的 `where` 子句及理由原文）、`items/traits.html`（dyn 兼容：`It must not have any associated types with generics.`）、Unstable Book（`generic_const_exprs` #76560、`min_generic_const_args` #132980 描述原文、`adt_const_params` #95174）、`releases.html`（1.51「Only values of primitive integers, bool, or char types are currently permitted.」、1.65 GAT、1.79 内联 const、1.89 `generic_arg_infer`）、std 文档（`std::ops::AsyncFnMut::CallRefFuture<'a>` 带 `where Self: 'a`，trait 标 1.85.0）、RFC 2996 原文（lending async iterators 段）、std 源码（`core::str::pattern::Pattern::Searcher<'a>`：整个模块 `#![unstable]`；`core::slice::iter::ChunksMut: Item = &'a mut [T]`）。
- 遗留/未做：①无「语言对照角」——原打算比 C++ 非类型模板参数，但 cppreference 抓取 403、离线快照无 C++ 标准，按 §5「拿不准写『大意是』或干脆不写」整节省略；②`AsyncFnMut` 的签名块是 std 文档摘录，标 `rust,ignore` 未编译验证（这些 trait 由编译器实现，stable 上写不出可靠用例）；③只实测了 `N > 0` 这类单参数断言，未跑跨参数比较（`assert!(N < M)`）；④未测 `generic_const_exprs` 在 nightly 上的真实行为（本机无 nightly，只按 Unstable Book 与 E0554 描述现状）。
- 新术语（GLOSSARY 缺，建议由协调者并表）：`associated type | 关联类型`；`const generics（const generic parameters）| const 泛型`；`generic associated type（GAT）| 泛型关联类型（简称 GAT）`；`lending iterator | 借用式迭代器`；`inline const | 内联 const`；`dyn compatibility | dyn 兼容性`（第 13 章会用）。已核对在表内、本章沿用：`泛型（generics）`、`单态化（monomorphization）`、`零成本抽象`、`关联函数`、`DST`。
- 邻章衔接：第 12 章（关联类型基础，已写「基础见第 12 章」）；第 13 章（dyn 兼容性的机制，本章只给 reference 规则 + 实测 E0038）；第 14 章（RPIT/`use<>` 与本章 `for<'a> I::Item<'a>` 是同类「约束里怎么写生命周期」问题，本章只给两种写法）；第 16 章（HRTB/variance 只到「第 16 章展开」）；第 21、22 章（async 迭代 / Pin，正文已标「第 21、22 章展开」）；第 34 章（手写集合的 GAT 实战）。
- 图示需求（本轮按批次用 `text`/表格，未画 SVG）：**想要一张 SVG**放在「### GAT 的语法与 `where Self: 'a`」——把 `next(&'a1 mut self) -> Option<Item<'a1>>`、`'a2`、`'a3` 三次调用画成**时间轴/借用区间图**，标出「`Item<'a1>` 在下一次借 `self` 之前失效」；现有三行 `text` 块只能并列列出三次调用，画不出区间与失效关系。可选的第二张（优先级低）：`Buf<const N>` 实例化成 `Buf<4>`/`Buf<8>` 两个类型 → 各自单态化产物的示意（现用表格覆盖）。另：正文的 const 泛型算术错误、`where Self: 'a` 报错都是纯文本块，无需配图。
- 主动取舍：原计划用「切片窗口（WindowsMut，产出 `&mut [T]`）」当 GAT 的主例，实测发现它用普通 `Iterator` + `mem::replace` 就能写（std `ChunksMut` 的正统路线），于是把 GAT 主例换成**自己持有数据**的 `Lines`（`Item<'a> = &'a str`），并把「先考虑另造类型 / 借外部数据」写进判断清单与练习 3——这样 GAT 的必要性才是真的，而不是被误当成「凡是借用都得上 GAT」。
