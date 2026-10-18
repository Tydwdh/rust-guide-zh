# 写作规范（WRITING.md）

> 本书写作的硬约束：每张写作票开工前读一遍，审核以它为准。改动要走地图（`.scratch/rust-guide/map.md`）。
> 相关：定位与承诺 → `issues/04-positioning-and-differentiation.md`；版图与章节清单 → `outline-v0.md`；术语 → `GLOSSARY.md`。

## 0. 工作流

`mdbook serve --open` 预览 → 按 §4 模板写章 → 章节接进 `src/SUMMARY.md`（草稿条目接上链接）→ 跑 §3 校验链 → 交叉审校（§6）→ 用户验收 → 票内 `## Answer` → `Status: resolved`。

> 一轮多章并行（一票一个写作者子代理）走 §8 批次协议。

## 1. 示例组织（混合制）

| 场景 | 放哪 | 怎么写 |
|---|---|---|
| 短片段（≤15 行，服务叙述） | 章内内联 | 直接写 ```` ```rust ```` 块。**非 Rust 的片段（伪代码/输出/目录树）必须标 `text`/`console`/`toml`…** —— 裸栅栏会被当 Rust 编译测试 |
| 「读者看 3 行、实际是完整程序」 | `snippets/<章号>/foo.rs`（仓库根） | `{{#rustdoc_include ../snippets/ch04/foo.rs:3}}`；文件须保持完整可编译 |
| 实战项目 | `examples/<项目名>/`（workspace 成员） | 章内 `{{#include}}` 关键片段 + 一句「完整代码在 `examples/…`，跑 `cargo run -p …`」 |

硬规则：

- **被测块里绝不出现外部 crate 的 `use`**：`-L` 只认 `extern crate`，2018+ 的 `use` 需要隐藏行垫片（03 实测）。要引用配套 crate 时：放进 `rust,ignore` 块展示。
- **`{{#include}}` / `{{#rustdoc_include}}` 的内容会被 `mdbook test` 编译并执行**（预处理器先跑；rustdoc 按 doctest 自动包 `fn main`）—— 坏片段会让整章失败；会 panic / 步出非零的程序要标 `rust,should_panic` / `rust,no_run`（第 3 章实测踩过）；不能独立编译就整块标 `rust,ignore`，别指望 include 躲测试。
- **include 指令必须写在围栏内**（作为代码块的内容）：裸写会当普通段落渲染、且 `mdbook test` 完全不编译它（第 4 章实测踩过 —— 测试一直是绿的，但片段根本没被测）。
- **测试相关代码块不被 `mdbook test` 检查**（1.98 实测：`#[test] fn` 与 `#[cfg(test)] mod tests` 块体内的类型错误仍 exit 0，假绿）—— 测试类演示必须同时放进**可跑 crate**（`snippets/` 或 `examples/`），用 `cargo test` 真跑并把输出粘进正文。
- 样板噪音用隐藏行 `# `：渲染时默认收起（mdbook 的 `book.js` 给含隐藏行的块加 `hide-boring`，读者可用眼睛按钮展开）—— 别把不想被读者看到的内容放进去。
- 要展示 mdBook 指令本身时写 `\{{#…}}` —— 行内代码里也会展开（03 实测踩过）。
- 以 `#` 开头的展示内容不要放进 `rust` 块（会被当隐藏行吃掉），用 `text`。
- 教学报错块用 `rust,compile_fail`（正向断言「必须编译失败」）；要读者能点 play 但不断言用 `rust,ignore,mdbook-runnable`。
- 文件命名：章节 `src/chNN-<slug>.md`；片段与 `snippets/chNN/` 同名对应。

## 2. 版本与基线

- `book.toml` 的 `[rust] edition = "2024"` 必须存在（默认 2015 会让 `async` 等直接编译失败）。
- 正文声明：示例在**最新 stable**（写书时 1.98）验证；nightly-only 内容必须显式标注（`> [!WARNING]`）。
- 报错呈现 = **最小复现 + 关键行节选**：`compile_fail` 块 + 紧跟 ```` ```text ```` 的真实输出关键几行（剔噪声），标注「以 Rust 1.98 为例」。不整段粘贴。

## 3. 校验链（每张写作票必过；本地与 CI 同套命令）

```sh
# 1) 构建 + 扫 ERROR（build 会静默吞错且 exit 0，不能只看退出码）
mdbook build 2>&1 | tee .book-build.log
grep -q ERROR .book-build.log && { echo "build 日志有 ERROR"; exit 1; }

# 2) 章内代码块（失败 exit 101，可靠门禁）
mdbook test                      # 全量
mdbook test -c <章>              # 只测一章（并行批次自验用；参数 = SUMMARY 里该章的 src 相对路径或标题）

# 3) 示例 crate（workspace 就位后）
cargo test --manifest-path examples/Cargo.toml

# 4) 链接检查（book.toml 已启用 mdbook-linkcheck2；断链 → exit != 0；HTML 输出在 book/html/）
mdbook build
```

前置：`cargo install mdbook mdbook-linkcheck2`。

## 4. 章节模板（八节；可选加「语言对照角」）

```markdown
# 第 N 章 标题

## 这一章解决什么问题
（1–3 段：问题 + 读完能做到什么）

## 为什么这样设计
（心智模型：对比 GC/手动管理/其他语言，先讲为什么，再给规则）

## 机制与规则
（逐步展开；每条规则配最小示例）

## 动手看看
（§1 的示例；能跑的都进校验链）

## 报错现场
（至少一个真实报错：最小复现 → 报错关键行 → 定位路径 → 修法）

## 小结
（3–6 条，回指心智模型）

## 练习
（2–3 道理解题：解释为什么 / 找错 / 预测编译结果）

<details><summary>参考答案</summary>
（答案与解析）
</details>

## 延伸阅读
（官方一手链接：The Book / Reference / Nomicon / std…）
```

提示盒用内置 admonitions：`> [!NOTE]`、`> [!TIP]`、`> [!WARNING]`。
可选的「语言对照角」：只在概念跨语言差异大的章加（C++/Java/Python 读者注意点）；**跨语言断言必须核对对应语言的一手文档**（标准/官方参考；别凭记忆 —— 第 6 章的 C++ `substr` 断言就被审校抓出过）。

## 5. 风格细则

- 第二人称「你」；句子短；直接 —— 不卖萌、不文学化比喻、不用 emoji。
- 代码注释用中文；标识符永远英文。术语按 `GLOSSARY.md`（首现「中文（English）」，之后中文为主）。
- 章末「延伸阅读」只放官方一手来源；**链接不钉版本**（用 `https://doc.rust-lang.org/book/…`、`…/std/…` 这类稳定入口；版本敏感的事实写在正文里，如「以 Rust 1.98 为例」）。不点名任何现有中文资源（04 决定）。例外：必备工具的官方仓库可给（如调试扩展 CodeLLDB）。
- **命令行与路径口径**：console 块提示符只用 `$`；正文引用文件用正斜杠（`src/main.rs`）；命令与粘贴输出按本机实况（Windows 的 `\` 照实），首次出现处说明一次即可；调用刚编译出的可执行文件写 `$ .\name.exe`（Windows；PowerShell 也认这种写法）。（早期章节里个别按正斜杠展示的位置行属同义写法，不追溯。）
- 引用官方英文原文/报错行必须**逐字核对**（引号与「原话」内不得是转述；拿不准就写「大意是」「译自」）—— 第 1/2/11 章都被审校抓过同类问题。
- 常见误解走暗线：在叙述里纠偏，不设专门装置。

## 6. 写作票 DoD（加严版）

> 并行批次：步骤 1–3 由写作者完成并记录在票里；步骤 4–5 由协调者/用户执行，之后才写 `## Answer` 与 `Status: resolved`（见 §8）。

1. **成稿**：按 §4 模板写完，含练习与答案。
2. **自证**：跑 §3 校验链全绿，把命令与结果写进票。
3. **自查**：对照本文件逐项过（模板 / 术语 / 报错呈现 / 示例组织）。
4. **交叉审校**：起一个 `reviewer` 子代理（或另一会话）按 §3+§5 审，问题回票处理。
5. **用户验收**：通读 + 抽查 1–2 个示例按书中命令跑一遍。
6. **收尾**：章节接进 `src/SUMMARY.md`；票内 `## Answer` 记录「做了什么 / 验证命令与结果 / 遗留不确定」→ `Status: resolved`。

## 7. 已知坑（03 实测，别踩第二遍）

- `mdbook build` 对非法 `[output.html]` 字段、缺失 `{{#include}}` 都是 **ERROR + exit 0** —— 必须扫日志。
- 裸栅栏会被当 Rust 测试；非 Rust 片段必须标语言。
- `{{#include}}` / `{{#rustdoc_include}}` 引入的代码**照样被测**（预处理器先展开）——想只展示不编译，用 `rust,ignore`。
- 行内代码里的 `{{#…}}` 也会展开。
- `[build] create-missing = false` 已设：SUMMARY 指向缺失文件会报错（默认 true 会静默建空文件）。
- `[output]` 表多于一个（linkcheck2 已启用）：HTML 输出在 `book/html/`（预览/部署路径同步）。
- `mdbook test` 失败 exit 101（可门禁）；它**不**校验 `[output.html]`。
- `mdbook test -c <章>` 只认 **SUMMARY 里已挂的章**；参数是 src 相对路径（`ch01-….md`，不是 `src/ch01-….md`）或章标题，找不到就报 `ERROR Chapter not found`。
- `compile_fail,E····`（校验具体错误码）是 **nightly-only**（实测 stable 1.98 不校验、静默通过）→ 正文统一用稳定的 `rust,compile_fail`，错误码放进紧跟的输出块与叙述里。
- 含 `#[test]` / `#[cfg(test)]` 的块**不被编译**（cfg 排除，假绿）→ 测试类演示要配可跑 crate + `cargo test` 证据（见 §1 硬规则）。

## 8. 并行写作批次（批次协议）

一轮多章并行时：**一票一个写作者子代理**；协调者不写稿，负责预置与集成。

协调者（开写前 / 收尾）：
- 预置：建好各章占位文件、把 `SUMMARY.md` 草稿条目全部接上链接（`create-missing = false` 下条目指的文件必须先存在）。
- 收尾：跑全量链（§3 的 1–4 步）→ 交叉审校（§6.4）→ 统一提交。

写作者（每票一人）：
- **独占写**：本票的章文件、`snippets/chNN/`、本票文件 `.scratch/rust-guide/issues/NN-*.md`。
- **禁写（只读）**：`src/SUMMARY.md`、`book.toml`、`WRITING.md`、`GLOSSARY.md`、`README.md`、地图与其他票 —— 防止同文件互相覆盖；术语新词在票里回报，协调者统一并表。
- **禁止** `git`（协调者统一提交）；**禁止** `mdbook build` / `mdbook serve`（`book/` 竞态）；自验只用 `mdbook test -c <本章>`。
- 运行输出必须真跑真粘；探针与临时编译放 `.scratch/tmp-<票号>/`（已 gitignore），收尾删除。
- 只写本章范围；指向别章写一句「（第 N 章展开）」。
- 收尾：票 `Status:` 置 `claimed`；追加 `## Comments` 一条（文件 / 自验命令与结果 / 实测过的输出清单 / 遗留不确定 / 新增术语 / 衔接提示 / 图示需求）。**不写** `## Answer`、**不置** `resolved`。

## 9. 图示规范（内嵌 SVG）

体系已定（08 票，用户裁决）：**内嵌 SVG —— 手写、无脚本**。空间关系（内存布局、借用区间、模块树、包含关系、流程）优先画图；一张表或三行文字能说清的就不画。

硬规则：

- **单块、无空行**：整张图必须是 markdown 里一个连续的 HTML 块；块内出现空行会被 CommonMark 截断（`</svg>` 错位、图形碎裂 —— 实测踩过）。
- **无脚本、无外部资源**：不用 `<script>`、不引外链图片、不用 mermaid。
- **尺寸**：`viewBox` 定坐标（约 1 单位 ≈ 1px）、`width="100%"`、**不写 `height`**（保持比例、随内容栏缩放）。单图建议宽 600–760、高 150–260。
- **颜色只用主题变量**：`currentColor`（线/字）、`var(--links)`（强调）、`var(--warning-border)`（错误/警告）、`var(--quote-border)`（次要线）。**不写死颜色**（换主题会看不清）。
- **字号**：正文标注 12–14，图题 14；`font-family="monospace"` 用于代码样文本。
- **无障碍**：`role="img"` + `aria-label="一句话说清这张图要说明什么"`。
- **风格**：线宽 1.5–2、圆角 `rx` 4–6、不用渐变/阴影/装饰；图内文字中文、标识符英文原样（`&str`、`capacity`）。
- 图前后各留一个空行；不写图注编号，正文用一句话引出（「下图把……摊开：」）。
- 插入后自查：`mdbook test -c <章>` 仍绿（图不参与代码测试，但要确认没碰坏栅栏）；HTML 渲染由协调者在收尾 build 统一确认。
