# 地图：Rust 进阶指南（中文 mdBook）

> wayfinder 地图。子票在 `issues/NN-<slug>.md`，每票一个**问题**；`Type:` 记录票型（research/prototype/grilling/task），`Status:` 记录 open/claimed/resolved，`Blocked by:` 连阻塞边（列出的票全部 resolved 才解锁）。
> 本图覆盖 wayfinder 的「只规划不施工」默认：Destination 是**成品本身**，写作执行票也在图内（先以迷雾形式存在，随决策票毕业）。

## Destination

一份写完并达到「可公开」质量线的中文 mdBook 指南（暂名《Rust 进阶指南》）：面向有编程经验的读者、假定 Rust 基础已过，概念为骨 + 每部分配实战，内容可编译、可验证。

## Notes

- **父仓库**：D:/Rust（git，`main`）。书籍源码 `src/`，配置 `book.toml`，构建产物 `book/` 已 gitignore。
- **语言**：正文简体中文；术语首现「中文（English）」，之后中文为主。
- **受众**：会编程（不限语言）、Rust 基础已过（读过 The Book 或能写小程序）；不重讲基础，必要时给「基础补充盒」。
- **教学法**：概念为骨 + 每部分配一个实战项目。
- **协作**：会话起草全部初稿、用户验收 → 每张写作票必须自证：示例编译/测试通过 + 关键事实引用官方来源（std/Reference/Nomicon）+ 不确定处显式标注。
- **工具链**：git 2.55 / rustc+cargo 1.98 stable / mdbook 0.5.4（Windows）。edition 与 MSRV 由 06 定。
- **发布**：先本地写，达质量线后再谈远端公开。
- **会话纪律**：每个会话只解决一张决策票；research 票可并行（例外）。
- **技能**：决策票用 grilling（+ domain-modeling）；原型票用 prototype；研究票用 research。

## Decisions so far

<!-- 一张关闭的票一行：票名（链接）: 一句话结论 -->

（暂无）

## Not yet specified

<!-- 还看不清的问题：够不上开票的精度，随前沿推进毕业。不许把它当成待办清单。 -->

- 每章写作票的切分与粒度（待 05 内容版图）
- 各进阶专题是否单独开深挖票：async/Pin、宏、unsafe/FFI、并发与内存序、性能剖析、错误处理与 API 设计（待 05）
- 每部分实战项目的选型与配套仓库（待 05）
- 图示体系（借用/内存图、async 状态机图）与工具（内嵌 SVG？Mermaid？）（待 08 样板章暴露）
- 练习题与自测（是否带答案、是否对接 Rustlings/Exercism）（待 05/08）
- 书名、副标题与品牌（暂用「Rust 进阶指南（暂定）」）
- 许可协议与版权页（公开前必须落定）
- 公开时机与反馈回路（连载节奏、评论渠道）
- 章内模板细节（开场、小结、延伸阅读）（待 06/08）

## Out of scope

- 以《The Book》为骨架的逐章对照解读 / 翻译：第 2 轮定位选型已否决 —— 与现有中文资料重叠，且不是本书定位（进阶深挖、假定基础已过）。
