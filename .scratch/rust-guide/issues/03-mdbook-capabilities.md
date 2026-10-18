# 03 mdBook 能力盘点

Type: research
Status: resolved

## Question

本仓库的文档工具链（mdbook 0.5.4 / rustc 1.98 / Windows）到底能做什么：示例代码如何校验、渲染、发布？

产出：

1. **代码校验**：`mdbook test`（rustdoc doctest）对本项目代码块的要求与限制；`ignore`/`no_run`/`should_panic`/隐藏行（`#`）等写法；`compile_fail` 场景（教学故意报错）怎么处理。
2. **渲染能力**：playground 按钮与 `{{#playground}}`；KaTeX；Mermaid / SVG 内嵌；自定义主题；`book.toml` 相关配置项。
3. **质量管线**：`mdbook build`、`mdbook serve`、`mdbook-linkcheck`（或替代品）的现状与安装方式；GitHub Actions 上构建 + 部署 Pages 的可行方案；本地优先的工作流。
4. **示例工程结构**：多文件示例（配套 crate / workspace）在 mdBook 里的常见组织方式与取舍。
5. **本机验证**：在本仓库实际跑一遍最小样例，记录真实输出（命令、版本、结果），别只抄文档。

**判据**：优先一手来源（mdBook 官方文档/仓库、rustdoc book），并附本机实测证据（命令 + 输出）。

**用途**：作为「06 写作与验收规范」与「07 仓库工程骨架」的输入。

产出物：`.scratch/rust-guide/research/03-mdbook-capabilities.md`，结论要点写进本票 `## Answer`；实测用的临时样例清理掉，只把证据留在 findings 文档里。

## Answer

1. **代码校验**：`mdbook test` 就是 rustdoc doctest，**只测 SUMMARY.md 里列出的章节**；通过 exit 0、失败 exit 101，可直接做 CI 门禁。`ignore`/`no_run`/`should_panic`/`compile_fail`/`editable`/`noplayground`/`mdbook-runnable`/`editionXXXX` 全部实测可用；`compile_fail` 即「教学故意报错」的正解。
2. **两个必须写进规范的坑（本机实测）**：①**无语言栅栏会被当 Rust 测**，正文非 Rust 片段必须标 `text` 等语言；②`[rust] edition` **默认 2015**，`book.toml` 必须显式设 `edition = "2024"`，否则 `async` 等语法全灭（07 号票必改）。
3. **隐藏行/转义**：`# ` → `<span class="boring">`（读者可展开、仍参与编译）；`##` 转义只解决渲染，喂给 test 仍会报错，展示 `#` 开头的文本请用 `text` 或 `ignore`。**mdBook 的 `{{#…}}` 指令在行内代码里也会展开**，正文展示指令本身必须写 `\{{#include …}}`（我实测踩过，导致 21 个栅栏错乱报错）。
4. **渲染**：playground 走公网 `play.rust-lang.org`（中国读者体验差，建议 `runnable=false` 或加说明）；数学是 `mathjax-support=true` → **cdnjs CDN 的 MathJax 2.7.1（不是 KaTeX、不落本地资源）**，且文档明确不支持 `$`/`$$`，只用 `\\(…\\)`/`\\[…\\]`；**0.5.4 无内置 Mermaid**（需 `mdbook-mermaid` v0.17.1，活跃、本地引 JS）；SVG 内嵌直接可用；0.5 内置 **admonitions**（`> [!NOTE]`）与定义列表，默认开启，适合做提示/警告盒。
5. **质量管线**：`mdbook build` 有**两个静默陷阱**——缺失 `{{#include}}`、以及 `[output.html]` 里任何未知字段，**都只打 ERROR 然后 exit 0**，后者还会让**整个 `[output.html]` 表被丢弃退回默认值**（源码：`html_config()` 吞错返回 `None` + 渲染器 `unwrap_or_default()`）；而 `mdbook test` 根本不校验 `[output.html]`。⇒ CI 必须额外 grep `ERROR` 日志，不能只看退出码。链接检查改用官方现推的 **`mdbook-linkcheck2`**（老 `mdbook-linkcheck` 最后发版 2022-10，停滞）。
6. **示例工程组织（可执行建议）**：短示例内联；「读者看 3 行、实际是完整程序」用 `{{#rustdoc_include file.rs:2}}`（范围外用 `#` 隐藏，`mdbook test` 仍编译整程序）；实战项目放独立 crate 由 `cargo test` 负责正确性。**引用配套 crate 的 `-L` 能力很窄**：只对 `extern crate X;` 生效，2018+ `use` 路径必须在块顶加隐藏行 `# extern crate X;`，且 `RUSTDOCFLAGS` 被忽略、无 `--` 透传。
7. **Main 追加问的 `use-unicode-segmentation`**：该字段在 mdBook **从来不存在**（v0.3.7/v0.4.0/v0.4.52/master 的 config 与文档搜 `unicode` 全 0 命中，均核对过 HTTP 200 与体积）；搜索 tokenizer 至今**只按空白和 `-` 切**，中文整句=一个 token，**无任何配置可改善**（建议：关键术语中英对照 + 标题分短语，真要做中文分词得自写 preprocessor）。配置字段非法 exit 0 的原因见第 5 条。
8. **[未验证]**：MathJax 四种定界符的浏览器实测（未起浏览器）、`mdbook-linkcheck2` / `mdbook-mermaid` 的实际运行（未安装，维护状态与安装方式已核实）、`create-missing=false` 的退出码、其他顶层表的未知字段行为。

完整证据（命令、输出、源码行号、来源链接）见 `.scratch/rust-guide/research/03-mdbook-capabilities.md`；实测临时目录已清理。
