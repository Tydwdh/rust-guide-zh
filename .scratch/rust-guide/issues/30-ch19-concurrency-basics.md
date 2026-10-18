# 30 第 19 章 并发基础：thread、Send/Sync 与共享状态

Type: task
Status: open

## Question

写第三篇第一章：**并发基础**。第 17 章已给 `Mutex`/`RwLock`/`Arc`/`Cell` 的 API 面；本章接「跨线程的组合与规则」。

- **范围**：`thread::spawn` 与 `thread::scope`（借用如何跨线程：`scope` 的 join 保证）；`Send`/`Sync` 的准确语义（auto trait、为什么 `Rc` 不是 `Send`、`unsafe impl` 与不变量）；`Arc<Mutex<T>>` 的组合（含锁中毒路径与 `RwLock` 的取舍回指第 17 章）；channel（`std::sync::mpsc` 两类发送端、`Sender` 的移动语义、消失的发送端）；`thread_local!` 一小节；收束回「数据竞争在安全代码里被类型系统排除」。
- **素材**：`research/02-knowledge-structure.md` 的 I9 行（依赖 B1/I7；书 ch16、`std/marker`、Nomicon send-and-sync / poisoning）；本机 1.98 快照：`std/thread`、`std/sync`、`std/sync/mpsc`、`std/marker/trait.Send.html`、`std/marker/trait.Sync.html`；实测行为用 rustc 1.98 + edition 2024 真跑。
- **边界**：`Ordering`/CAS/无锁 → 第 20 章；async/`Future` → 第 21 章；`Pin` → 第 22 章；别名模型与数据竞争的形式化 → 第 35 章；`loom` 在测试里的位置 → 第 31 章（一句带过）。第 17 章的 `Mutex`/`RwLock` API 细节不重讲，只写「跨线程组合时新出现的东西」。
- **篇幅**：4000–5500 汉字（不含代码块）；八节模板；练习 2–3 题带答案；`thread::scope` 标稳定版本（1.63）；nightly-only 只标注不进正文；可选「语言对照角」（若写，Java/Go 的线程模型断言须核一手文档，宁缺勿错）。
- **DoD**：按 `WRITING.md` §8 批次协议 —— 独占 `src/ch19-concurrency-basics.md`（替换占位内容）+ `snippets/ch19/` + 本票；禁 `git`/禁 `mdbook build`；自验 `mdbook test -c ch19-concurrency-basics.md` 绿；探针放 `.scratch/tmp-30/`（收尾删除）；运行输出真跑真粘；英文引文逐字核对；收尾 `Status: claimed` + `## Comments` 一条（文件 / 自验命令与结果 / 实测输出清单 / 遗留不确定 / 新增术语 / 衔接提示 / 图示需求）。

## Comments
