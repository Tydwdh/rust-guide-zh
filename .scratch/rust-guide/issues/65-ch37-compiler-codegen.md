# 65 第 37 章 编译器与代码生成

Type: task
Status: open

## Question

写第六篇第四章：**编译器与代码生成**（素材 = D4：rustc book `codegen-options`、reference `inline-assembly`、`linker-plugin-lto`）。

- **范围**：编译流水线鸟瞰（HIR→MIR→LLVM IR→机器码）；MIR 观察（`--emit=mir` stable 真跑；`-Zunpretty` 标 nightly）；单态化与代码膨胀（`--emit=llvm-ir`/asm 对照、符号重复观察）；`-C` 常用选项（opt-level/debuginfo/codegen-units/overflow-checks——讲「选项→代码」一条线，与 ch29 的 profile 层不重复）；inline asm（`core::arch::asm!` stable 最小例真跑）；naked functions（nightly 标注）；LTO 一句带过（回指 ch29 已验证结论）。
- **要求**：dump 输出真跑真粘（注明「本机 x86-64、随版本浮动」）；不得给未跑的输出。
- **边界**：性能方法学/PGO → ch29 回指；布局 → ch25；UB 与优化 → ch23。
- **篇幅**：4000–5500 汉字；八节模板；练习 2–3 题带答案；按 §9 内嵌 1 张流水线 SVG（可选）。
- **DoD**：§8 —— 独占 `src/ch37-compiler-codegen.md` + `snippets/ch37/` + 本票；禁 `git`/禁 build；自验 `mdbook test -c ch37-compiler-codegen.md` exit 0；探针 `.scratch/tmp-65/`；收尾 `Status: claimed` + `## Comments`。

## Comments
