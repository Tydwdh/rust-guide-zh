# 06 写作与验收规范

Type: grilling
Status: resolved
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
- 2026-10-18（05 定稿后）：版图就位（41 章 + 5 项目；`SUMMARY.md` 已挂草稿条目）。切片=每章一张写作票，**基础篇 13 张（09–21）全部 blocked by 本票**；章节模板需含「练习节：2–3 道理解题 + 答案落位（章末折叠 vs 附录由本票定）」。

## Answer

写作规范定稿（2026-10-18）。产出：`WRITING.md`（规范本体）、`GLOSSARY.md`（术语表草案）、`book.toml`（`[rust] edition = "2024"`、`[build] create-missing = false`）、CI 升为标准链、地图 / 08 / README 同步。标准链已实测跑通：build 无 ERROR、`mdbook test` exit 0。

### 决定（两轮 grill 全数确认）

- **示例组织（混合制）**：短片段章内内联；「读者看 3 行、实际完整程序」放 `src/snippets/chNN/` + `{{#rustdoc_include}}`；实战项目放 `examples/` workspace（`cargo test` 兜正确性）。
- **版本基线**：edition 2024 + 最新 stable（写书时 1.98）验证；nightly-only 显式标注；报错 = 最小复现 + 关键行节选 + 标注版本。
- **章节模板**：八节（问题 / 心智模型 / 机制 / 示例 / 报错现场 / 小结 / 练习+答案 / 延伸阅读）+ 可选「语言对照角」；答案用 `<details>` 折在章末。
- **校验链（标准链）**：`mdbook build` 扫 ERROR → `mdbook test` → `cargo test`（examples workspace）→ linkcheck2；本地与 CI 同套命令（CI 已写好，无远端不生效）。
- **DoD（加严）**：成稿 → 自证链全绿（命令与结果进票）→ 对照 `WRITING.md` 自查 → **交叉审校（`reviewer` 子代理或另一会话）** → 用户验收（通读 + 抽查 1–2 个示例按书中命令跑一遍）→ 接入 `SUMMARY.md` + `## Answer` → resolved。
- **术语**：根 `GLOSSARY.md`（作者向，随写随更新）；首现中英对照、之后按表；**trait 保留英文不译**（「特征/特质」之争的处置；可改判，只改表）。
- **风格**：「你」直呼、句子短、不卖萌、无 emoji；代码注释中文、标识符英文。

### 实测与坑（已写进 WRITING.md §7）

- `mdbook build` 对非法 `[output.html]`、缺失 `{{#include}}` 都是 ERROR + exit 0 → 链里必须扫日志。
- 裸栅栏被当 Rust 测；行内代码里的 `{{#…}}` 也会展开；`-L` 对 2018+ `use` 无效（须隐藏行 `extern crate`）。
- `create-missing = false` 已设（默认 true 会静默建空文件）。
- linkcheck2：本机 `cargo install mdbook-linkcheck2` 进行中；启用后 HTML 输出挪到 `book/html/`（部署路径同步改）。
