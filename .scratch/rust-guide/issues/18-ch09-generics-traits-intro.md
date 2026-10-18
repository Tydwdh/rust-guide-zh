# 18 第 9 章 泛型与 trait 入门

Type: task
Status: claimed
Blocked by: 06

## Question

写基础篇第 9 章：泛型函数/结构体、trait 定义与实现、trait bound、常用标准 trait（Debug/Clone/PartialEq/Display）、derive。

- 心智模型：泛型 = 编译期替换（单态化初识）；trait = 行为契约。
- 前置：第 4 章（所有权）、第 5 章（枚举与模式匹配）。
- 章末 2–3 道理解题（带答案）。DoD 见地图 Notes + 06。

## Comments

- 2026-10-18 初稿：文件 `src/ch09-generics-traits-intro.md`（4137 CJK 字；含代码/输出总字符 16818）+ `snippets/ch09/` 六个完整程序（largest.rs / pair.rs / summary.rs / std_traits.rs / from_into.rs / newtype.rs）；自验 `mdbook test -c ch09-generics-traits-intro.md` → exit 0（本机 mdbook 0.5.4；成功时只打印一行 INFO，不列测试数），harness 有效性另做对照实验：往 largest.rs 尾部塞一行 `this is not rust` → 同一命令 exit 101 且报告 `动手看看 (line 301) ... FAILED`、`ERROR One or more tests failed`，已还原（该实验同时证明 `{{#rustdoc_include}}` 的片段会被展开并编译，见遗留）；构建链只跑这一条，未跑 `mdbook build`/`mdbook serve`（批次协议）。实测输出（全部 rustc 1.98 `--edition 2024` 编译后真跑）：largest → `整数最大值: 100` / `字符最大值: y` / `字典序最大: zebra`；pair → `3 与 7 中较大的是 7` / `apple 与 pear 中较大的是 pear`；summary → `新内容：《RISC-V 的十年》，作者 林一` / `新内容：（qiao 的内容摘要待补充）`；std_traits → `1.2 < 1.10: true` / `1.2 和 1.2 都还在` / 两行 Debug（`{:#?}` 多行）；from_into → `25 °C = 77.0 °F` / `32.0 °F` / `212.0 °F`；newtype → `3 个词：alpha / beta / gamma`；单态化探针 `show.rs` → `--emit=llvm-ir -C opt-level=0` 产物里三个 `show` 定义（`Re`/`d`/`m` 对应 `&str`/`f64`/`u32`，参数类型分别为 ptr+i64 / double / i32）；章内「边界怎么写」块实跑 → `第一次：hello` / `第二次：hello` / `共 3 个，最大的是 9` / `7` / `1.2`；真实报错 5 个（compile_fail 块全部被测过）：E0369（`<T>` 无边界用 `>`，help 指向 `PartialOrd`）、E0107（`impl Trait` 参数位置不能显式给泛型实参）、E0277（`Point` 缺 `Display`，含 help+note 链路读法）、E0599（trait 未在作用域，help 给出 `use crate::media::Summary;`）、E0117（孤儿规则）；另有两条仅用于措辞、未进正文的验证：泛型函数里的局部 `static` 在各实例间共享（输出 0/1/2，与常见说法相反，故弃用该实验）、`b::<u8>(1)` 显式泛型实参合法。遗留：① 回指「第 4 章讲移动时见过 Copy/Clone」「第 5 章方法定义在 impl 块里」，若这两章最终内容未覆盖请协调者改句或返我改；② WRITING §1 说 include「不参与 mdbook test」与实测不符（0.5.4 下 rustdoc_include 会被展开并编译，塞坏片段实验为证）——按协议未改 WRITING.md，请 06 裁决；③ 报错文本块的行号（如 `--> e0277.rs:12:10`）来自探针文件而非章内块布局，正文已注明「行号随代码布局变化」；④ `AFIT` 未点名（`async` 属第 21 章，本章只点 RPIT+生命周期捕获）；⑤ 篇幅口径：CJK 4137，若按含代码总字符算为 16818。新术语（GLOSSARY 缺，请并表）：`monomorphization | 单态化`（首现「单态化（monomorphization）」；_Avoid_ 模板展开）、`orphan rule | 孤儿规则`、`newtype | newtype`（建议保留英文，E0117 帮助文本作 “new type”）、`trait bound | 边界`（本章统一「边界」，需表内确认；_Avoid_ 与「约束」混用）、`derive | derive`（建议保留英文；_Avoid_ 派生宏——那是 proc macro 的一类）、`opaque type | 不透明类型`（本章只在 E0308 说明里出现，正式定义留给第 14 章）；`trait 对象`/`关联类型` 留给 12/13 票。邻章衔接：本章引用了第 2（读报错）、4、5、7、12（关联类型/trait 设计/孤儿规则细节）、13（dyn、vtable、`Box<dyn>`）、14（RPIT 捕获）、15（const 泛型与 GAT）、17（Deref）、29（体积）；给 12/13/14/15 票的直接接口是「`impl Trait` 参数位置 = 匿名泛型参数、返回位置 = opaque 且只能一种具体类型」；给第 10 章票：「零成本抽象」「单态化」本章已首现，别再当首现（本章未引用第 10 章）；给前言：语言对照角已按承诺落实（泛型 + trait 各一组，C++/Java/Python）。图示需求：① 单态化处想要「一份泛型源码 → N 份实例（u32/f64/&str）」的示意，本批按规范用 LLVM IR 真实输出 + console 块替代、未画图；② trait 三角色（定义/实现/使用）处想要一张简图（现为三条列表）；两处都未引入 mermaid/HTML-SVG。
- 2026-10-18 追加（跨章回指协调后）：① 回指已核实——Ch04Ownership 确认第 4 章 §4.2 就是讲 `Copy`/`Clone`（赋值/传参默认移动，`Copy` 赋值后原变量仍可用，`String`/`Vec`/`Box` 要复制得 `.clone()`），本章措辞照旧成立；Ch05StructsEnums 确认第 5 章 §5.2「impl：方法与关联函数」、§5.3「derive：按需索取常用能力」已讲，并要求本章交付它预告的两点。据此把「标准库常用 trait」一节的 derive 段改成「第 5 章已经见过」，并显式补齐：`#[derive(Debug)]` 展开成 `impl Debug for Version`、「手写 impl 与 derive 生成的 impl 在语言里地位完全相同」、以及第 5 章预告的「`Display` 为什么不能 derive」（原因：怎么显示是人的决定，不是字段的机械组合）。改后 `mdbook test -c ch09-generics-traits-intro.md` → exit 0。CJK 4169（仍在 3500–4500）。② 链接风格不统一，供 06/收尾裁决：ch05（4 处）、ch07（6 处）用钉版本的 `doc.rust-lang.org/1.98.0/…`，其余各章（含本章）用不钉版本的 `doc.rust-lang.org/…`；本章按多数派不钉版本，若要统一钉版本请一并改。
