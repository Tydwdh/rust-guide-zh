# 地图：Rust 进阶指南（中文 mdBook）

> wayfinder 地图。子票在 `issues/NN-<slug>.md`，每票一个**问题**；`Type:` 记录票型（research/prototype/grilling/task），`Status:` 记录 open/claimed/resolved，`Blocked by:` 连阻塞边（列出的票全部 resolved 才解锁）。
> 本图覆盖 wayfinder 的「只规划不施工」默认：Destination 是**成品本身**，写作执行票也在图内（先以迷雾形式存在，随决策票毕业）。

## Destination

一份写完并达到「可公开」质量线的中文 mdBook 指南（暂名《Rust 指南》）：**从零起步**（假定会一门主流语言，不假定会 Rust），以「理解 Rust 的设计」为主线、诊断训练贯穿全书，示例全可编译可验证。内容 = 基础篇 → 进阶篇（02 主干 13 主题）→ 补充章节（深水 D1–D7 与领域）。

## Notes

- **父仓库**：D:/Rust（git，`main`）。书籍源码 `src/`，配置 `book.toml`，示例 `examples/`（占位），构建产物 `book/` 已 gitignore。
- **语言**：正文简体中文；术语首现「中文（English）」，之后中文为主。
- **定位**（2026-10-18 由 04 翻转）：从零教程 —— 假定会一门主流语言/命令行，不假定会 Rust 与泛型/编译期概念（现场建立）；不设「基础补充盒」，基础即正文。
- **承诺主轴**：理解（读完能理解 Rust 的设计与底层模型）；诊断训练（读报错、改对、定位性能/并发）是贯穿方法；示例全可编译可验证、自包含（章末延伸阅读给官方链接）。
- **深度**：基础篇 → 进阶篇（02 主干 13 主题）→ 补充章节（深水 D1–D7 与领域；含系统层深入：手写集合/别名模型要点）。
- **不点名任何现有中文资源**（差异化只做正面陈述）；常见误解走暗线（纠偏融进叙述）。
- **教学法**：概念为骨 + 每部分配一个实战项目（心智模型先行）。
- **写作票**：每章一张；《基础篇》批次为 `09`–`21`（其余各篇按篇开）；每章末 2–3 道理解题（带答案）；每篇一个独立小项目。
- **写作规范**：开工前读 `WRITING.md`（示例组织 / 版本基线 / 校验链 / 八节模板 / 加严 DoD / 已知坑）与 `GLOSSARY.md`（术语）；写作票按它们自证。
- **进度**（2026-10-18）：基础篇（前言 + 第 1–11 章 + 项目 1）13 票全部落稿 → 13 份审校（每票一份） → 逐票修回完成 → **全量链全绿**（build 0 WARN/ERROR｜`mdbook test` exit 0｜examples 19+2 passed）。过程记录见 `batch-notes.md`。
- **进度**（2026-10-18，进阶篇）：第 12–18 章（票 `23`–`29`）全部落稿 → 7 份交叉审校（合计 5 blocker + 17 major + 37 minor）→ 逐票修回完成 → 术语并表（GLOSSARY +42 条、三处裁定修正）→ **全量链全绿**（同上一档口径）；2026-10-18 用户验收通过，票全部关闭（见 Decisions）。
- **进度**（2026-10-19，第三批）：第 19–22 章（票 `30`–`33`）全部落稿 → 4 份交叉审校（合计 **2 blocker + 2 major + 30 minor**）→ 逐票修回完成 → 术语并表（GLOSSARY +25 条）→ **全量链全绿**（build 0 ERROR/0 警告｜`mdbook test` 24 章 exit 0｜examples 19+2 passed）；进阶篇补图（票 `34`–`40`）落 **19 张**并过 HTML 计数与浏览器实机核对；**2026-10-19 用户验收通过，票全部关闭（见 Decisions）**。
- **图示**：体系 = **内嵌 SVG**（手写、无脚本、`currentColor` 随主题变色；单块内不得有空行）。基础篇 **36 张已补完**（票 `22`）；**进阶篇 19 张已补完**（票 `34`–`40`：ch12 3/ch13 3/ch14 2/ch15 1/ch16 3/ch17 5/ch18 2，HTML 计数核对通过）；后续篇章按 §9 沿用（规范 = `WRITING.md` §9）。
- **协作**：会话起草全部初稿、用户验收 → 每张写作票必须自证：示例编译/测试通过 + 关键事实引用官方来源（std/Reference/Nomicon）+ 不确定处显式标注。
- **工具链**：git 2.55 / rustc+cargo 1.98 stable / mdbook 0.5.4（Windows）。edition = **2024**（已写进 `book.toml`），验证基线 = 最新 stable；注意 mdbook 默认按 2015 编译代码块（见 03）。
- **发布**：先本地写，达质量线后再谈远端公开。
- **会话纪律**：每个会话只解决一张决策票；research 票可并行（例外）。
- **技能**：决策票用 grilling（+ domain-modeling）；原型票用 prototype；研究票用 research。

## Decisions so far

<!-- 一张关闭的票一行：票名（链接）: 一句话结论 -->

- [01 中文 Rust 学习资源地图](issues/01-zh-rust-resources.md): 中文第一层资源齐但陈旧分叉（book-cn 无异步章、末更 2022）；course.rs 是进阶事实上限、也是竞品兼法务红线（No License）；术语三方不一致（trait=特质/特征/保留英文）。
- [07 仓库工程骨架](issues/07-repo-engineering-skeleton.md): 骨架可 `mdbook serve`（`language=zh-CN` 生效、README/CI 就绪待远端、examples/ 留位）；实测 mdbook 0.5.4 配置非法时打印 ERROR 却 exit 0、`use-unicode-segmentation` 非法 —— 已转 06/03。
- [03 mdBook 能力盘点](issues/03-mdbook-capabilities.md): 实测能力与坑 —— `[rust] edition` 默认 2015（async 直接编译失败）；build 静默吞掉非法 `[output.html]` 字段与缺失 include 且 exit 0；代码块不标语言会被当 Rust 测；数学=MathJax 2.7(CDN)、Mermaid 需插件；链接检查改用 `mdbook-linkcheck2`；中文搜索无分词配置（整句=1 token，缓解见 findings §5）。
- [02 Rust 知识结构基线](issues/02-knowledge-structure-baseline.md): 以本机 1.98 文档快照 + rust-src 源码核对 —— 3 层 37 项主题全集 + 硬依赖 DAG / 6 章节批次；2024 edition 与 1.63→1.98 里程碑、仍未稳定清单；已过时说法 25 条 + 易踩坑 17 条；§6 已备好给 05 的输入与六项深挖票可行性判断。
- [04 定位与差异化边界](issues/04-positioning-and-differentiation.md): 定位翻转为「从零教程（假定会编程）」；承诺=理解（诊断作方法）；起点线/引用策略/误解暗线/边界（系统层深入、深水与领域进补充章节、不点名任何现有资源）/暂名《Rust 指南（暂定）》全部定稿。
- [05 内容版图与章节结构](issues/05-content-map-and-structure.md): 版图 v0 定稿 —— 六篇 41 章 + 5 项目（`SUMMARY.md` 草稿条目 + `outline-v0.md`）；顺序吃 02 批次序；每章一张写作票、每章 2–3 道理解题（带答案）；基础篇 13 张写作票（09–21）已开、全部 blocked by 06。
- [06 写作与验收规范](issues/06-writing-and-acceptance-spec.md): 规范定稿 —— `WRITING.md`（混合示例制 / edition 2024 + 最新 stable / 标准校验链 / 八节章节模板 / 加严 DoD 含交叉审校 / 已知坑）+ `GLOSSARY.md` 草案（trait 保留英文）；`book.toml` 已设 edition 与 create-missing。
- [08 样板章](issues/08-sample-chapter-prototype.md): 原型由批量写作取代（基础篇 13 票落稿 + 13 份审校（每票一份） + 逐票修回）；反馈沉淀为 WRITING 增补（示例组织四硬规则 / 跨语言断言 / 逐字核对 / 命令行与路径口径 / 六条实测坑）；**图示体系定案 = 内嵌 SVG**（用户裁决），样例图落在 ch06。
- 基础篇写作票 09–21（13 张）: **全部 resolved**（2026-10-18 用户验收通过）—— 13 章落稿 + 13 份审校修回 + 全量链全绿；逐票 `## Answer` 已写，过程记录见 `batch-notes.md`。
- 进阶篇写作票 23–29（第 12–18 章）: **全部 resolved**（2026-10-18 用户验收通过）—— 7 章落稿 + 7 份交叉审校修回（5 blocker + 17 major + 37 minor）+ 术语并表（GLOSSARY +42 条）→ 全量链全绿；逐票 `## Answer` 已写，过程记录见 `batch-notes.md`。
- 第三批：写作票 30–33（第 19–22 章）与进阶篇补图票 34–40: **全部 resolved**（2026-10-19 用户验收通过）—— 4 新章（5435–5495 汉字）+ 4 份审校修回（2 blocker + 2 major + 30 minor）+ 19 张 SVG + 术语并表（GLOSSARY +25 条）→ 全量链全绿；逐票 `## Answer` 已写，过程记录见 `batch-notes.md`。

## Not yet specified

<!-- 还看不清的问题：够不上开票的精度，随前沿推进毕业。不许把它当成待办清单。 -->

- 其余各篇的写作票（按篇开）：第三批（第 19–22 章 + 进阶篇补图，票 30–40）已关闭（见 Decisions）；**项目 2/3 与系统层（第 23 章起）/ 工程实践 / 补充章节待开**
- 其余各篇实战项目的选型与配套 crate（项目 1 已定：`examples/todo-cli`；项目 2–5 各篇开写时定）
- 基础篇之外的图示沿用（async 状态机图、数据布局图等：体系已定 SVG，写到对应篇时再验复杂图的可行性）
- 书名、副标题与品牌（暂用《Rust 指南（暂定）》；正式书名待定）
- 许可协议与版权页（公开前必须落定）
- 公开时机与反馈回路（连载节奏、评论渠道）
- 中文搜索体验：03 已结论 —— tokenizer 只按空白/`-` 切，中文整句=1 token，无配置可解（缓解思路见 `research/03-mdbook-capabilities.md` §5）；要不要额外方案，等有正文后拿真内容试搜索再定。

## Out of scope

- 以《The Book》为骨架的逐章对照解读 / 翻译：第 2 轮定位选型已否决 —— 与现有中文资料重叠，且不是本书定位（进阶深挖、假定基础已过）。
