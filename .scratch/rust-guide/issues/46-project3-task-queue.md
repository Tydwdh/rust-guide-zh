# 46 项目 3：迷你任务队列（并发与异步篇）

Type: task
Status: open

## Question

写第三篇收尾项目：**迷你任务队列**（`examples/task-queue/` 已建 stub，crate 名 `task-queue`；写作者替换 stub 为真实现）。

- **目标**：零依赖线程池式任务队列：`spawn` 提交闭包任务；N 个 worker 线程从 `mpsc` 取任务；结果经 `JoinHandle` 或 channel 收集；优雅关闭（drop 全部发送端 + join workers）；可选：`recv_timeout` 超时接收、panic 任务不拖垮池（`catch_unwind`）。覆盖第三篇概念并让读者看见设计取舍：`Send + 'static` 边界（为什么闭包必须满足）、channel-only vs `Arc<Mutex>` 的选择与理由、锁中毒路径（若用了锁）、背压（`sync_channel` 容量）、并发测试的写法（重复跑/小压力/确定性断言）。
- **章结构**：对齐项目 1 成稿；引真实代码；所有行为真跑（含并发输出的确定性/非确定性说明）。
- **素材**：已定稿的 ch19–22（尤其 19/20）；std 文档逐条核对。
- **边界**：std-only；**不用 async**（一句话预告 async 版运行时 → 第 38 章）；不做外部依赖；不用 `unsafe`。
- **篇幅**：4000–5500 汉字（不含代码块）；练习 2–3 题带答案。
- **DoD**：§8 —— 独占 `src/project3-task-queue.md` + `examples/task-queue/**`（含其 `Cargo.toml`）+ 本票；禁 `git`/禁 `mdbook build`；自验：`CARGO_TARGET_DIR=.scratch/tmp-46/target cargo test --manifest-path examples/Cargo.toml -p task-queue` 全绿 + `mdbook test -c project3-task-queue.md` 绿；并发测试若偶发（时序类），在票里说明并给出稳定化方式（如 join 顺序/超时上界），不许留 flaky；探针 `.scratch/tmp-46/`；**勿动** `examples/Cargo.toml`、`examples/README.md`、其他 crate、其他章/票；收尾 `Status: claimed` + `## Comments`。

## Comments
