# 09 前言：读者须知与使用约定

Type: task
Status: claimed
Blocked by: 06

## Question

写「前言」（替换 `src/preface.md` 的占位）：读者须知、前置假设、怎么用这本书、示例与工具链约定。

- 素材：04 的定位声明（起点线、承诺主轴=理解、诊断作方法）；本书约定（edition 2024、示例怎么跑、去哪儿找官方资料）。
- 必须体现：起点线说人话（会一门主流语言、不假定会 Rust）；讲法四条（心智模型先行 / 诊断贯穿 / 示例全可编译 / 自包含 + 章末延伸阅读）。
- 不做：点名或对标任何现有中文资源（04 决定）。
- DoD：地图 Notes 的自证要求 + 06 的规范。

## Comments

- 2026-10-18 初稿：文件 `src/preface.md`（整页替换占位；H1 `# 前言` + 6 个自然小节：这本书写给谁 / 这本书承诺什么 / 讲法 / 怎么用这本书 / 示例怎么跑，报错怎么看 / 一手资料在哪；正文 1258 汉字，不含代码块）。
  自验：① `mdbook test -c preface.md` → `INFO Testing chapter '前言': "preface.md"`，exit 0；② 负向对照（确认门禁非空转）：临时在 `rust` 块里注入 `let bad: u32 = "not a number";` → `test result: FAILED. 1 passed; 1 failed`、`ERROR One or more tests failed`、exit 101（1 passed 即 `compile_fail` 块，处理器真的在编译两种块），随后还原，`md5sum` 与备份一致，再跑 exit 0；③ 章内 `compile_fail` 块与 `rust` 块各 1 个、`console`/`text` 块各 1 个（非 Rust 片段全部标了语言，无裸栅栏）。
  实测输出（1.98.0，均在 `.scratch/tmp-09/` 跑完后删除）：`rustc --version` → `rustc 1.98.0 (88d9e12ae 2026-08-18)`；E0382 样例（`moved.rs`）→ `error[E0382]: borrow of moved value: `s`` / `--> moved.rs:4:16` / `value moved here` / `value borrowed here after move`；可运行样例 → `你好, Rust`；`edition-guide/rust-2024/index.html`（本机快照）→ `RFC #3501 / Release version 1.85.0`；`book/index.html` → 该书假设 1.90.0 / edition 2024；`reference/index.html` → `the primary reference for the Rust programming language`；`rustup doc --book/--std/--nomicon/--reference/--cargo/--edition-guide/--error-codes` 均存在，`rustup doc --path --book` 指向工具链离线副本。
  遗留（请协调者裁定）：① `unsafe` 在前言表格里首现（作为关键字、写作 `unsafe Rust`），未加「首现说明」——按 GLOSSARY 它本就保留英文，我判断不需要括注；② 「包管理器」里含「包」，未按 GLOSSARY 的「包（package）」加英文括注（判为固定搭配 package manager，非同一条目）；③ Reference 的「不是正式规范」这类定位语没写：在本机 1.98 快照 `reference/index.html` 里没找到该表述，只核到「primary reference…」，故不写；需要的话可补；④ 未承诺「每章都有语言对照角」（`WRITING.md` §4 是可选），前言只写「概念跨语言差异大的地方会加」；⑤ 「本书承诺什么」三条与「讲法」四条是按 04 定位声明与正文要求重述的，措辞是我拟的，未与 04 逐字对齐。
  新术语：无新增。前言用到的术语（所有权/借用/生命周期/泛型/迭代器/闭包/零成本抽象/未定义行为/trait）都在 GLOSSARY 里；按「首现写「中文（English）」」在正文加了英文括注（`所有权（ownership）`、`借用（borrow）`、`生命周期（lifetime）`、`泛型（generics）`、`迭代器（iterator）`、`闭包（closure）`、`零成本抽象（zero-cost abstraction）`、`未定义行为（UB）`、`trait（行为契约）`）。
  邻章衔接：前言点到的章号 = 第 1 章（工具链）、第 2 章（报错解剖）、第 4 章（所有权心智模型）、第 8 章（Cargo）、第 9/10 章（泛型与 trait、迭代器与闭包）、第二篇、第六篇、项目 1；这些章若改号改题，前言这几处要同步。
  图示需求：无（本章未画图）。
  验收自查：全文无 emoji、无营销腔、一律「你」直呼；未点名或对标任何现有中文资源（grep 过 圣经/菜鸟/中文版/知乎/B站 等关键词，零命中；表格只列官方英文文档与 `doc.rust-lang.org` 域名）。
