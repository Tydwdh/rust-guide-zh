# Rust 指南（暂名）

从零起步的中文 Rust 指南：假定读者会一门主流语言，**不假定会 Rust** —— 以「理解 Rust 的设计」为主线、诊断训练贯穿全书，用 [mdBook](https://rust-lang.github.io/mdBook/) 写。

写作路线 —— 地图与全部决策票 —— 在 `.scratch/rust-guide/`：先读 `map.md`。

## 本地预览与校验

需要 mdBook 0.5.4 与 mdbook-linkcheck2（`cargo install mdbook --version 0.5.4 --locked`、`cargo install mdbook-linkcheck2 --locked`）。

```bash
mdbook serve --open   # 本地预览（默认 http://localhost:3000，改动自动重建）
mdbook build          # 构建到 book/html/，并跑书内链接检查（断链 → 构建失败）
mdbook test           # 用 rustdoc 校验书里的代码块（全量；`-c <章>` 只测一章）
```

## 目录结构

| 路径 | 用途 |
| --- | --- |
| `src/` | 书籍源码（`SUMMARY.md` 即目录） |
| `examples/` | 实战项目的示例 crate（workspace；约定见 `WRITING.md` §1） |
| `snippets/` | 章节示例片段（`mdbook test` 编译；章内 `{{#rustdoc_include}}` 引用） |
| `.scratch/rust-guide/` | wayfinder 地图与决策票 |
| `WRITING.md` | 写作规范（章节模板、校验链、DoD、风格） |
| `GLOSSARY.md` | 术语表（写作一致性依据，将来派生书末附录） |
| `book/` | 构建产物（已 gitignore） |

## 状态

基础篇（前言 + 第 1–11 章 + 项目 1）与进阶篇（第 12–18 章）初稿完成：两批都已过交叉审校与逐票修回，全量校验链全绿（`mdbook build` 0 ERROR/0 警告、`mdbook test` 20 章 exit 0、examples 19+2 passed）；基础篇 12 章配有内嵌 SVG 图示（共 36 张）。`SUMMARY.md` 已挂到第 18 章（其余各篇为草稿条目，未接链接）。写作规范 / 校验链见 `WRITING.md`，术语见 `GLOSSARY.md`。
