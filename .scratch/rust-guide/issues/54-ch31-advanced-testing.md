# 54 第 31 章 测试进阶

Type: task
Status: open

## Question

写第五篇第五章：**测试进阶**。

- **范围**：单元/集成/doctest 的分工与陷阱（`#[test]`/`#[cfg(test)]` 不被 rustdoc 编译——书里常见的假绿坑；`mdbook test` 绿 ≠ 测过）；`compile_fail`/`no_run`/`should_panic` 的语义边界；参数化（表驱动 helper 的写法）；属性测试（proptest，说明性外部依赖）；快照测试（insta，说明性）；`cargo nextest`（说明性）；Miri/loom 在 CI 里的位置（nightly、外部 crate，说明性；回指 23/24 章的 Miri 用法）。
- **素材**：`research/02` 的 I21 行（书 ch11、`rustdoc/write-documentation/documentation-tests.html`、`cargo/guide/tests.html`、`cargo/commands/cargo-miri.html`）；本机 1.98 实测（`cargo test` 各能力真跑）。
- **边界**：Miri 细节 → 第 23/24 章（回指）；loom 与并发 → 第 19/20 章（回指）；CI 模板 → 第 30 章（回指）；`#[expect]`/lint → 第 32 章。
- **篇幅**：4000–5500 汉字；八节模板；练习 2–3 题带答案；外部工具只作说明、`ignore`/文字块。
- **DoD**：§8 —— 独占 `src/ch31-advanced-testing.md` + `snippets/ch31/` + 本票；禁 `git`/禁 build；自验 `mdbook test -c ch31-advanced-testing.md` 绿；探针 `.scratch/tmp-54/`（含真跑 `cargo test` 的小 crate）；收尾 `claimed` + `## Comments`。

## Comments
