# 64 第 36 章 高级类型系统（nightly 标注）

Type: task
Status: open

## Question

写第六篇第三章：**高级类型系统**（素材 = D3：unstable-book `specialization`/`generic_const_exprs`/`adt_const_params`；coherence 细节）。

- **范围**：coherence 细节（孤儿规则精确版、fundamental 类型、覆盖判定）——stable 侧用 `compile_fail` 实测冲突；negative impls（`negative_impls`）、specialization（`specialization`/`min_specialization`）、`adt_const_params`/`generic_const_exprs`（均 nightly）——**全部 WARNING 标注 + 只给文档化示例**（本机无 nightly，不伪造输出）。
- **要求**：每个 unstable 特性给稳定面替代方案（若有）与追踪 issue 编号（逐条核官方）；`compile_fail` 错误码与错误类匹配。
- **边界**：基础 trait/泛型 → ch12/13/15 回指；nightly 工具链约定 → ch02 口径一句。
- **篇幅**：4000–5500 汉字；八节模板；练习 2–3 题带答案。
- **DoD**：§8 —— 独占 `src/ch36-advanced-type-system.md` + `snippets/ch36/` + 本票；禁 `git`/禁 build；自验 `mdbook test -c ch36-advanced-type-system.md` exit 0；探针 `.scratch/tmp-64/`；收尾 `Status: claimed` + `## Comments`。

## Comments
