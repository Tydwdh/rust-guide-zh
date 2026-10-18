# 45 项目 2：表达式求值器（小型库 crate）

Type: task
Status: open

## Question

写第二篇收尾项目：**表达式求值器**（`examples/expr-eval/` 已建 stub，crate 名 `expr-eval`；写作者替换 stub 为真实现）。

- **目标**：零依赖库 crate：词法（tokenizer）→ 语法分析（递归下降或 Pratt，自选并论证）→ AST → 求值；覆盖第二篇概念并让读者看见它们「为什么这么设计」：trait 与关联类型（操作符/节点设计）、泛型与 impl Trait 的取舍、生命周期（借用词法切片 vs 拥有 String）、智能指针（`Box<Expr>`，必要时讨论 Rc）、错误设计（自定义错误 enum + `Display`/`Error`/`From` 实例）、API 设计（公开面最小化、doc 注释、`#[must_use]`/`#[non_exhaustive]` 的取舍）。
- **章结构**：对齐项目 1 成稿（逐块实现 + 设计取舍 + 报错现场 + 小结 + 练习 2–3 题带答案）；正文用 `{{#include}}`/`{{#rustdoc_include}}` 引 crate 真实代码；每段引用的代码与测试都必须真实且全绿。
- **素材**：已定稿的 ch12–18（尤其 12/14/15/17/18）；算法自包含，无需外部资料。
- **边界**：std-only（零依赖）；不用 `unsafe`；含一个薄 CLI `src/bin/eval.rs`（能被 e2e 测试，参照 todo-cli 的写法）；异步/并发不涉。
- **篇幅**：4000–5500 汉字（不含代码块）。
- **DoD**：§8 —— 独占 `src/project2-expr-eval.md` + `examples/expr-eval/**`（含其 `Cargo.toml`）+ 本票；禁 `git`/禁 `mdbook build`；自验：`CARGO_TARGET_DIR=.scratch/tmp-45/target cargo test --manifest-path examples/Cargo.toml -p expr-eval` 全绿 + `mdbook test -c project2-expr-eval.md` 绿；探针 `.scratch/tmp-45/`（收尾删）；**勿动** `examples/Cargo.toml`（workspace 根，members 已配好）、`examples/README.md`、其他 crate、其他章/票；收尾 `Status: claimed` + `## Comments`（同 §8 清单 + crate 的测试计数）。

## Comments
