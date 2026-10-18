# 25 第 14 章 impl Trait 与 opaque type

Type: task
Status: resolved

## Question

写第 14 章：**`impl Trait` 与不透明类型** —— 参数位置 vs 返回位置、2024 的捕获规则。

- **范围**：参数位置 `impl Trait`（= 泛型语法糖，与 `<T: Trait>` 的异同）；返回位置 RPIT（opaque type：调用者只见 trait、不见具体类型；不能同一函数返回两种具体类型 → `Box<dyn>` 或 enum 的取舍）；**2024 的捕获规则**（`impl Trait` 隐式捕获作用域内全部泛型参数与生命周期，`use<..>` 精确控制）；RPITIT（trait 里的 RPIT）与 AFIT（`async fn in trait`），以及 **AFIT 的 future 不保证 `Send`** 这个头号坑；类型泄露与「不透明」的边界。
- **素材**：`research/02`（I3 + §4.1 edition 2024 条目 + §5.2#12）；`reference` 的 impl-trait / 生命周期捕获页；edition-guide 的 `rust-2024/` 相关页。
- **边界**：`dyn` → 13；GAT → 15；async 运行时 → 21。
- 篇幅 4000–5500 汉字；八节模板；练习 2–3 题带答案；nightly-only 只标注；可选「语言对照角」。
- DoD：按 `WRITING.md` §8 批次协议（自验 `mdbook test -c ch14-impl-trait-opaque.md`）。

## Comments

- 2026-10-18 初稿：文件 `src/ch14-impl-trait-opaque.md`（4398 汉字；八节模板 + 可选「语言对照角」；31 个被测试块 = 20 `rust` + 11 `rust,compile_fail`，另有 23 个 `text` 输出块）；片段 `snippets/ch14/pipeline.rs`（RPIT 迭代器链，实测 5 行输出）与 `snippets/ch14/afit_send.rs`（AFIT + `Send` 编译期断言，实测 1 行输出）。
- 自验：`mdbook test -c ch14-impl-trait-opaque.md` → exit 0，`31 passed; 0 ignored`。为证明门禁真的在编译（成功时 mdbook 静默），临时追加一个必失败块重跑得 `test result: FAILED. 31 passed; 1 failed; 0 ignored`，随后已还原并复跑 exit 0。两个 snippet 另用 `rustc --edition 2024` 真跑取输出（`rustc --version` = 1.98.0）。
- 实测输出清单（探针曾置于 `.scratch/tmp-25/`，已删除）：
  1. 2024 捕获：`fn capture<'a>(_: &'a ()) -> impl Sized {}` 被 `fn test<'a>(..) -> impl Sized + 'static` 调用 → 2024 报 `error: lifetime may not live long enough`（含 `'a must outlive 'static` 与两条 help），2021 通过；`capture` 改成 `+ use<>` 后 2024 通过。
  2. 本章开头的 `fn evens(xs: &[u32]) -> impl Iterator<Item = u32>`：edition 2024 通过；edition 2021 报 `error[E0700]: hidden type for 'impl Iterator<Item = u32>' captures lifetime that does not appear in bounds`（note 里打印出隐藏类型 `Filter<Copied<std::slice::Iter<'_, u32>>, {closure@...}>`）。2024 与 2021 的对照是本章核心实测。
  3. `use<'a, T>` 编译通过，`type_name_of_val` 打印 `(&(), u32)`；`use<'_>` 可用。
  4. `use<..>` 与 APIT 同签名 → `` error: `impl Trait` must mention all type parameters in scope in `use<...>` ``；trait 内 `use<'a, T>` 缺 `Self` → `` error: `impl Trait` must mention the `Self` type of the trait in `use<...>` ``，补成 `use<'a, T, Self>` + `where T: 'a` 后通过。
  5. `impl-trait-overcaptures`：默认 allow（实测 `rustc -W help` 里属 `rust-2024-compatibility` 组），`-W impl_trait_overcaptures` 后在 **edition 2021** 下报 `` warning: `impl Sized` will capture more lifetimes than possibly intended in edition 2024 `` 并建议 `+ use<>`；2024 下不再触发。
  6. RPIT 两分支类型不同 → `error[E0308]: 'if' and 'else' have incompatible types`，编译器附带 `Box<dyn std::fmt::Display>` 建议；改成 `Box<dyn Display>` 后实测输出 `1 hello`；两分支都给 `std::iter::once(..)`（同一具体类型）时通过，输出 `1`。
  7. 同一函数两次调用返回同一不透明类型，`a == b` 输出 `true`；不同泛型实参 `generic::<u8>()` vs `generic::<i8>()` → E0308 + note `distinct uses of 'impl Trait' result in different opaque types`。
  8. 返回借用局部 `Vec` 的迭代器 → `error[E0597]: 'a' does not live long enough ... opaque type requires that 'a' is borrowed for 'static`；改成「数据从参数借入 + `use<'a>`」后输出 `[1, 2]`。
  9. APIT + turbofish → E0107 + note `` `impl Trait` cannot be explicitly specified as a generic argument ``；`let x: impl Debug` 与结构体字段 → `error[E0562]`（note：only allowed in arguments and return types of functions and methods）。`fn show_all(v: Vec<impl Debug>)` 合法，实测输出 `1 2 3`。
  10. AFIT 的 `Send` 坑：`trait Job { async fn run(&self) -> u32; }` + `fn needs_send<F: Send>` → `error[E0277]: 'impl Future<Output = u32>' cannot be sent between threads safely`，标头 help 原文为「`Send` can be made part of the associated future's guarantees for all implementations of `Job::run`」；改成 `fn run(&self) -> impl Future<Output = u32> + Send;` 后同一代码编译通过，且**不需要** `+ '_`。`&dyn Job` → `error[E0038] ... is not dyn compatible`（note 指向 reference/items/traits.html#dyn-compatibility）；公开 trait 的 `async fn` → `warning: use of 'async fn' in public traits is discouraged as auto trait bounds cannot be specified`（`#[warn(async_fn_in_trait)]` on by default）。
  11. auto trait 透传：`fn counter() -> impl Iterator<Item = u32>` 的返回值可直接交给 `fn needs_send<T: Send>`，实测输出 `0 2 4`；`type_name_of_val` 打印隐藏类型 `core::iter::adapters::map::Map<core::ops::range::Range<u32>, src/main.rs::values::{{closure}}>`。
  12. `impl Trait` 放关联类型位置 → `error[E0658]: 'impl Trait' in associated types is unstable`（正文用 `> [!WARNING]` 标注为不稳定）。
- 一手来源核对：Reference `types/impl-trait.html`（APIT 语法糖原文、函数选返回类型原文、同一隐藏类型原文、`use<..>` 约束、RPITIT desugaring 原文）、Reference `items/traits.html#dyn-compatibility`（「Not have an opaque return type」）、Edition Guide `rust-2024/rpit-lifetime-capture.html`（全 edition 捕获类型/const 参数原文、`Captures` 戏法可被 `use<..>` 取代原文、迁移 lint）、std `any::type_name` 文档（诊断用途原文）、Swift 官方书 `OpaqueTypes.md`（「reverse of a generic type」原文、opaque 保留 type identity vs boxed protocol type）。所有英文引文逐字比对过（含原文的排印撇号 `doesn’t`）。
- 遗留不确定：
  1. `impl_trait_overcaptures` 的自动修复行为（`cargo fix --edition` 会插 `use<>`，APIT 场合需手动命名类型参数）只按 Edition Guide 陈述，本机没跑 `cargo fix`（迁移流程属第 33 章）。
  2. 「trait 内 `use<..>` 自 1.87 起可用」依据 `.scratch/rust-guide/research/02` §4.2（releases.md 1.87 `precise_capturing_in_traits`）；本机 1.98 实测可用，无法回测 1.87。
  3. 未展开 `-> impl Trait` 的 variance/不变性（第 16 章）与 `impl Future` 的 drop/取消语义（第 21 章）。
- 新术语（建议协调者并入 GLOSSARY）：`hidden type | 隐藏类型`（不透明类型背后的具体类型）；`capture | 捕获`、`precise capturing | 精确捕获`（`use<..>`）；`APIT | 参数位置的 impl Trait`（对照已有 RPIT 条目）；`RPITIT`、`AFIT`（保留缩写，首现给全称）；`auto trait | 自动 trait`（`Send`/`Sync` 这类编译器自动推导的 trait；建议保留英文并首现解释）。
- 邻章衔接：正文指向第 13 章（dyn 兼容性/对象安全）、第 10 章（迭代器与闭包）、第 12 章（关联类型）、第 15 章（GAT）、第 21 章（执行器/运行时），均写成「（第 N 章…）」。建议：第 13 章讲 `dyn` 取舍时交叉引用本章的 RPIT/`Box<dyn>`/enum 三表；第 21 章讲 `!Send` future 时直接引用本章「AFIT 的 `Send` 坑」；第 33 章迁移章可引用本章 `use<..>` 与 overcaptures lint。
- 图示需求（本轮按规范只用 text/表格，SVG 走票 22）：
  1. 「同一处语法、两处相反的角色」：APIT（调用者选类型 → 函数）与 RPIT（函数选类型 → 调用者）的左右方向对照图。
  2. 捕获集合示意：`fn evens(xs: &'a [u32]) -> impl Iterator<Item = u32>` 的作用域内参数与 `use<..>` 筛选结果，2021 vs 2024 两列勾/叉对比（可附「多捕 → 调用者约束紧」的箭头）。
  3. RPIT / `Box<dyn>` / enum 三条路的分派方式与分配代价对照图（表格已能说清，属可选）。

- 2026-10-18 审校修回（RvCh14：2 blocker + 4 major + 8 minor；协调者转达）：
  - **blocker 1**（表「调用者能要求什么」）：`-> Box<dyn Trait>` 一格改为「只有签名里显式写出来的那些（如 `dyn Trait + Send`）」，表下补 auto trait 的定义（此处理论首现）与两侧对照，引实测关键行 ``error[E0277]: `dyn Foo` cannot be sent between threads safely`` 与 ``= help: the trait `Send` is not implemented for `dyn Foo` ``。新探针：`Box<dyn Foo>` 交给 `fn needs_send<T: Send>` 编译失败，`Box<dyn Foo + Send>` 通过（输出 `ok`）。
  - **blocker 2**（AFIT 小节结尾 + 练习 3 修法二）：`Box<dyn Future<…> + Send + '_>` 改为 `Pin<Box<dyn Future<…> + Send + '_>>`；删掉「不动 trait」「不必让实现者承担 `Send` 义务」两句，改为「把 future 装箱进 `Pin<Box<…>>`；代价是每次调用一次堆分配」；补一行说明：裸 `Box<dyn Future>` 因 `!Unpin` 连 `.await` 都用不了（实测 ``error[E0277]: `dyn Future<Output = Vec<u8>> + Send` cannot be unpinned`` 与 ``= note: required for `Box<dyn Future<Output = Vec<u8>> + Send>` to implement `Future``）。**真跑验证**：`.scratch/tmp-25b/src/main.rs`（`Pin<Box<dyn Future>>` 版 trait + 一个 `Box::pin` + `Waker::noop()` + `Context::from_waker` 的极简 `block_on`）驱动 `async fn run_it(l) { l.load().await }` → `rustc --edition 2024` rc=0，运行输出 `[1, 2, 3]`；章内练习答案的代码块也随 `mdbook test` 编译通过。
  - **major 3**：`type_name_of_val` 的输出改为真实前缀 `main::values::{{closure}}`（`rustc --crate-name main` 实测），并加「前缀是 crate 名，随 crate 名与模块路径变化」。
  - **major 4**（报错现场 §6）：按真实输出重抄：`--> src/main.rs:8:27`、`fn generic` 在第 1 行、`the expected/found opaque type` 两行、caret 落在 `d` 下、两行 `= note:`。
  - **major 5**：编译器 help 逐字改回 `2 +     fn run(&self) -> impl std::future::Future<Output = u32> + Send;`，并注明「编译器写全路径，2024 prelude 已有 `Future`，本书示例用简称」，同时补回被跳过的 `| required by a bound introduced by this call` 与 `note: required by a bound in needs_send` 段。
  - **major 6**：GLOSSARY 的 `_Avoid_: 隐藏类型` 由协调者修；正文用词不动，仍在首现处补了「隐藏类型（hidden type，即不透明类型背后那个具体类型）」。
  - **minor 7**：Repo 的 `use<..>` 摘录补上真实的 E0309（含 span、`T: 'a` 的 help）作为第一条错误，`T: 'a` 的解释随之前移。
  - **minor 8**：lint 改为规范名 `impl_trait_overcaptures`（注明 CLI 也接受连字符），并补一个可编译的 `fn f<'a>(x: &'a ()) -> impl Sized { *x }` 代码块 + 真实 edition 2021 输出（含 `--> src/main.rs:1:6`、`^^`、`= note: requested on the command line with `-W impl-trait-overcaptures``）。
  - **minor 9**：「生命周期参数要写在类型与 const 参数之前」。
  - **minor 10**：关联类型位置的 `> [!WARNING]` 改为「以最小复现 `type Out = impl Iterator<Item = u32>;` 为例」，并说明本机最小复现只报 1 条 E0658 + issue #63063 note（不同写法会连带出 `E0562` 等）。
  - **minor 11**：练习 2 答案改用真实诊断类型：`std::iter::Once<u32>` 与 `Map<std::ops::Range<{integer}>, {closure@src/main.rs:5:20: 5:23}>`（并说明 `{integer}` 的来历）。
  - **minor 12**：所有诊断摘录重抄为**真实输出的连续片段**（E0107 补 `note: function defined here` 段、pick 补 `expected because of this`、E0038 补 `-->` 与 trait 上下文两行、E0597 补 `6/7/8/9` 行与 `- `a` dropped here…` 收尾、E0562 补 issue #63065 note、overcaptures 补 `-->`+`^^` 与 CLI note）；一处无法完整展示的（`capture` 的第二条 help、`b` 的第二条 E0597）在块外用「（…略。）」注明。复核方式：脚本把章内 12 个诊断块的每一行与真实 rustc 输出逐行比对（全部逐字命中），并验证每块都是真实输出的连续片段（允许 `...` 省略），**0 处跨位置拼接**；另校验 5 条行内引用的关键行也在真实输出中命中。
  - **minor 13**：「第 21 章的执行器正是这么要求的」。
  - **minor 14**：新术语首现处补齐：隐藏类型（hidden type）、精确捕获（precise capturing，也就是 `use<..>`）、RPITIT 全称 `return-position impl Trait in trait`、AFIT 展开 `` `async fn in trait` ``、auto trait 的一句定义。
  - 复跑结果：`mdbook test -c ch14-impl-trait-opaque.md` → exit 0；注入必失败块得 `test result: FAILED. 32 passed; 1 failed; 0 ignored`（随后还原并复跑 exit 0），当前章内共 32 个受测块（21 `rust` + 11 `rust,compile_fail`）+ 23 个 `text` 块，0 ignored；汉字 4398 → 4781（仍在 4000–5500）。

## Answer

2026-10-18 批次收尾（用户验收通过）。

- 交付：`src/ch14-impl-trait-opaque.md`（4781 汉字；八节模板 + 语言对照角 + 3 道练习）+ `snippets/ch14/`。
- 自验：`mdbook test -c ch14-impl-trait-opaque.md` → exit 0；注入必失败块反证 `32 passed; 1 failed; 0 ignored`（随即还原）→ 32 个受测块（21 `rust` + 11 `compile_fail`）、0 ignored。
- 交叉审校（RvCh14；**2 blocker** + 4 major + 8 minor）：①表内 `Box<dyn Trait>` 误写「透传 auto trait」→ 改正并补实测对照；②练习修法二 `Box<dyn Future>` 无法 `.await` → 改 `Pin<Box<…>>`；③`main::values` 前缀、§6 行/列坐标、help 逐字、`impl_trait_overcaptures` 下划线、const 参数、`{integer}`、12 个摘录块连续片段化（脚本逐行比对，0 处跨位置拼接）等全修。
- 术语裁定（协调者）：GLOSSARY 删「不透明类型」的 `_Avoid_: 隐藏类型` 并新增「隐藏类型（hidden type）」条；正文首现括注已补。
- 全量链（批次级）：`mdbook build` 0 ERROR/0 警告｜`mdbook test` 20 章 exit 0｜examples 19+2 passed。
- 遗留：`cargo fix --edition` 插 `use<>` 依 Edition Guide 未本机跑（票内记录）。
