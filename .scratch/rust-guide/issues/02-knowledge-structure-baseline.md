# 02 Rust 知识结构基线

Type: research
Status: open

## Question

以官方与权威资料为准，Rust 的知识结构基线是什么：进阶主题的全集、彼此依赖关系，以及 2024 edition / 当前 stable（1.98）下的现状？

产出：

1. **专题清单**：以 The Book、Rust Reference、Rustonomicon、std 文档、Rust by Example 为底，枚举进阶主题（trait 设计、泛型与 const 泛型、生命周期进阶、智能指针与内部可变性、错误处理与 API 设计、并发与内存序、async/Future/Pin、宏、unsafe/FFI、性能与剖析、cargo/workspace/工程实践……），每个注明：属于哪一层、依赖哪些前置主题。
2. **依赖序**：哪些主题必须在哪些之前（用于章节排序）。
3. **现状校验**：2024 edition 的要点；近几个 stable 版本（至 1.98）已稳定的相关特性；常见教程里已经过时/踩坑的说法清单。

**判据**：以官方一手资料为准（doc.rust-lang.org、blog.rust-lang.org、RFC/release notes）；条目附链接；版本号与日期写清楚。

**用途**：作为「05 内容版图与章节结构」的输入。

产出物：`.scratch/rust-guide/research/02-knowledge-structure.md`，结论要点写进本票 `## Answer`。
