# 32 第 21 章 async 基础：Future、执行器与运行时

Type: task
Status: open

## Question

写第三篇第三章：**async 基础**。

- **范围**：`Future` 的契约（惰性、`Poll`、谁负责推进）；执行器概念（`Waker` 一句 + 「为什么要被执行器管理」）；`async`/`.await` 展开（状态机直觉，具体展开 → 第 38 章）；`join!`/`select!` 的概念（外部 crate 只作说明性引用，不进被测块）；`!Send` future 与 `Send` 边界（跨 `.await` 持锁的最小复现）；取消与 drop（`select` 掉的分支去哪了）；运行时选型一段（取舍标准，不点名背书）。
- **素材**：`research/02-knowledge-structure.md` 的 I11 行（依赖 B1/I3/I7/I9；书 ch17、`std/future`、`std/task`）；本机 1.98 快照：`std/future/trait.Future.html`、`std/task`（`Waker`/`Context`）；`std/pin` 只回指第 22 章。
- **边界**：`Pin`/`Unpin` 细节 → 第 22 章；手写运行时 / `Waker` 深水 / pin 投影 → 第 38 章；AFIT 语法在第 14 章（回指）；`async fn in trait` 的 `Send` 坑在第 14 章（回指）。执行器演示可用 std-only 的极简 `block_on`（把「Waker 被用来叫醒」说清即可，完整运行时 → 第 38 章）。
- **篇幅**：4000–5500 汉字（不含代码块）；八节模板；练习 2–3 题带答案；`async`/`.await` 在 edition 2024 + 1.98 stable 上真跑；外部 crate 的代码块一律 `rust,ignore` 并注明原因。
- **DoD**：同 §8 协议 —— 独占 `src/ch21-async-basics.md` + `snippets/ch21/` + 本票；禁 `git`/禁 build；自验 `mdbook test -c ch21-async-basics.md`；探针 `.scratch/tmp-32/`；收尾 `claimed` + `## Comments`（同 §8 清单）。

## Comments
