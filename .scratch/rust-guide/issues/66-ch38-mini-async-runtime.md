# 66 第 38 章 自己实现极简 async 运行时

Type: task
Status: open

## Question

写第六篇第五章：**自己实现极简 async 运行时**（素材 = D5：`std::task`/`std::future`/`std::pin` 文档）。

- **范围**：不用 async/await 语法糖，手写一个 `impl Future` 状态机；`RawWaker`/`Waker` 构造与 `Wake`；手写 pin 投影（`unsafe` + `Pin::map_unchecked_mut` 一类，封装成安全 API）；单线程 executor：任务队列 + `block_on` + 唤醒路径。与 ch21 明确划界：ch21 讲「怎么用 executor」，本章讲「怎么从 `Future`/`Waker` 原语搭出来」。
- **要求**：全部 stable 真跑（`snippets/ch38/` 含测试）；`Wake`/`RawWaker`/`Pin` 引文逐字核 1.98 快照。
- **边界**：async/await 语法与 executor 使用 → ch21 回指；Pin 语义 → ch22；不重讲 waker 基础概念。
- **篇幅**：4000–5500 汉字；八节模板；练习 2–3 题带答案；按 §9 内嵌 1–2 张 SVG（poll/唤醒时序、状态机展开）。
- **DoD**：§8 —— 独占 `src/ch38-mini-async-runtime.md` + `snippets/ch38/` + 本票；禁 `git`/禁 build；自验 `mdbook test -c ch38-mini-async-runtime.md` exit 0 + snippet `cargo test`（私有 target）全绿；探针 `.scratch/tmp-66/`；收尾 `Status: claimed` + `## Comments`。

## Comments
