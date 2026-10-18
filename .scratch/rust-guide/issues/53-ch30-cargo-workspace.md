# 53 第 30 章 Cargo 与 workspace 工程化

Type: task
Status: open

## Question

写第五篇第四章：**Cargo 与 workspace 工程化**。

- **范围**：features（定义、`dep:` 语法、可选依赖、默认 feature、feature 统一与 resolver）；workspace（members、`workspace.dependencies`/`package`/`lints` 继承、共享 `Cargo.lock`/`target`）；`[lints]` 表与 lint 层级（衔接到第 32 章）；`rust-version`（MSRV）与 MSRV 感知解析（resolver `"3"`）；build script（`cargo:rerun-if-changed`、`cargo::rustc-cfg` 的 1.77 新写法与旧写法之别——核 Cargo Book）；CI 模板（GitHub Actions 最小可用）；发布与 yank（说明性）。
- **素材**：`research/02` 的 I20 行（`cargo/reference/features.html`、`workspaces.html`、`build-scripts.html`、`lints.html`、`cargo/guide/continuous-integration.html`）；**本仓 `examples/` 是现成的真实 workspace 示例（只读引用，勿改）**；探针可在 `.scratch/tmp-53/` 建独立 workspace 实测。
- **边界**：lint 细节 → 第 32 章；Edition 迁移 → 第 33 章；性能档位 → 第 29 章（回指）；宏 → 第 27/28 章（不展开）。
- **篇幅**：4000–5500 汉字；八节模板；练习 2–3 题带答案；能真跑的代码必须真跑（探针 workspace），说明性的用 `ignore` 并注明原因。
- **DoD**：§8 —— 独占 `src/ch30-cargo-workspace.md` + `snippets/ch30/` + 本票；禁 `git`/禁 build；自验 `mdbook test -c ch30-cargo-workspace.md` 绿；探针 `.scratch/tmp-53/`（独立 workspace 的全套 Cargo.toml 放这里）；收尾 `claimed` + `## Comments`。

## Comments
