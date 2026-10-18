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
