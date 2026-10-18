# 基础篇批量写作 · 协调台账（票 09–21）

> 协调者维护；写作者只读不写。滚动更新 —— 各票交付 / 裁定 / 新术语 / 图示需求 / 遗留。

## 状态：已落地 12/13（前言、第 1–11 章全覆盖，缺项目 1）；在写 1 票（项目 1）

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

### 票 12 第 3 章 语言基础 — 初稿到（3624 汉字，exit 0；展开 include 后 rustdoc 27 块全绿）
- snippet：`snippets/ch03/tour.rs`（70 行）、`overflow.rs`（6 行）。
- 实测：debug 溢出 panic（101）vs release 环绕 `-128`；编译期 `arithmetic_overflow`（同程序 `cargo check` 退出 0）；`as`/`try_into`；`{1+1;}` 的 unit；`wrapping/saturating/checked_add`；char 4 字节与 `from_u32(0xD800)`；8 组 cargo 报错原文。
- **include 真执行的新证据**：故意 panic 的程序经 include 使 `mdbook test -c` FAILED；标 `rust,should_panic` 后通过 → 已并入 WRITING §1 的规则措辞。
- 术语（待并表）：绑定（binding）、遮蔽（shadowing）、单元类型（unit type）、尾表达式（tail expression）、整数溢出（integer overflow）、文档注释（doc comment）、类型标注/推断（annotation / inference）、可变绑定（mutable binding）。
- 图示（转 08）：表达式导向 vs 语句导向、溢出 debug/release 决策树。
- 遗留：E0571/E0317/运行期越界一句带过（已实测）；遮蔽的精确 drop 语义留第 4 章。

### 票 13 第 4 章 所有权、借用与切片 — 初稿到（核心章 5401 汉字，exit 0；include 覆盖实证）
- snippet：`snippets/ch04/word_stats.rs`（唯一文件；目录内无编译产物 ✓）。
- 实测：E0382/E0499/E0502/E0515/E0106/E0204 + 非字符边界 panic（101）；尺寸表（String24 / Vec24 / &String8 / &Vec8 / &str16 / &[u8]16 / `Box<str>`16 / &[u8;4]8）；切片地址偏移 6；drop 顺序；`v.push(v.len())` 两阶段借用可编译。
- **重要发现（已写进 WRITING §1）**：`{{#rustdoc_include}}` 指令必须写在 ```rust 围栏**内**；裸写 → 当段落渲染且完全不被测试（本章已修；作者核查确认其余 11 章 + 项目章围栏全部正确）。
- 术语（待并表）：移动（move）、克隆（clone）、析构（drop）、动态大小类型（DST）、胖指针（fat pointer）、瘦指针（thin pointer）、解引用强转（deref coercion）、借用区间（borrow region）、别名规则（aliasing rules）、悬垂引用（dangling reference）、重复释放（double free）；`Copy`/`Clone`/`Drop` 保留英文。
- 图示（转 08）：① §4.5 借用区间时间轴（作者建议「最值得上 mermaid」）；② §4.7 三型内存布局对照；③ §4.3「move = 释放责任交接」状态转移。
- 主动裁掉：research/02 §5.2#4（2024 临时值 drop 影响）留第 5/7 章（实则第 33 章有专章，收尾时确认由它覆盖）；`for x in v` 隐式移动只一句。

## 事件与勘误

- **编译产物事故（已处置）**：ch04 的 `ws.exe/ws.pdb`、ch08 的 `shop17.exe/shop17.pdb` 落进 `snippets/` → 已删；`.gitignore` 增 `*.exe`/`*.pdb`；已通知两票（输出一律进 `.scratch/tmp-NN/`，snippets 只放源码）。
- **ch07 单章测试瞬时 UTF-8 报错**：复跑 exit 0（bg_181）—— 判为批次中途读到半写文件/二进制混入的瞬时态，非内容缺陷；最终链以收尾态为准。
- **「UTF-8 报错」结案**：全仓库非 UTF-8 文件 = 0、所有 include 目标合法；两次报错均为写作者正在写文件时的瞬时读（Python 捕获复跑 ch02/ch07/ch09 全绿）—— 最终链在写作者空闲时跑。
- **隐藏行机制（实测 mdbook 0.5.4）**：`book.js` 给含 `.boring` 的块加 `hide-boring`（默认收起）+ 眼睛按钮展开；WRITING.md §1 已补说明。
- **linkcheck2 已启用**（book.toml，follow-web-links=false / require-md-extension=true / warning-policy=warn）：clean build exit 0；HTML 输出改为 `book/html/`；README 与 WRITING §3/§7 已同步。
- **跨章承诺**：前言承诺第 9、10 章「C++/Java/Python 对照」→ 已转达两章落实。

- **内嵌 SVG 可行性（08 票前置事实，已实测）**：mdbook 原样渲染 `<svg>`（`<main>` 内完整保留，`currentColor` 生效）；**但不能含空行**（CommonMark HTML 块遇空行截断 → `</svg>` 错位、图形碎裂）。08 决策时给用户：A 文本图 / B 内嵌 SVG（已验证）/ C mermaid 插件（需安装 + JS 资产）。

## 审校（reviewer 回报与分流）

- 票 11 第 2 章（RwCh02）：5 × minor、0 blocker/major；事实/规范/跨章三轴全部复跑通过。已转 Ch02Compiler 修（计数矛盾 / warning 的 help 断言过宽 / `= note:` 举例错位 / 术语首现位置 ×3 / 「原话」实为译文）。
- 票 15 第 6 章（RwCh06）：**1 × major**（C++ `substr` 断言错：实际 `pos > size()` 抛 `out_of_range`）+ 7 × minor（`vec!` 容量「等价」、`push` 摊还 O(1)、`&&str` 层级、术语首现、1.98 标注 ×2、第 10 章指针）。已转 Ch06Collections 修；WRITING §4 新增「跨语言断言必须核对一手文档」。
- 票 14 第 5 章（RwCh05）：**1 × blocker**（NOTE 称 edition 2021 下 `ref` 写法「只是警告」，实为「允许且无警告」；edition-guide 原文已核）+ 5 × minor（C# 无原生 DU；术语首现 ×4；「同一个 crate 内」误述；`HashMap` 键需 `Eq + Hash`）。已转 Ch05StructsEnums 修。
- 票 10 第 1 章（RwCh01）：9 × minor（引文实为转述；CRT 依赖导致「跨机可跑」不严谨；`:::` 归类；E0601 缺 1.98 标注；「编译不过」无可见标记；「借用」裸首现；路径分隔符；目录树缺 `.git/`；命令计数含糊）+ 与 ch02 的 `cargo check` 重复（**裁定：接受不改**）。已转 Ch01GettingStarted 修。
- 票 20 第 11 章（RwCh11）：6 × minor（错误行引用缺反引号 ×2；doctest 回指应为第 31 章；输出含批次临时路径；`--nocapture` 已弃用；失败起点不可直接复现）。已转 Ch11Testing 修。
- 票 17 第 8 章（RwCh08）：**1 × major（跨章承诺缺口）**：ch05 承诺「E0616 跨模块访问私有字段，第 8 章会碰到」，ch08 通篇没有 → 已要求补 E0616 最小复现 + 1.98 输出；另 6 × minor（跨 crate 优化归因、Cargo.lock 节选标注、features/workspace 体例、标题「私有模块」应为「私有项」、根模块措辞、Windows 执行写法）。已转 Ch08Modules 修。**另：ch08 的三段 include 待 build 后目视确认渲染**（评审侧无法跑 build）。
- 新规则：WRITING §5 增「引用官方英文原文必须逐字核对」（第 1/2/11 章同类问题）；§4 增「跨语言断言核对一手文档」（第 5/6 章同类问题）；§5 增「命令行与路径口径」（提示符 `$`；正文文件引用正斜杠；命令与粘贴输出按实况）。
- **修回进度**：ch02（5）✓ / ch06（7）✓ / ch01（9）✓ / ch05（8，含提示符与 `.\task_state.exe` 回正）✓ / ch08（8，含新补 E0616+E0451）✓ / ch03（5）✓ / ch11（6）✓ / ch04（5，E0505 归属修正 + 速查表补行）✓ / ch07（5，含 blocker：`unused_must_use` 是警告不是错误）✓ / ch09（14，含 blocker：Windows `-o` 命名坑）✓；在修：preface（11）、ch10（12）；RwProject1 审校在跑。
- **术语并表已完成**：`GLOSSARY.md` 19 → **99 条**（新增「类型与建模 / 所有权与借用 / 集合与字符串 / 错误处理 / 模块与工程 / 泛型与 trait / 迭代器与闭包 / 测试」八个分节；社区表已核对，分歧与 `_Avoid_` 均已记录）。
- 收尾待办（我）：路径/提示符终扫；全量链 + include 渲染目视确认；08 图示决策（已向用户发问）；地图/README/提交。

## 收尾状态（2026-10-18）

- **08 票 resolved**：图示体系 = **内嵌 SVG**（用户裁决）；样例图已落 `src/ch06-collections-strings.md` 的 UTF-8 一节，并验证：`aria-label` 匹配、元素齐全、build 无 WARN/ERROR、全站 0 处 `{{#` 残留。
- **新开票 22**（基础篇图示补全，需求 ~35 处，等用户验样例后排期）；地图（进度 / 图示 / 决策 / 迷雾）、README、`examples/README.md` 已同步。
- **术语并表完成**：`GLOSSARY.md` 19 → 99 条。
- **全量链已全绿**（2026-10-18）：`mdbook build` 0 WARN/ERROR（含 linkcheck2）｜`mdbook test` exit 0（13 章全测）｜`cargo test --manifest-path examples/Cargo.toml` 19 + 2 passed｜项目 1：`cargo build` 0 warning、`cargo fmt --check` 干净。收尾提交 `3ede8a1`，工作区干净。
- **收尾提交（票）**：`c868803` —— 13 张票 `## Answer` + `resolved`（用户验收通过）。

## 批次关闭（2026-10-18）

- **用户验收通过**（「效果非常好」）；13 张写作票（09–21）已写 `## Answer` + `Status: resolved`。
- 未结：**22 票**（基础篇图示补全 ~35 处，等排期）；进阶篇第一批写作票（第 12–18 章）待开。

## 新一轮：两线并行（2026-10-18 开工）

- **线 A — 进阶篇第一批写作**：票 `23`–`29`（第 12–18 章）已开（占位文件 + `SUMMARY.md` 接链完成）；7 个写作者在跑；流程同上一轮（批写 → 审校 → 修回 → 链）。
- **线 B — 基础篇补图（票 22）**：12 个补图代理在跑（ch01–ch11 + 项目 1；ch06 已有「你好」字节布局图，再补 2 张）；规范 = `WRITING.md` §9；样例 = ch06 那张。
- 收尾待办：图落地后跑 build + 逐章 HTML 校验（`aria-label` 计数）；然后 22 票写 Answer（用户抽查后）；进阶篇落地后按上轮模式派 7 份审校。
