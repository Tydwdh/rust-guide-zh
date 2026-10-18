# 06 写作与验收规范

Type: grilling
Status: open
Blocked by: 03

## Question

在这本书里，「合格的一章」长什么样，怎么被验证？

要定下来的：

1. **示例规范**：代码放哪（章节内片段 / 配套 crate / workspace）、edition 与 MSRV、错误示例怎么写、编译器输出怎么呈现（真实输出）、示例命名。
2. **校验链**：写作票必须通过的检查（示例编译/测试、`mdbook build`、链接检查、术语一致性……），在 CI 还是本地跑。
3. **章节模板**：开场、正文、小结、延伸阅读、「基础补充盒」的固定形态。
4. **验收标准（DoD）**：一张写作票从 draft 到 resolved 的状态机与判据；用户验收的可操作性（看什么、怎么抽查）。
5. **写作风格细则**：人称、语气、代码注释语言；「中文为主」下术语一致性的维护机制（术语表放哪、怎么更新）。

产出：仓库内写作规范文档（路径由本票定）+ 术语表文件草案。

技能：grilling + domain-modeling（术语表）。

## Comments

- 2026-10-18（07 仓库工程骨架会话实测）：mdbook 0.5.4 在 `book.toml` 配置字段非法时**打印 ERROR 日志但继续构建、exit 0**；本地实测 `[output.html.search] use-unicode-segmentation` 即非法（合法字段：enable / limit-results / teaser-word-count / use-boolean-and / boost-title / boost-hierarchy / boost-paragraph / expand / heading-split-level / copy-js / chapter）。写校验链时不能只看退出码。
- 2026-10-18（03 返回后补记）：mdbook 默认按 **edition 2015** 编译书里代码块 —— 不显式设 `[rust] edition` 的话，`async` 等新语法在 `mdbook test` 里直接失败（该值由本票定，定完写进 `book.toml`）。另：代码块不标语言会被当 Rust 编译；`mdbook test` 不校验 `[output.html]`，而 `build` 会静默吞掉它的非法字段（exit 0）⇒ 校验链必须扫 build 日志中的 ERROR。完整证据见 `research/03-mdbook-capabilities.md`。
- 2026-10-18（04 落定后）：定位翻转为**从零教程**（承诺主轴 = 理解，诊断作方法）。章节模板需承载：心智模型先行（先讲为什么再给规则）、诊断训练（教读编译器报错）、自包含（章末延伸阅读）；术语仍按地图 Notes（首现中英对照、之后中文为主）。
