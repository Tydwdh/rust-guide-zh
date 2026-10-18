# 33 第 22 章 Pin 与自引用

Type: task
Status: open

## Question

写第三篇第四章：**Pin 与自引用**（编排上必须在第 21 章「先有 Future」之后讲，已满足）。

- **范围**：自引用为什么是问题（值被移动后内部指针失效的最小构造）；`Pin<P>` 的契约（「不移动」的保证到底由谁承担）；`Unpin` 的含义与「几乎所有的类型都 Unpin」；`pin!` 与 `Box::pin` 的用法与差别；`PhantomPinned` 制造 `!Unpin`；「什么时候真的需要 `unsafe`」的边界清单（投影规则预告 → 第 38 章）。
- **素材**：`research/02-knowledge-structure.md` 的 I12 行（依赖 I11/I6；`std/pin/index.html`、`std/pin/struct.Pin.html`、书 ch17.5）；本机 1.98 快照核对 `Pin` 的方法签名与 `Unpin` 实现清单。
- **边界**：unsafe 四能力与 UB 清单 → 第 23 章；手写 pin 投影 / 自引用容器 → 第 38 章（第 34 章可回指）；`Pin` 在 async 里的角色在第 21 章末尾已引出（回指不重讲）。
- **篇幅**：4000–5500 汉字（不含代码块）；八节模板；练习 2–3 题带答案；所有示例在 1.98 + 2024 上真跑；nightly-only 只标注。
- **DoD**：同 §8 协议 —— 独占 `src/ch22-pin-self-reference.md` + `snippets/ch22/` + 本票；禁 `git`/禁 build；自验 `mdbook test -c ch22-pin-self-reference.md`；探针 `.scratch/tmp-33/`；收尾 `claimed` + `## Comments`（同 §8 清单）。

## Comments
