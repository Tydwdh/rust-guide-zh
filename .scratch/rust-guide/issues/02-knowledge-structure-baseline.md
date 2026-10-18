# 02 Rust 知识结构基线

Type: research
Status: resolved

## Question

以官方与权威资料为准，Rust 的知识结构基线是什么：进阶主题的全集、彼此依赖关系，以及 2024 edition / 当前 stable（1.98）下的现状？

产出：

1. **专题清单**：以 The Book、Rust Reference、Rustonomicon、std 文档、Rust by Example 为底，枚举进阶主题（trait 设计、泛型与 const 泛型、生命周期进阶、智能指针与内部可变性、错误处理与 API 设计、并发与内存序、async/Future/Pin、宏、unsafe/FFI、性能与剖析、cargo/workspace/工程实践……），每个注明：属于哪一层、依赖哪些前置主题。
2. **依赖序**：哪些主题必须在哪些之前（用于章节排序）。
3. **现状校验**：2024 edition 的要点；近几个 stable 版本（至 1.98）已稳定的相关特性；常见教程里已经过时/踩坑的说法清单。

**判据**：以官方一手资料为准（doc.rust-lang.org、blog.rust-lang.org、RFC/release notes）；条目附链接；版本号与日期写清楚。

**用途**：作为「05 内容版图与章节结构」的输入。

产出物：`.scratch/rust-guide/research/02-knowledge-structure.md`，结论要点写进本票 `## Answer`。

## Answer

1. **校验基线**：本机 rustup 自带**与 1.98 完全同步**的官方文档快照（`book/`、`reference/`、`nomicon/`、`std/`、`edition-guide/`、`cargo/`、`rustc/`、`releases.md`…），研究全程用它核对，避免在线 `stable` 随版本漂移。环境：rustc 1.98.0 (88d9e12ae 2026-08-18)、cargo 1.98.0。
2. **专题全集**：3 层 × 37 项 —— B 基础 6（只做衔接、不设章）/ I 进阶 24 / D 深水 7，每项标注硬前置与官方一手链接（findings §2）。
3. **依赖序可直接用于排序**：硬边为 I1(`trait` 设计)→I2(`dyn` 兼容性)/I3(`impl Trait`)/I4(GAT)；I11(async)→I12(`Pin`)；I13(unsafe/UB)→I14(指针)/I15(布局)/I16(FFI)。据此给出 6 个章节批次：约定章 → 类型与 trait 主干 → 并发/异步 → 系统层 → 工程实践 → 深水选读（§3）。
4. **官方书籍的空白 = 本书的差异化空间**：Book 1.98 已有 ch17 完整 async 章（含 `Pin`/`Unpin`/`Stream`）与 ch20 进阶章（unsafe+Miri、高级 trait、高级类型、宏），但**完全缺** variance/dropck/PhantomData、`dyn` 兼容性与 upcasting、RPIT concise capturing、const 泛型、内存序、FFI/ABI/布局、性能剖析、错误处理与 API 设计、no_std。
5. **edition 2024（1.85 稳定）要点**：14 个语言页 + 3 个 std 页 + 3 个 Cargo 页；会改变写作前提的是 RPIT 生命周期捕获、`unsafe extern`、`#[unsafe(no_mangle)]`、`unsafe_op_in_unsafe_fn` 默认 warn、`static_mut_refs` deny、let chains、never type fallback、prelude 加入 `Future`/`IntoFuture` —— 均在本机 1.98 上做 2021/2024 对照实测（§4.1）。
6. **至 1.98 的稳定里程碑**（按版本，均附来源）：1.65 GAT/`let-else`、1.68 `pin!`、1.70 `OnceLock`、1.75 async fn in trait + RPITIT、1.80 `LazyLock`、1.81 `#[expect]`/`core::error::Error`、1.82 `use<>`/`&raw`/unsafe extern、1.85 async 闭包、1.86 trait upcasting、1.88 let chains、1.89 const 实参 `_`、1.91 模式绑定 drop 顺序、1.94 `LazyCell/LazyLock::get`、1.95 `if let` guard、1.96 `assert_matches!`、1.97 v0 mangling 默认、1.98 `repr(transparent)` 收紧（§4.2）。
7. **仍未稳定（写作必须标 nightly）**：specialization、generic_const_exprs、gen blocks、`impl Trait` in assoc type、`never_type`、arbitrary_self_types、allocator_api、try_trait_v2、async iterator/dyn async、泛型 `Atomic<T>`、trait_alias 等（特性门探针在本机 1.98 逐个复核，§4.3）。
8. **清单规模**：已过时说法 **25 条** + 易踩坑 **17 条**，逐条附来源；其中 4 条是本机实测的反直觉坑：`LazyLock/LazyCell` 的 `force`/`get` 等因 `this:` 接收者**只能 UFCS 调用**、AFIT 的 future 不保证 `Send`（E0277，手写 `impl Future + Send` 可解）、`AtomicU32` 的泛型拼写 `Atomic<u32>` 仍不稳定、`assert_matches!` 1.96 才稳定且**不在 prelude**（§5）。

findings：`.scratch/rust-guide/research/02-knowledge-structure.md`
