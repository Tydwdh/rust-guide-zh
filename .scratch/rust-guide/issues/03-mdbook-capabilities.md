# 03 mdBook 能力盘点

Type: research
Status: open

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
