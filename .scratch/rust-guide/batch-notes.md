# 基础篇批量写作 · 协调台账（票 09–21）

> 协调者维护；写作者只读不写。滚动更新 —— 各票交付 / 裁定 / 新术语 / 图示需求 / 遗留。

## 状态：已落地 10/13（前言、第 1、2、5、6、7、8、9、10、11 章）；在写 3 票（第 3、4 章 + 项目 1）

### 票 09 前言 — 定稿（两轮退修完成）
- 1202 汉字（四种计数口径全 ≤1500）；退修：跳读路径消歧、隐藏行说明按实测更正、修辞精简。
- 裁定：`unsafe` 首现不加括注 / 「包管理器」不算术语首现 / Reference 定位语保持 / 对照角保持可选 / 不与 04 逐字对齐。
- 自验：`mdbook test -c preface.md` exit 0（两次）+ 负向对照 101。新术语：无。图示需求：无。

### 票 10 第 1 章 起步 — 初稿到（2789 汉字，exit 0）
- 实测：工具链三件套版本、`cargo new` 全流程、check/run/build/--release、`printlin!` 报错全文、E0601、rustc 单文件。
- 术语（待并表）：**profile（构建配置）**（建议入表；_Avoid_: 配置文件）。
- 图示（转 08）：cargo 编排流程（横向管线）、rustup/cargo/rustc 三层分工、`target/debug` 树 + 可删缓存/Cargo.lock 标注、check/build/run 产出差异。
- 遗留：无阻塞项（耗时数字标注为量级参考）。

### 票 14 第 5 章 结构体、枚举与模式匹配 — 初稿到（3969 汉字，exit 0；两次防静默跳过对照）
- 报错实测：E0004 / E0382 partial move / E0609 / E0204（初稿误写别处，已按实测改正）。
- 术语（待并表，16 条）：结构体（struct）、枚举（enum）、变体（variant）、元组结构体、单元结构体、字段（field）、关联函数、方法（method）、解构（destructuring）、穷尽性、匹配守卫、或模式、部分移动、匹配人体工程学、代数数据类型（ADT）、`#[derive]` 保留英文。
- 图示（转 08）：「bool 汤 vs enum」、`self` 三形态 ↔ 所有权/借用、match 自上而下 + 穷尽性流程、模式速查表。
- 主动裁掉：2024 `if let` 临时值作用域、`repr`/判别值（第 25 章）、`..rest` 更新语法/Display/Default/non_exhaustive。

### 票 15 第 6 章 集合与字符串 — 初稿到（3081 汉字，exit 0；负向对照）
- 亮点：票面报错描述与 1.98 实测不符（「cannot index into a value of type String」实为 E0277 `the type str cannot be indexed by {integer}`），按实测写。
- 术语（待并表）：集合、哈希映射、容量（capacity）、重新分配、元素、Unicode 标量值、字素簇、entry API（保留英文）。
- 图示（转 08）：`"你好"` 字节布局 + 边界 + `&s[0..1]` 切点、Vec 容量增长与搬移（4→8→16）、`str`/`&str`/`String` 与 `[T]`/`&[T]`/`Vec<T>` 并排。
- 遗留：Vec 越界 panic 未贴（控篇幅，接受）。

### 票 16 第 7 章 错误处理 — 初稿到（3368 汉字，exit 0；18 doctest + no_run 负向对照）
- 裁定：`Result`/`Option`/`unwrap`/`expect`/`ok_or`/`source` 不立条目（API 名按原名保留）。
- 术语（待并表）：展开（unwind，与「宏展开」区分）、错误链（error chain）、错误传播（error propagation）。
- 图示（转 08）：`?` 语义流程、`source()` 错误链指向、unwind-清理 vs abort 对比。

### 票 11 第 2 章 读懂编译器 — 初稿到（3544 汉字，exit 0；22 doctest + 负向对照）
- 五家族全落（各含最小复现 + 1.98 实测 + 修法 + 正向对照实跑）：E0308/E0425/E0599/E0382/E0502；warning 三例；连带错误；真实项目式 E0382（`Vec`+`for`+`.into_iter()` note）；`cargo check` 成败两态（含「不产出 exe」实测）。
- 术语（待并表）：诊断（diagnostic）、最小复现（minimal reproduction）、移动（move）、错误码（error code）、连带错误（cascading errors）；span / lint 保留英文。
- 图示（转 08）：诊断解剖图（现 text）、「五步读报错」流程、E0502 span 时间轴（第 4 章也需要）。
- 发现：`compile_fail,E····` nightly-only（stable 1.98 静默不校验）→ 已写进 WRITING §7。

### 票 18 第 9 章 泛型与 trait 入门 — 初稿到（4169 汉字，exit 0；负向对照）
- 「语言对照角」已落实（C++/Java/Python）✓；6 个完整 snippet（largest / pair / summary / std_traits / from_into / newtype）。
- 实测：6 段程序输出；**单态化 LLVM IR 实验**（三个 `show` 实例：ptr+i64 / double / i32）；5 处报错（E0369/E0107/E0277/E0599/E0117）；弃用一则「局部 static 跨实例共享」反直觉实验（未入正文）。
- 跨章自组织 ✓：与 Ch04/Ch05/Ch10 相互确认回指与预告（derive、Copy/Clone、单态化回引），无循环引用。
- 术语（待并表）：单态化（monomorphization）、孤儿规则（orphan rule）、trait bound、不透明类型（opaque type）、newtype；derive 保留英文。（「trait bound」中译并表时查社区表定：约束 vs 边界。）
- 提请裁决（已裁）：**链接不钉版本** —— 全文用稳定入口；ch05/ch07 的 `1.98.0` 钉版链接由协调者统一扫改。WRITING §5 已补规则。

### 票 20 第 11 章 测试与调试 — 初稿到（3119 汉字，exit 0；可跑 crate 真跑）
- snippet：`snippets/ch11/`（`Cargo.toml` + `src/lib.rs` + `tests/integration.rs`，完整可跑 crate）。
- 实测：失败态/修复态 `cargo test`、子串过滤、`#[should_panic]` 两例、`dbg!` 捕获行为、`-- --help`。
- **重大发现（已写进 WRITING §1/§7）**：`#[test]` 与 `#[cfg(test)] mod tests` 的块体在 mdbook test 下**不编译**（本体类型错误仍 exit 0 假绿）→ 测试类演示必须配可跑 crate + `cargo test` 真跑证据。协调者两种形态均已复现。
- 术语（待并表）：单元测试（unit test）、集成测试（integration test）、断言（assertion）、测试运行器（test runner）、测试框架（test harness）、文档测试（doctest）。
- 图示（转 08）：cargo test 三段目标关系、复现→最小化→假设→验证循环、`cfg(test)` 在 build/test 下入不入产物。
- 其他：`--nocapture` 在 1.98 可用但不在 `--help`（按实测措辞）；断点调试未实测（已如实标注）。

### 票 17 第 8 章 模块、包与依赖 — 初稿到（3541 汉字，exit 0；防静默跳过对照）
- snippet：`snippets/ch08/shop/`（`main.rs` + `catalog.rs` + `catalog/items.rs`，多文件布局；正文以 `{{#rustdoc_include}}` 包在 `rust,ignore` 中 —— **收尾 build 时须确认渲染展开**）。
- 实测：E0603/E0432/E0433/E0583、`cargo new`、`cargo add itoa --offline`、`cargo build/run --offline`、Cargo.lock 真身。
- 环境注：本机无外网 → 依赖实操走 `--offline`（正文已注明，未声称联网行为）。
- 术语（待并表）：模块（module）、可见性（visibility）、命名空间（namespace）、重导出（re-export）、依赖（dependency）、工作区（workspace）；features 保留英文（第 30 章定）。
- 图示（转 08）：模块树 ↔ 文件树、crate 边界 + 私有链断点（E0603 图景）。

### 票 19 第 10 章 迭代器与闭包 — 初稿到（4501 汉字，exit 0；对照角 ✓）
- snippet：`snippets/ch10/log_stats.rs`（47 行；命令式 vs 链式 + `assert_eq!` 钉一致）。
- 实测：三种捕获、`Fn`/`FnMut`/`FnOnce`、惰性分步、适配器/消费者输出、未消费块的两个真实警告；报错 E0502/E0499/E0283/E0382/E0373（各配已验证修法）。
- 裁定（遗留三项全按稿）：47 行示例保留（15–30 行是软建议）；4501 汉字 ≈ 上限，接受；HashMap 不打印、零成本不跑基准 ✓。
- 术语（待并表）：适配器（adapter）、消费者（consumer）、惰性求值（lazy evaluation）、拉/推模型；对照角用视图（view）、流（stream）。
- 图示（转 08）：`Fn` 系包含关系、迭代器链套娃类型、惰性管线对比。

## 事件与勘误

- **编译产物事故（已处置）**：ch04 的 `ws.exe/ws.pdb`、ch08 的 `shop17.exe/shop17.pdb` 落进 `snippets/` → 已删；`.gitignore` 增 `*.exe`/`*.pdb`；已通知两票（输出一律进 `.scratch/tmp-NN/`，snippets 只放源码）。
- **ch07 单章测试瞬时 UTF-8 报错**：复跑 exit 0（bg_181）—— 判为批次中途读到半写文件/二进制混入的瞬时态，非内容缺陷；最终链以收尾态为准。
- **「UTF-8 报错」结案**：全仓库非 UTF-8 文件 = 0、所有 include 目标合法；两次报错均为写作者正在写文件时的瞬时读（Python 捕获复跑 ch02/ch07/ch09 全绿）—— 最终链在写作者空闲时跑。
- **隐藏行机制（实测 mdbook 0.5.4）**：`book.js` 给含 `.boring` 的块加 `hide-boring`（默认收起）+ 眼睛按钮展开；WRITING.md §1 已补说明。
- **linkcheck2 已启用**（book.toml，follow-web-links=false / require-md-extension=true / warning-policy=warn）：clean build exit 0；HTML 输出改为 `book/html/`；README 与 WRITING §3/§7 已同步。
- **跨章承诺**：前言承诺第 9、10 章「C++/Java/Python 对照」→ 已转达两章落实。

## 下一步（协调者）

- [ ] 其余 8 票落地即验；批次收尾跑全量链（build 扫 ERROR/WARN + `mdbook test` + `cargo test --manifest-path examples/Cargo.toml`）。
- [ ] 交叉审校：计划 4 个 reviewer（每 3–4 章，含组内跨章一致性）+ 全局术语/章引用一致性 grep（术语并表前跑）。
- [ ] 术语并表 `GLOSSARY.md`；图示需求汇总 → 08 票定「图示体系」。
- [ ] 收尾提交（单章文件 + 票 + snippets + examples），地图/README/08 同步。
