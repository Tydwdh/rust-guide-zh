# 11 第 2 章 读懂编译器：报错的解剖与修复路径

Type: task
Status: claimed
Blocked by: 06

## Question

写基础篇第 2 章：把「读报错」当第一等技能 —— error/note/help 的结构、最小复现、从报错回推心智模型、常见错误类别的定位路径。

- 本章是「诊断训练贯穿」的开场章，同时承担所有权引子（预告「为什么这段代码不能编译」，为第 4 章埋动机）。
- 素材：01 §2.2（读者卡点：靠规则背诵而非心智模型）；示例用真实编译器输出（本机 1.98）。
- 章末 2–3 道理解题（带答案）。DoD 见地图 Notes + 06。

## Comments

- 2026-10-18 初稿：文件 `src/ch02-reading-the-compiler.md`（572 行 / 正文 3544 汉字；22 个 Rust 块 = 11 个正向可编译 + 11 个 `rust,compile_fail`）；配套 `snippets/ch02/report.rs`（E0599 修好后的完整程序，正文用 `{{#rustdoc_include}}` 引入）。自验：`mdbook test -c ch02-reading-the-compiler.md` → exit 0（22 个 doctest 全绿）；另做门禁负向验证：临时插入一个「能编译却标 `compile_fail`」的块 → FAILED、exit 101，还原后 exit 0。环境：`rustc/rustdoc 1.98.0 (88d9e12ae)`、`mdbook v0.5.4`、edition 2024、Windows。
  - 实测输出清单（全部真跑后粘进正文；`rustc --edition 2024`；文件统一命名 `main.rs`）：① E0308/E0425/E0599/E0382/E0502 五条最小复现（含 E0599 的 `the method is available for \`Stdout\` here` 与两条 `help`、E0382 的 `does not implement the Copy trait` 注解、E0502 的三条 span 证据链）；② warning 三例：unused variable（`_total` 建议）/ unused import / dead_code，`rustc` exit 0；③ 连带错误：漏 `use std::io::Write;` → 两条 E0599（5:7、7:7）+ `aborting due to 2 previous errors`；④「真实项目式」E0382（`Vec<i32>` + `for` + `.into_iter()` note + `&scores` 建议）与其 5 行最小复现；⑤ `cargo check` 出错（exit 101，末行 `error: could not compile \`checkdemo\` (bin "checkdemo") due to 1 previous error`）与修好后 `Finished \`dev\` profile [unoptimized + debuginfo] target(s) in 0.04s`；并实测 `cargo check` 不产出 `target/debug/*.exe`、`cargo build` 才产出。
  - 正向对照（每个报错家族都有，逐条实跑记录输出）：E0308 修法 → `42`；E0425 修法 → `3`；E0599 修法（`snippets/ch02/report.rs`）→ `report: 42`；E0382 修法①`clone()` → `hi hi`、修法②`len()` → `hi 长度为 2`；E0502 修法 → `1 [1, 2, 3, 4]`；scores 修法（`&scores`）→ `总分 240，一共 3 项`。
  - 新术语（GLOSSARY 缺；请协调者统一并表）：`diagnostic | 诊断`；`minimal reproduction | 最小复现`；`move | 移动`；`span | 建议保留英文（正文首现已给一句说明）`；`lint | 建议保留英文`；`error code | 错误码`；`cascading errors | 连带错误`。表内已有并已按表使用：所有权（ownership）、借用（borrow）、借用检查器（borrow checker）、trait（保留英文 + 首现一句说明）。
  - 邻章衔接：开头「在第 1 章你跑通了第一个程序」衔接第 1 章（建议第 1 章顺带提一句 `cargo check`）；第 3 章留了类型转换的指针；E0382/E0502 两支明确写「第 4 章展开」，并以「报错是设计反馈」收束铺路第 4 章；E0599 → 第 9 章 trait 设计；运行期错误 → 第 7 章；lint 工程 → 第 32 章；`std::io` 与导入的语境 → 第 8 章。项目 1（CLI）可直接复用本章的五步流程与 E0599 例子（`std::io::Write` 正是 CLI 输出会用到的 trait）。
  - 图示需求：①「一条诊断的解剖」现用 `text` 块画（①–⑦ 标注），08 票定图示体系后可升级为正式图；②想要一张「五步读报错流程」流程图（现为有序列表）；③想要一张 E0502 的「span 时间轴」示意（共享借用 ↔ 可变借用重叠，第 4 章也需要）。
  - 遗留 / 待协调者确认：(a) `mdbook test` 成功时不打印用例清单（只打 INFO 行），故用「临时插入应失败块」的方式枚举用例并证明门禁有效；(b) 实测 `{{#rustdoc_include}}` 的内容**会**被 `mdbook test` 编译（该块作为普通 doctest 通过），与 `WRITING.md` §1 括注「`{{#include}}`（不参与 `mdbook test`）」不符——本稿不受影响（`report.rs` 可编译），但规范宜更正；(c) `compile_fail,E0308` 形式为 nightly-only（rustdoc Unstable Book 原文：stable 上会「被当作纯文本」），stable 1.98 实测**不校验**错误码（故意写错码仍判 ok）→ 本章统一用稳定的 `rust,compile_fail`，错误码只出现在正文与输出中；(d) `cargo check` 输出中段我按「与前面 `rustc` 输出一致」呈现并注明「路径已省略」（本机捕获管道对同一段有一次 `||` 渲染噪声，未逐字粘贴中段）；(e) 首次 `cargo check` 因并行批次抢占 package cache 锁等待约 100s（项目无依赖），流程本身无碍。
- 2026-10-18 复审修订（RwCh02 findings 1–5，全 minor，均已改，只动本章文件）：① span 计数统一：`:264` 由「剩下两处相关位置」改为「三处相关位置（移动 2 处 + 使用 1 处）」，与 `:236` 的「三个相关位置」及输出块中的 3 行源码位置一致；② warning 读法改正：不再说三条「读法一致、`help:` 直接给出修法」，改为「骨架一样（首行 + 末尾 `= note: #[warn(···)]`），中间那段不一定有」——只有例一（unused variable）带内联 `help:`，例二/例三（unused import、dead_code）需自行判断处理；③ `note` 例子改用真实 note 行：由「`String` 没有实现 `Copy`」（那是 ④ 的 span 标签文字）改为 ``note: `into_iter` takes ownership of the receiver `self` ``；④ 术语首现顺序前移：`lint` 括注移到 `:18`（首现处），`移动（move）`括注移到 `:236`（首现处，`:422` 已去重为「移动」），`trait` 说明移到 ③ 标题后的第一句 `:358`（标题无法带括注，已是最靠近首现的位置）；⑤ `:276` 改引英文原文 "compile the packages without performing the final step of code generation, which is faster than running `cargo build`" 并括号给大意，不再把中文译文称作「原话」。修订后复跑：`mdbook test -c ch02-reading-the-compiler.md` → **exit 0**；正文 3648 汉字；22 个 Rust 块（11 正向 + 11 `compile_fail`）不变。
