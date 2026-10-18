# 01 中文 Rust 学习资源地图

Type: research
Status: resolved

## Question

面向「有编程经验、Rust 基础已过」的中文读者，现有中文 Rust 学习资源的版图是什么：谁覆盖了什么、深度到哪、在哪里薄弱或缺失、读者在哪里卡住？

产出三份事实：

1. **资源清单**：The Book 中文版、Rust 圣经、Rust By Example 中文、Rustlings、中文社区 wiki/术语表、有影响力的博客/专栏/课程等。逐项：定位、覆盖范围、深度、更新状态与时效。
2. **缺口与痛点**：进阶主题（借用检查器心智模型、生命周期、异步、宏、unsafe、性能、工程实践）在中文资源里普遍薄弱或缺失的点；中文读者常见卡点与抱怨。
3. **术语惯例**：中文社区既有的 Rust 中英对照表与通行译法（所有权/借用/生命周期/智能指针/trait/泛型……），以及不一致、有争议之处。

**判据**：每条结论附一手来源链接（官方文档、原仓库、原帖），标注信息时效；结论里区分「事实」与「推断」。

**用途**：作为「04 定位与差异化边界」的输入 —— 可差异化的空间在哪。

产出物：findings 文档归档到 `.scratch/rust-guide/research/01-zh-rust-resources.md`，结论要点写进本票 `## Answer`。

## Answer

（查证日期 2026-10-18；详见 `.scratch/rust-guide/research/01-zh-rust-resources.md`）

1. **[事实] 第一层已齐全但陈旧**：The Book / RBE / Rustlings / Nomicon 的中译都有，但 `rust-lang-cn/book-cn` 自述内容最后更新 **2022-02-06**、且**中文目录无异步章**（`src/ch17-00-async-await.md` 404）；RBE 中文自述最后更新 **2022-01-26**。rust-lang.org 中文站把这些都标为"**非官方翻译**"。
2. **[事实] The Book 中文有两版分叉**：跟进到 **2024 edition（含 ch17 异步）**的是 `KaiserY/trpl-zh-cn`(5647★)，也是官网链接的那版；被搜到的 rustwiki 版反而缺异步章。
3. **[事实] 进阶事实标准是 course.rs**（Rust 语言圣经，31038★，2026-08 仍在更新，170+ 章）；但**性能/内存布局/编译优化整段几乎全是 `todo`**，且协议为 **No License（禁改后分发）**。异步中文只有"旧版 async-book 译本 + Tokio tutorial 译本"，官方 async-book 自述仍在重写、much is missing。
4. **[事实] 读者卡点**：核心概念（借用/生命周期/指针类型差异、move/Copy/Clone）缺心智模型；course.rs 自述"90% 的人还是云里雾里"、"链表是地狱"；rustcc 帖吐槽"资料网页长得都一样"——缺的是进阶路径，不是入门材料。
5. **[事实] 术语**：术语表（CC0，2025-06 更新）作 trait=**特质**，但两版 The Book **保留英文 trait**，course.rs 用**特征**（三方不一致）；lifetime 术语表并列"生存时间/寿命/生命周期"而主流用"生命周期"；术语表自认 coherence/target triple/token tree 待议。翻译指引停在 2019。
6. **[推断→可操作] 写什么**：概念心智模型（为何如此设计）+ 异步现状（Pin/Stream/async trait）+ 工程实践与性能（填 course.rs 的 todo）+ 示例可编译。**不写什么**：不做 The Book 逐章对照/又译一版、不重讲入门与 Rustlings 级练习、不搬 course.rs 文本、不追 unsafe 全谱（Nomicon 中译已覆盖）。
