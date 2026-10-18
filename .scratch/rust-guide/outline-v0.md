# 版图 v0（05 定稿 · 2026-10-18）

> 依据：02 的依赖序与批次表（批 0→批 5、13 主干主题）、04 的定位声明（从零起步 / 承诺=理解 / 诊断作方法 / 示例可编译 / 自包含）。
> 顺序：照 02 §3.2 批次序；三条不可妥协的边已落在章序里（trait 先于 dyn/RPIT/GAT；async 先于 Pin；unsafe 先于指针/布局/FFI）。
> 切分：**每章一张写作票**；每篇一个独立小项目（补充章节除外，各章自带动手做）；**每章末 2–3 道理解题（带答案）**。
> `src/SUMMARY.md` 已挂全书草稿条目；写完一章就把对应条目接上链接。

## 第一篇 基础篇（项目 1：命令行小工具）

| 章 | 一句话 | 02 依据 |
|---|---|---|
| 1 起步 | 工具链与第一个程序；本书约定（edition 2024、示例怎么跑） | 批 0（I23 约定） |
| 2 读懂编译器 | 报错的解剖与修复路径（诊断第一课）+ 所有权引子 | 04 承诺（诊断作方法） |
| 3 语言基础 | 变量、类型、函数与控制流 | 新增（原 B 层假设已会） |
| 4 所有权、借用与切片 | 心智模型核心章：为什么这样设计 + 常见报错与修法 | B1 |
| 5 结构体、枚举与模式匹配 | 状态建模的基本工具 | 新增 |
| 6 集合与字符串 | Vec/String/HashMap + UTF-8 与索引陷阱 | 新增 |
| 7 错误处理 | Result/?/panic/自定义错误 | B3 |
| 8 模块、包与依赖 | 模块系统与 Cargo 基本项 | B6 |
| 9 泛型与 trait 入门 | 泛型 = 编译期替换（单态化初识）；trait = 行为契约 | B2 |
| 10 迭代器与闭包 | 闭包捕获、迭代器链、零成本抽象初识 | B4 |
| 11 测试与调试 | cargo test 与「从失败回推问题」 | 新增 |
| 项目 1 | 命令行小工具（**定稿：待办清单 CLI**，代码在 `examples/todo-cli/`） | — |

## 第二篇 类型与 trait 主干（项目 2：小型库 crate）

| 章 | 一句话 | 02 依据 |
|---|---|---|
| 12 trait 设计 | 关联类型 vs 泛型参数、supertrait、sealed、扩展 trait、coherence | I1 |
| 13 dyn 兼容性与 trait 对象 | 对象安全规则、vtable、upcasting | I2 |
| 14 impl Trait 与 opaque type | RPIT/RPITIT/AFIT、2024 捕获规则 | I3 |
| 15 泛型的边界 | const 泛型与 GAT | I4/I5 |
| 16 生命周期进阶 | variance、HRTB、drop check、PhantomData | I6 |
| 17 智能指针与内部可变性 | Box/Rc/Arc/Cell/RefCell/Mutex/RwLock/Cow、Deref 陷阱 | I7 |
| 18 错误处理与 API 设计 | source 链、From 策略、builder/typestate、SemVer | I8 |
| 项目 2 | 小型库 crate（候选：配置解析器 / 表达式求值器） | — |

## 第三篇 并发与异步（项目 3：并发/异步小程序）

| 章 | 一句话 | 02 依据 |
|---|---|---|
| 19 并发基础 | thread/scope、Send/Sync、Arc<Mutex>、channel | I9 |
| 20 原子与内存序 | Ordering 五值、happens-before、CAS 与无锁入门 | I10 |
| 21 async 基础 | Future、执行器概念、async/.await、运行时选型 | I11 |
| 22 Pin 与自引用 | Pin/Unpin、pin!/Box::pin、何时真的需要 unsafe | I12 |
| 项目 3 | 并发/异步小程序（候选：并发爬取 / 迷你任务队列） | — |

## 第四篇 系统层（项目 4：FFI 与 unsafe 小项目）

| 章 | 一句话 | 02 依据 |
|---|---|---|
| 23 unsafe 与 UB | unsafe 四能力、UB 清单、Miri | I13 |
| 24 指针、provenance 与未初始化内存 | NonNull、strict provenance、MaybeUninit、&raw | I14 |
| 25 数据布局与 repr | repr(Rust)/C/transparent、niche、DST 与胖指针 | I15 |
| 26 FFI 与 ABI | unsafe extern、C 字符串、回调、repr(C) 契约 | I16 |
| 项目 4 | FFI 小项目（候选：封装一个系统 API / 写一个 unsafe 容器） | — |

## 第五篇 工程实践（项目 5：工程化改造）

| 章 | 一句话 | 02 依据 |
|---|---|---|
| 27 宏：macro_rules! | 片段说明符、卫生性、递归、$crate | I17 |
| 28 过程宏 | derive/属性/函数式、syn/quote、诊断信息 | I18 |
| 29 性能与剖析 | profile、benchmark、flamegraph、PGO、分配与克隆成本 | I19 |
| 30 Cargo 与 workspace 工程化 | features、workspace 继承、lints、MSRV | I20 |
| 31 测试进阶 | 集成/doctest、compile_fail、proptest、nextest、Miri/loom 入 CI | I21 |
| 32 文档与 lint 工程 | rustdoc、intra-doc link、clippy 分组、#[expect] | I22 |
| 33 Edition 与迁移 | 2024 改动清单、cargo fix --edition | I23 |
| 项目 5 | 把项目工程化（CI + 发布 + 文档） | — |

## 第六篇 补充章节

| 章 | 一句话 | 02 依据 |
|---|---|---|
| 34 手写集合与智能指针 | RawVec/Drain/ZST、Arc+Mutex 实现 | D1 |
| 35 别名模型与形式化 | Stacked/Tree Borrows、UCG、provenance 细节 | D2 |
| 36 高级类型系统 | coherence 细节、negative impls、specialization（标注 nightly） | D3 |
| 37 编译器与代码生成 | MIR、单态化、-C 选项、inline asm、LTO | D4 |
| 38 自己实现极简 async 运行时 | Waker、手写状态机、pin 投影 | D5 |
| 39 宏系统极限 | macro_metavar_expr、属性宏组合、track_caller、诊断设计 | D6 |
| 40 生产工程 | panic 策略、动态库与 ABI、交叉编译、build-std、registry | D7 |
| 41 领域地图 | no_std/嵌入式、WASM、GUI、CLI/服务…各领域入口与最小示例 | 04 决定（领域进补充章节） |

## 写作票与切片

- **《基础篇》13 张**（`09`–`21`）：初稿 + 交叉审校（13 份 reviewer 报告（每票一份））+ 逐票修回已全部完成（2026-10-18）；逐票收尾（`## Answer` + `Status: resolved`）待用户验收后补记，过程记录见 `batch-notes.md`。
- 其余各篇按篇开票（规则已定，见地图迷雾）。
- 每票自证：示例编译/测试通过 + 关键事实引用官方来源 + 不确定处显式标注（地图 Notes）。
