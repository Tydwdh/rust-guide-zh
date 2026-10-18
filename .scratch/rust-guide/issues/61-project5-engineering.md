# 61 项目 5：工程化改造

Type: task
Status: open

## Question

写第五篇收尾项目：**工程化改造** —— 把 `examples/release-drill`（协调者已建 stub）工程化到「可以发布」，并以**本仓库的真实工程**为样板。

- **目标**：
  1. `release-drill` crate：实现一个零依赖小库（建议「字节数 → 人类可读字符串」一类，接口自定），带单元测试与文档示例；补齐发布件：`Cargo.toml` 元数据（description / license / readme / keywords / categories / rust-version）、`README.md`、`CHANGELOG.md`（Keep-a-Changelog 模板 + 起始条目）、docs.rs 元数据（`[package.metadata.docs.rs]`）。
  2. 「发布前打包检查」：`cargo package --list` 与 `cargo package`（构建验证）真跑真粘；`cargo publish` 明确不在本书执行（无凭据），给一份「发布清单」。
  3. CI：以本仓库真实工作流 `.github/workflows/book.yml` 逐段讲，并**真实加固**它（二选一：A. 同 workflow 增加 `fmt`/`clippy`/`doc -D warnings` 作业；B. 新增独立 workflow 文件）。新命令必须在本机逐条真跑并写进票；不破坏既有三个步骤；说明「真实运行要等仓库公开到 GitHub」。
- **素材**：`.github/workflows/book.yml`（现成）、`WRITING.md` §3、ch30/32 已定稿（`[lints]`/clippy/文档门禁——回指不重讲）、Cargo Book `publishing.html` 与 manifest 页（快照）、docs.rs 文档。
- **边界**：不实际 `cargo publish`；不给 `release-drill` 加依赖；工具细节回指 ch29/30/32。
- **篇幅**：4000–5500 汉字（全文 CJK 计数）；章结构照项目 1 成稿（要做的东西 / 验收标准 / 设计取舍 / 逐块实现 / 报错现场 / 测试 / 小结 / 练习 2–3 题带答案 / 延伸阅读）。
- **DoD**：§8 —— 独占 `src/project5-engineering.md` + `examples/release-drill/**` + `.github/workflows/**`（按你的方案）+ 本票；禁 `git`；自验：`CARGO_TARGET_DIR=.scratch/tmp-61/target cargo test --manifest-path examples/Cargo.toml -p release-drill` 全绿 + `cargo package`（私有 target）通过 + `mdbook test -c project5-engineering.md` exit 0；探针 `.scratch/tmp-61/`；收尾 `Status: claimed` + `## Comments`。

## Comments
