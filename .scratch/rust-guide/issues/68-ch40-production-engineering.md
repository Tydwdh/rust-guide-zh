# 68 第 40 章 生产工程

Type: task
Status: open

## Question

写第六篇第七章：**生产工程**（素材 = D7：rustc `exploit-mitigations`、cargo `registries`、unstable-book `build-std`）。

- **范围**：panic 策略（`panic = "unwind"` vs `"abort"`：真跑两种 release 构建对照）；动态库与 ABI 稳定（`cdylib` 真跑 + 「Rust ABI 不稳定」的工程含义，回指 ch26 结论）；交叉编译（**本机已装** `thumbv7m-none-eabi`/`thumbv7em-none-eabihf`/android 系 target——真跑 `cargo build --target` 粘输出）；`build-std`（nightly 标注，只文档化）；供应链与 registry（`[registries]` 配置、私有 registry 文档化、`cargo vendor` 方向说明）。
- **要求**：真跑与「文档化 + 标注」严格区分；thumb 目标用 `#![no_std]` 库示例（`snippets/ch40/` 独立 crate）。
- **边界**：FFI/ABI → ch26 回指；profile → ch29；workspace/features → ch30。
- **篇幅**：4000–5500 汉字；八节模板；练习 2–3 题带答案。
- **DoD**：§8 —— 独占 `src/ch40-production-engineering.md` + `snippets/ch40/` + 本票；禁 `git`/禁 build；自验 `mdbook test -c ch40-production-engineering.md` exit 0 + 交叉编译真跑；探针 `.scratch/tmp-68/`；收尾 `Status: claimed` + `## Comments`。

## Comments
