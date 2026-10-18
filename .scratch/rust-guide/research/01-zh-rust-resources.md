# 01 中文 Rust 学习资源地图 — findings

- 票：`.scratch/rust-guide/issues/01-zh-rust-resources.md`
- 查证日期：**2026-10-18**（下述星标/提交时间均为该日抓取的 GitHub API / 页面快照；时间敏感信息已注明）
- 标注约定：**[事实]** = 一手来源直接可见；**[推断]** = 由事实推导，非来源原文。
- 一手来源优先级：官方文档 / 原仓库 README·SUMMARY·commit API / 机构官网。

---

## 0. 结论速览（供 04 定位票直接使用）

1. **[事实]** 中文 Rust 资源的第一层（The Book / RBE / Rustlings / Nomicon 的中译与练习）已经**覆盖完整但普遍"停在 2022–2024 的快照"**：`rust-lang-cn/book-cn` 自述最后内容更新 2022-02-06，且**中文目录里根本没有异步章节**；`rust-by-example-cn` 自述最后一次更新 2022-01-26。
2. **[事实]** 唯一跟进到 **2024 edition（含 ch17 异步）**的中文 The Book 是 KaiserY 的 `trpl-zh-cn`（也被 rust-lang.org 中文站列为官方英文版的"非官方翻译"）。中文世界的"官方书"其实是两条互不同步的分叉。
3. **[事实]** 中文"进阶"内容事实上被 `course.rs`（Rust 语言圣经，31k★）一家垄断；它覆盖面最广，但**性能剖析、内存布局、编译优化等章节在 SUMMARY 里几乎全是 `todo`**，且声明为 **No License**（禁止修改后分发）。
4. **[事实]** 进阶里最被抱怨的几条 —— 借用检查器/生命周期心智模型、异步（Pin/Stream/async trait）、unsafe 的 soundness、工程实践（API 设计、性能）—— 在中文资源中**要么缺失、要么只是英文官方书/官方仓库的翻译或镜像**，缺少"中文原生、示例可编译、成体系"的进阶材料。
5. **[推断]** 差异化窗口：**中文原生、概念优先、示例可编译、覆盖 course.rs 的 todo 区（性能/内存/工程实践）与异步现状**。不写什么：不要再做 The Book 的逐章对照/又一版翻译（与 map.md 已否决的方向一致），不要重复入门练习。

---

## 1. 资源清单（逐项：定位 / 覆盖 / 深度 / 更新时效）

> 星标与提交时间 = 2026-10-18 通过 GitHub API 抓取。

### 1.1 The Book 中文版 —— 存在两条分叉（关键事实）

| | A. `rust-lang-cn/book-cn` | B. `KaiserY/trpl-zh-cn` |
|---|---|---|
| 仓库 | https://github.com/rust-lang-cn/book-cn | https://github.com/KaiserY/trpl-zh-cn |
| 星标 | 1034★ | 5647★ |
| 描述 | Rust 程序设计语言 中文版 | Rust 程序设计语言（**2024 edition 施工完毕**） |
| 最后 push | 2026-04-16（仅错别字类 PR：#177 "Fix formatting issue"） | 2026-08-22 |
| 阅读地址 | rustwiki.org/zh-CN/book（双语切换） | kaisery.github.io/trpl-zh-cn |
| 自述内容时效 | README："已 100% 翻译成中文，**最后更新时间为 2022-02-06**" | 描述注明 2024 edition，SUMMARY 与英文当前目录一致 |
| 是否含异步章 | **否** | **是（ch17-00-async-await）** |
| 官方站态度 | —— | rust-lang.org 中文站的"**非官方翻译**"指向它 |

- **[事实]** `book-cn` 的 `src/SUMMARY.md` 结构为 ch16 并发 → **ch17 面向对象** → ch18 模式 → ch19 高级 → ch20 Web 服务器，**没有异步章节**；仓库根下也不存在 `src/ch17-00-async-await.md`（API 返回 HTTP 404）。
  来源：https://raw.githubusercontent.com/rust-lang-cn/book-cn/master/src/SUMMARY.md ；https://api.github.com/repos/rust-lang-cn/book-cn/contents/src/ch17-00-async-await.md
- **[事实]** 英文 `rust-lang/book` 现目录为 ch17 "Fundamentals of Asynchronous Programming: Async, Await, Futures, and Streams" → ch18 OOP → … → ch21 最终项目；正文页声明假设 **Rust 1.90.0（2025-09-18）、edition 2024**。
  来源：https://raw.githubusercontent.com/rust-lang/book/main/src/SUMMARY.md ；https://doc.rust-lang.org/book/
- **[事实]** `trpl-zh-cn` 的 `src/SUMMARY.md` 已含 `ch17-00-async-await.md`，与英文 ch17 标题对齐（"异步编程基础：Async、Await、Future 与 Stream"）。
  来源：https://raw.githubusercontent.com/KaiserY/trpl-zh-cn/main/src/SUMMARY.md
- **[事实]** rust-lang.org 中文站"学习 Rust"页把 The Book / RBE / Nomicon 的中文版都标为 **"阅读非官方翻译"**，分别指向 kaisery.github.io/trpl-zh-cn、rustwiki.org/zh-CN/rust-by-example、nomicon.purewhite.io。
  来源：https://rust-lang.org/zh-CN/learn/
- **[推断]** 因此中文读者被搜索/引用引导到 `rustwiki`（book-cn）时，会读到**缺少异步章、结构停留在旧版**的 The Book；只有主动找到 KaiserY 版本才拿到 2024 edition。这是当前中文第一层资源最具体的"时效缺口"。

### 1.2 《Rust 语言圣经》(course.rs) —— 中文进阶事实标准

- 仓库：https://github.com/sunface/rust-course ，在线：https://course.rs
- **[事实]** 31038★ / 2641 fork；最后 push **2026-08-24**；README 自述"已写 **170 余章、110 余万字**"、"在线阅读 course.rs、在线练习 practice.course.rs"；**开源协议为 No License**（README 明示"不能私下修改后再包装分发"，仅 fork/阅读）。
  来源：https://api.github.com/repos/sunface/rust-course ；README。
- **[事实]** 覆盖：基础 → 进阶（生命周期、闭包/迭代器、智能指针、循环/自引用、多线程并发+原子/内存顺序、unsafe（含内联汇编）、宏、async 含 Pin/Unpin 与 Stream）→ 工具链（Cargo 全解、测试/CI/bench）→ 开发实践（日志/可观测、最佳实践）→ **手把手实现链表（含 Miri、栈借用 Stacked Borrows）** → 攻克编译错误/常见陷阱 → 性能优化。
  来源：https://raw.githubusercontent.com/sunface/rust-course/main/src/SUMMARY.md
- **[事实]** **性能优化整段几乎全部标注 `todo`/`doing`**：如"深入内存""指针和引用""内存分配""内存布局""常用性能测试工具""编译优化/LLVM""提升编译速度"等均 todo；"字符串操作性能""深入理解 move"为 doing；"Rust 性能优化 todo"。
  来源：同 SUMMARY.md
- **[事实]** README 坦言借鉴 Rust Book / Nomicon / Async Book 并统一致谢；随附 `assets/rustlings-zh`（内置 Rustlings 中文）。
- **[推断]** course.rs 是中文"进阶"质量的实际上限参考，也是最大竞品；它的 todo 区（性能/内存/编译优化）与工程实践正是可差异化空间。其 No License 意味着**不能直接搬运/改写其中的内容**（法务红线，写书须注意）。

### 1.3 《通过例子学 Rust》中文版 (Rust By Example)

- 仓库：https://github.com/rust-lang-cn/rust-by-example-cn ；阅读：https://rustwiki.org/zh-CN/rust-by-example
- **[事实]** 1978★；最后 push **2024-09-26**；README："首次于 **2016-08-07** 翻译完全部内容…**最后更新时间 2022-01-26**"；含在线代码编辑器；按 rust-translation-guide 规范翻译。
  来源：https://api.github.com/repos/rust-lang-cn/rust-by-example-cn ；README。
- 定位/深度：**示例驱动、入门级**；`macro_rules!`、`unsafe` 等只给最小示例，不解释心智模型。

### 1.4 Rustlings

- **[事实]** 官方 `rust-lang/rustlings` 64383★，纯英文（仓库无 i18n 目录，README/网站未提供官方中文）。
  来源：https://github.com/rust-lang/rustlings
- **[事实]** 中文：`rust-lang-cn/rustlings-cn` 245★，自述为 **"非官方中文翻译"**，最后 push **2024-11-24**；另有 course.rs 内置 `assets/rustlings-zh`。
  来源：https://api.github.com/repos/rust-lang-cn/rustlings-cn
- 定位/深度：**入门到中级的手感训练**；不承担进阶概念讲解。

### 1.5 rust-lang-cn 文档集（rustwiki.org 背后的组织）

- 组织：https://github.com/rust-lang-cn （24 个仓库，2026-10-18 快照）
- **[事实]** 重点仓库与各自最后 push：`book-cn`(2026-04-16)、`rust-by-example-cn`(2024-09-26)、`nomicon-zh-Hans`(**2026-10-05**)、`std-cn`(2026-02-26)、`cargo-cn`(2025-11-11)、`rust-cookbook-cn`(2025-11-08)、`reference-cn`(2023-08-01)、`rustdoc-cn`(2024-06-15)、`edition-guide-cn`(2022-10-01)、`style-guide-cn`(2023-07-31)、`english-chinese-glossary-of-rust`(2025-06-21)、`rust-translation-guide`(**2019-05-27**)。
  来源：https://github.com/orgs/rust-lang-cn/repositories
- **[事实]** `nomicon-zh-Hans`（Rust 秘典/死灵书简中）706★，**双语版**、声明"仍为草案"，活跃（2026-10-05）。**这是中文 unsafe 深水区最可靠的一手翻译。**
  来源：https://github.com/rust-lang-cn/nomicon-zh-Hans
- **[推断]** 该组织是"官方文档中文化"的主力，但多数镜像仓库的维护频率不均：越冷门/越底层的文档越陈旧（reference 停在 2023、edition-guide 停在 2022、translation-guide 停在 2019）；越常用越活跃（std、cargo、nomicon）。

### 1.6 异步（async）中文资源

- **[事实]** 官方 `rust-lang/async-book`（2.2k★）正文自带说明："this guide is currently undergoing a rewrite after a long time without much work. It is work in progress, **much is missing**"。
  来源：https://raw.githubusercontent.com/rust-lang/async-book/master/src/intro.md
- **[事实]** 中文：`rustcn-org/async-book`（196★，自述为 course.rs 中镜像专题，"高质量手翻"旧版 Async Book）；`sunface/tokio-course`（178★，"手翻并扩展" Tokio 官方 tutorial）。
  来源：https://github.com/rustcn-org/async-book ；https://github.com/sunface/tokio-course
- **[推断]** 中文异步内容 = "course.rs 镜像（旧版 async-book 译本）+ Tokio tutorial 译本"；没有跟进现状（Streams、async fn in trait、取消语义）的中文原生材料。**异步是最大的进阶缺口。**

### 1.7 社区与资讯

- **[事实]** `rustcc.cn`（Rust 语言中文社区）持续发《Rust 日报》，最近一期"【Rust日报】2026-08-27"发布于 2026-08-29。
  来源：https://rustcc.cn/article?id=b9753d50-9737-47fe-a394-7c979cbf8e28
- **[事实]** `rustcn-org/rust-weekly`（Rust 语言周刊，809★）**最后 push 2022-07-15，已停更**。
  来源：https://api.github.com/repos/rustcn-org/rust-weekly
- 其他：`rustcc/rust-zh`(729★ 中文化旧项目)、`rustcc/awesome-rust`(1069★)、`rustcn-org/rust-algos`（Rust 算法题解）。

### 1.8 付费课程与纸质书

- **[事实]** 极客时间《陈天 · Rust 编程第一课》**65 讲**，页面显示"**23196 人已学习**"，新人价 ¥68；作者 Tubi TV 研发副总裁。
  来源：https://time.geekbang.org/column/article/424017 等
- **[事实]** 极客时间《Rust 语言从入门到实战》（唐刚）**36 讲**，"**5266 人已学习**"，¥59；作者"Rust 语言中文社区联合创始人"。
  来源：https://time.geekbang.org/column/article/740385
- **[事实，二手来源]** 纸质书《Rust 权威指南》电子工业出版社 2020-05，毛靖凯/唐刚/沙渺 译，即 The Book 的中文译（截至 2020 版本）。
  来源：https://baike.baidu.com/item/Rust权威指南/50423819 ；网络销售页 https://www.vvnnvv.com/1149.html
- **[推断]** 付费课程为闭源、不可编译验证，读者无法像开源书那样核对示例；纸质书内容冻结在 2020 年前后版本。

### 1.9 博客 / 知乎 / 自媒体（碎片层）

- **[事实]** 中文进阶内容大量散落在知乎专栏、博客园、CSDN、腾讯云/阿里云开发者社区的**单篇/翻译稿**中（例如宏、借用检查器、并发专题），来源与深度参差，多为二手复述或译文（如《【译】Rust宏：教程与示例》zhuanlan.zhihu.com/p/353421021）。
- **[推断]** 没有单一权威的中文进阶博客/专栏；这一层适合做"延伸阅读"，不适合作为本书的参照坐标系。

---

## 2. 缺口与痛点

### 2.1 进阶主题的中文覆盖（事实 + 来源）

| 进阶主题 | 中文覆盖现状 | 一手依据 |
|---|---|---|
| 借用检查器**心智模型**（NLL/Polonius、为什么报错） | 只有零散"踩坑"文章；The Book cn 只到语法层 | course.rs 把"重复借用/生命周期过大/常见陷阱"单列成章，说明是高频卡点：SUMMARY "征服编译错误" |
| 生命周期进阶（型变 variance、子类型、HRTB、`'static` 误区） | 中文几乎空白；course.rs 的 `misconceptions`/`hrtb` 为注释掉/待办 | SUMMARY 中 `advance/lifetime/misconceptions` 被注释、`advance/hrtb` 注释 |
| 异步（Pin/Unpin、Streams、async trait、取消语义、运行时选择） | 仅旧版 async-book 译本 + Tokio tutorial 译本；官方 async-book 本身在重写 | async-book `intro.md`；`rustcn-org/async-book`；`sunface/tokio-course` |
| unsafe 的 soundness（别名规则、Stacked Borrows/Miri） | Nomicon 中译活跃且优质，但属**翻译**，无中文原生深化 | `nomicon-zh-Hans`(2026-10-05) |
| 性能/内存布局/剖析/编译优化 | course.rs 该整段**基本全是 TODO** | course.rs SUMMARY 性能优化段 |
| 宏（声明宏卫生性、过程宏工程化） | 入门示例有（RBE/course.rs），工程化/卫生性/调试深挖缺 | RBE `macros.html`；course.rs `advance/macro.md` 单章 |
| 工程实践（API 设计、错误处理在库层面、测试策略、发布/CI） | course.rs 有部分（Cargo/测试/日志），API 设计与错误处理体系化缺 | course.rs SUMMARY "开发实践"/"常用工具链" |

### 2.2 中文读者卡点（来自中文一手材料）

- **[事实]** course.rs 专设《避免从入门到放弃》："在开始接触 Rust 核心概念时（**所有权、借用、生命周期、智能指针**等），难度会陡然提升"；"学习一门新语言先用它写个链表或图试试…在 Rust 中**千万别这么干**…链表在 Rust 中简直是地狱一般的难度"；并批评"看一遍菜鸟教程就上手写 demo…最后要么放弃，要么回炉重造"。
  来源：https://raw.githubusercontent.com/sunface/rust-course/main/src/first-try/sth-you-should-not-do.md
- **[事实]** course.rs《Rust 难点攻关》开篇："我敢说 **90% 的人还是云里雾里**"，并以"切片与切片引用的区别/各种字符串的区别/各种指针与引用的区别/所有权转移·拷贝·克隆的区别"为典型困惑。
  来源：https://raw.githubusercontent.com/sunface/rust-course/main/src/difficulties/intro.md
- **[事实]** rustcc 论坛帖《Rust 推荐学习路径》读者反馈："网上找到蛮多 Rust 资料，**网页长得都一样**"（The Book / Comprehensive Rust / RBE / Reference 难择路）；评论建议"过一遍 Rust 圣经，直接搞项目，碰到问题边学边解决"、"直接写项目…简单的练习对 Rust 有时无法深入"，并推荐陈天/唐刚课程与 Jon Gjengset 的 Crust of Rust（B 站有搬运）。
  来源：https://rustcc.cn/article?id=a0bf1545-1dc7-429c-9f57-a12563e494be
- **[推断]** 综合三点：读者的核心痛点不是"缺入门材料"，而是 **(a) 入门材料太多且雷同、不知进阶该看什么**；**(b) 核心概念（借用/生命周期/指针类型差异）缺心智模型，只有规则背诵**；**(c) 从"能编译"到"工程可用"（异步、性能、API 设计）之间是断层**。

### 2.3 可操作的差异化结论（喂给 04 票）

**要写：**
1. **概念为骨的心智模型**：所有权/借用/生命周期**为什么这样设计**（对比 GC 语言与手动内存管理），而非规则罗列；尤其补"切片 vs 切片引用、`String`/`&str`/`str`、各种指针与智能指针、move/Copy/Clone"这类 course.rs 自认 90% 人含糊的点。
2. **异步现状**：Pin/Unpin、Streams、async fn in trait、运行时与取消语义 —— 中文目前只有旧译本，缺口最大。
3. **工程实践与性能**：API/错误处理设计、测试与 CI、benchmark、内存布局与剖析、编译优化 —— 直接填 course.rs 的 todo 区。
4. **示例可编译可验证**（地图已定的质量线）—— 现有中文资源里，翻译稿多、编译验证少的差距可被放大。

**不写：**
1. 不做 The Book 的逐章对照解读/又一版翻译（map.md "Out of scope" 已否决；且 book-cn/KaiserY 两版已占位）。
2. 不重讲入门语法与 Rustlings 级练习（RBE / rustlings-cn / course.rs 已充分覆盖）。
3. 不复制 course.rs 文本（其 **No License** 禁止修改后分发）。
4. 不追求 unsafe 黑魔法全谱（Nomicon 中译已高质量覆盖，且属进阶天花板，边际收益低）。

---

## 3. 术语惯例（中英对照与争议）

### 3.1 权威依据

- **《Rust 语言术语中英文对照表》**：`rust-lang-cn/english-chinese-glossary-of-rust`（156★，最后 push 2025-06-21，**CC0**），含 en / 简体 / 繁体三列，标注"持续更新中"、9 个未决 issue。
  来源：https://github.com/rust-lang-cn/english-chinese-glossary-of-rust/blob/master/rust-glossary.md
- **《Rust 文档翻译指引》**：`rust-lang-cn/rust-translation-guide`（69★，**最后 push 2019-05-27，已陈旧**），规定"术语首次出现加英文括号"、汉字与字母间空格、中英标点规则等。
  来源：https://github.com/rust-lang-cn/rust-translation-guide

### 3.2 通行译法（直接摘自术语表）

| English | 术语表译法 | 备注（术语表原文） |
|---|---|---|
| ownership | 所有权 | owner 所有者；own 占有/拥有 |
| borrowing / borrow checker | 借用 / 借用检查 | borrower 借用者 |
| lifetime | **生存时间，寿命，生命周期**（三者并列） | lifetime elision 生命周期省略 |
| smart pointer | 智能指针 | |
| trait | **特质** | "其字面上有'特性，特征'之意" |
| trait object | 特质对象 | |
| generic | 泛型 | monomorphization 单态 |
| box | 箱子，盒子，装箱类型（"一般不译"） | boxing 装箱 |
| crate | 包，包装箱，装包（"一般不译"） | "crate 是 Rust 的基本编译单元" |
| vector | （动态数组，一般不译） | |
| slice | 切片 | |
| closure | 闭包 | |
| macro | 宏 | declarative macro 声明宏；procedural macros 过程宏 |
| move | 移动，转移（并注："transfer 比 move 更贴合"） | |
| interior mutability | 内部可变性 | |
| deref (coercion) | 解引用 / 解引用强制转换 | |
| DST / unsized types | 动态大小类型 / 不定长类型 | |
| NLL | 非词法生命周期 | |
| soundness | 可靠性 | |
| coherence | **连贯性** | 依据 RFC 2451 语境自创，非通行译法 |
| target triple | **"多层次指标，三层/重 指标/目标"** | 术语表自注"此词翻译需要更多讨论" |
| token tree | 令牌树 | 自注"待进一步斟酌" |
| RAII / panic / lint / Rustacean / transmute / prelude / nightly | （不译） | |

### 3.3 实际存在的分歧（关键：术语表 ≠ 实际主流用法）

- **[事实]** `trait`：**术语表作「特质」**，但两版 The Book 中文都**保留英文 "trait"**（如 ch10 "泛型、trait 与生命周期"、"trait：定义共享的行为"），而 course.rs 用**「特征」**（"泛型和特征"/"特征 Trait"）。
  来源：book-cn SUMMARY；trpl-zh-cn SUMMARY；course.rs SUMMARY。→ **三方不一致**。
- **[事实]** `lifetime`：术语表给「生存时间/寿命/生命周期」并列，但主流资源统一用**「生命周期」**。
- **[事实]** `crate`：术语表倾向"包/包装箱"，book-cn 写作"包和 crate"（混用），course.rs 用"包"。
- **[推断]** 术语表因多年未体系化治理（9 issue、translation-guide 停在 2019），已与社区实际用法脱节；本书**不应盲从术语表**，宜采用"主流用法 + 首现加英文"的策略，并在内部维护自己的术语表。

### 3.4 建议（供 06 写作规范票参考）

- 采用主流社区用法：**trait（可保留英文）或"特征"二选一并全书统一**；**生命周期**；**智能指针**；**所有权/借用**；**宏**；**泛型**。
- 术语表仅作**参考来源**而非权威；对术语表自认有争议的词（coherence、target triple、token tree）在书中**保留英文原文**并首现给出中文解释。
- 沿用 rust-translation-guide 的排版规则（汉字与字母间空格、中英标点分离、术语首现加括号英文），因其已是社区通行规范且与 map.md 的"术语首现中文（English）"一致。

---

## 4. 主要一手来源清单（全部可复核）

- The Rust Book（英文，现行 edition 2024 / Rust 1.90）：https://doc.rust-lang.org/book/
- The Book 英文目录：https://raw.githubusercontent.com/rust-lang/book/main/src/SUMMARY.md
- rust-lang.org 中文"学习 Rust"（"非官方翻译"标注）：https://rust-lang.org/zh-CN/learn/
- book-cn（rustwiki）：https://github.com/rust-lang-cn/book-cn ；SUMMARY：https://raw.githubusercontent.com/rust-lang-cn/book-cn/master/src/SUMMARY.md
- trpl-zh-cn（KaiserY，2024 edition）：https://github.com/KaiserY/trpl-zh-cn ；SUMMARY：https://raw.githubusercontent.com/KaiserY/trpl-zh-cn/main/src/SUMMARY.md
- Rust 语言圣经 course.rs：https://github.com/sunface/rust-course ；SUMMARY：https://raw.githubusercontent.com/sunface/rust-course/main/src/SUMMARY.md
- 通过例子学 Rust 中文：https://github.com/rust-lang-cn/rust-by-example-cn
- Rustlings 官方：https://github.com/rust-lang/rustlings ；中文：https://github.com/rust-lang-cn/rustlings-cn
- Nomicon 中译：https://github.com/rust-lang-cn/nomicon-zh-Hans （阅读：https://nomicon.purewhite.io/）
- rust-lang-cn 组织仓库索引：https://github.com/orgs/rust-lang-cn/repositories
- async-book 官方：https://github.com/rust-lang/async-book ；中译镜像：https://github.com/rustcn-org/async-book ；Tokio 中译：https://github.com/sunface/tokio-course
- rustcc 社区日报：https://rustcc.cn/ ；Rust 语言周刊（停更）：https://github.com/rustcn-org/rust-weekly
- 术语表：https://github.com/rust-lang-cn/english-chinese-glossary-of-rust/blob/master/rust-glossary.md ；翻译指引：https://github.com/rust-lang-cn/rust-translation-guide
- 极客时间课程页：https://time.geekbang.org/column/article/424017 （陈天）、https://time.geekbang.org/column/article/740385 （唐刚）
- 中文读者痛点一手帖：https://rustcc.cn/article?id=a0bf1545-1dc7-429c-9f57-a12563e494be
