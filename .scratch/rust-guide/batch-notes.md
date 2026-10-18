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

- **线 A 进度**：**7 章全部落稿** —— ch12（4717 汉字）✓、ch13（5106）✓、ch14（4398）✓、ch15（4513）✓、ch16（6336，裁定不删减；术语「变型→型变」18 处已统一）✓、ch17（5994）✓、ch18（5486）✓；当前在收：7 份交叉审校 + ch18 的 linkcheck 警告修复。
  - 新术语与图示需求（收尾统一并表/补图）：ch12 —— 关联类型/扩展 trait/密封 trait/一致性/固有方法/基础类型构造器/dyn 兼容性/blanket impl；coherence 2×2、sealed 三层、关联类型 vs 泛型对照。ch13 —— trait 对象/向上转型/向下转型/vtable/auto trait/dyn；胖指针+vtable 布局（最高优先）、单态化 vs dyn 路径、向上转型。ch14 —— 隐藏类型/捕获/精确捕获/APIT/RPITIT/AFIT；APIT/RPIT 角色方向、捕获集合 2021 vs 2024、RPIT/Box<dyn>/enum 取舍。ch15 —— const 泛型/GAT/借用式迭代器/inline const；GAT 借用时间轴。ch16 —— 型变/**协变/逆变/不变**/HRTB/drop check/sound generic drop/drop scope/晚绑定生命周期；三种变型赋值方向、借用区间 vs 析构点、省略规则决策树。ch18 —— typestate/builder 模式/SemVer/破坏性变更/uninhabited；库↔应用责任分工、`#[non_exhaustive]` crate 内外语义差异。
  - **术语待裁**：coherence 的「覆盖（covered）/未覆盖」与 GLOSSARY「遮蔽」条的 `_Avoid_: 覆盖` 冲突 → 并表时在「遮蔽」下加区分注（coherence 语境的「覆盖」指本地类型构造器覆盖，不是遮蔽）。ch16 的「型变」裁定已执行（18 处统一，提交 `54137ae`）。
  - 章内遗留：ch13 —— vtable 头部字段顺序留 ch25；`async_fn_in_dyn_trait` 取自 research 未自行探针；`Pin<P>` 未实测。ch14 —— `cargo fix --edition` 插 `use<>` 依 Edition Guide 未本机跑；trait 内 `use<..>` 的 1.87 起点依 research 基线。ch16 —— `&mut` unsize 缩短仅证 1.98 可编译（≤1.97 未对照）；Vec 的 `#[may_dangle]` 依赖 Nomicon 结论未本机复现。ch18 —— Cargo semver「`as` 转换不允许」1.98 复现不出（正文回避）；Reference 的 `must_use` 位置列表未列方法但实测有效（按实测写）。
- **全量链（20 章）**：build exit 0（linkcheck2 9 条「Potential incomplete link」警告 → 已定位为方括号文本：ch13 `*[class.virtual]*`、ch18 引文 `[…]` ×5 与一处嵌套反引号断开的 span；已转作者修并写进 WRITING §7）；`mdbook test` exit 0（20 章）；examples 19+2 passed。
  - ch13 已修 ✓（`[class.virtual]` → 行内代码；全章方括号核查无其它隐患）；ch18 已修 ✓（`\[…\]`×4、C-GOOD-ERR 游离 `[`、两处双反引号 span；pulldown-cmark 渲染探针核对）→ **复跑 build：0 ERROR / 0 警告，test 20 章 exit 0**。
- **审校**：7 份进阶篇交叉审校已派（票 23–29，四轴，含对照角与版本事实核对）。
  - RvCh15 已回（**3 major** + 3 minor）：臆造 std 例（`core::fmt::Pointer` 无关联类型）、`adt_const_params`/`min_adt_const_params` 混淆、E0277/E0477 粘贴输出行号错一位 ×2；minor：E0554 引文漏反引号、`AsyncFnMut` 摘录漏 `where Args: Tuple,`。已转 Ch15GenericsLimits 修。
  - RvCh16 已回（**1 blocker** + 2 major + 5 minor）：练习 3 答案自相矛盾（`holder` 借 `bag` 自身字段 → 实测 E0597，与正文现场 1 相悖；块为 `ignore` 漏网）、术语「寿命」「容器」违规、「释放」误用于 drop、三层图方向、`unique.rs` 注释行号（37–38）、「drop glue」未解释、速查表 E0599 文案非真实。已转 Ch16Lifetimes 修（要求修后可编译则把块改 `rust`）。
  - RvCh12 已回（8 × minor，无 blocker/major）：三块 `rust,ignore` 实可编译应改 `rust`、密封 trait/一致性/孤儿规则术语统一、E0119 表述、跨 crate 帮助差异、`Counter` 出处、术语清单漏登、票内遗留与实际不符（裁定：更正票内遗留）。已转 Ch12TraitDesign 修。
  - RvCh18 已回（**1 blocker** + 2 major + 7 minor）：C-BUILDER「五条骨架」实为四条、PhantomData 章号错（15→16）、transcript 行号与 cat 源码矛盾；E0639 struct/variant 文案、should/must 口径、弯引号、条目名方向、丢值片段版本、缩进、新词首现。已转 Ch18ErrorApi 修。
  - RvCh14 已回（**2 blocker** + 4 major + 8 minor）：表里 `Box<dyn Trait>` 误写「透传 auto trait」（实测 `dyn` 不透传）；练习 3 修法二 `Box<dyn Future>` 无法 `.await`（须 `Pin<Box<…>>`，且两句自相矛盾）；`main::values` vs `src/main.rs::values`、§6 行/列坐标错、help 非逐字；**GLOSSARY 矛盾**：`不透明类型` 条目 `_Avoid_: 隐藏类型` 与本章核心术语冲突（裁定：新增 `隐藏类型（hidden type）` 条 + 删该 `_Avoid_`，正文不动）；E0309 漏、lint 下划线、const 参数、3 条报错、`{integer}`、摘录指针、执行器章号、新词首现。已转 Ch14ImplTrait 修。
  - RvCh13 已回（2 major + 2 minor）：JVMS §2.11.8 引文多出 "to invoke"、`supertrait` 未按第 12 章统一为「父 trait（supertrait）」；`Box<I>` 漏分配器参数、E0038 块缩进多 1 空格 + 省略未点明。已转 Ch13DynObjects 修。
  - RvCh17 已回（**1 blocker** + 4 major + 4 minor）：`Cell::update` 签名写错（`FnOnce(T) -> T` 非 `&mut`）、Send/Sync 表出处段错（自动 trait 段 vs Trait Implementations 段）、`&String` 标签误作 `&mut String`、`List` 布局漏 niche（无独立 tag）、「自动强转」与 GLOSSARY「解引用强转」冲突（裁定：统一「解引用强转」）；Deref「头号理由」、LazyLock 方法版本/接收者、`r.clone()` help 不可用。已转 Ch17SmartPointers 修。
  - **7/7 审校全收**（合计 5 blocker + 17 major + 37 minor）；7 位作者在修。修完即进：全量链（build+test+cargo test）→ 术语并表（含裁定：新增「隐藏类型」删 `_Avoid_`、「覆盖」区分注、「型变」「父子 trait」「解引用强转」等）→ 票 23–29 Answer（待用户验收）。
- **术语并表（GLOSSARY 侧完成）**：新增 **42 条** + 3 处裁定修正，现共 142 条，无重名、无连续空行。分布：语言与运行时 +5（引用计数/强弱引用/写时克隆/互斥锁·读写锁/锁中毒）、类型与建模 +2（无值类型/typestate）、所有权与借用 +4（析构检查/析构作用域/健全的泛型析构/自动解引用）、模块与工程 +3（SemVer/破坏性变更/builder）、泛型与 trait +28。跳过：关联常量（仅标题出现）、对象安全（并入「dyn 兼容性」条目）、防呆（票内已否决）。3 处裁定修正：遮蔽条 coherence 区分注、不透明类型/隐藏类型、向上转型。**待修稿全收后做正文侧复核**（密封 trait、一致性、父 trait、解引用强转、型变、隐藏类型、dyn 兼容性、向下转型、vtable、auto trait 等与表对齐）。
- **修稿进度**：✓ Ch12（9/9 + 自查反斜杠转义）、Ch13（4/4，finding 4 缩进经逐字节复核为**误报**，已回证据）、Ch15（6/6 + 自查追加 1 major）、Ch16（8/8）、Ch18（10/10，transcript 与真编文件脚本比对逐字一致；为保篇幅压缩措辞、未删事实）。在修：**Ch14、Ch17**。
- **2026-10-18 收尾（进阶篇批次）**：7 篇全部修回（5 blocker + 17 major + 37 minor 全处理）→ 术语并表落地（GLOSSARY 142 条；「遮蔽/覆盖」「不透明/隐藏类型」「向上转型」三裁定 + 42 条新增）→ 全书 `_Avoid_` 扫尾（转移→移动、销毁→析构、容器→集合、编号→错误码、守卫→匹配守卫、解引用强制转换→解引用强转、可变变量→可变绑定、属性→字段访问、释放（drop 义）→析构 等 ~50 处；含 ch01/04/05/06/08/09/10/preface 基础篇残留）→ **全量链**：build 0 ERROR/0 警告；test 20 章 exit 0；examples 19+2 passed。探针/临时目录已清（.scratch 只剩 rust-guide/）。**用户验收通过（2026-10-18）**；票 23–29 已写 `## Answer` + resolved，地图同步，批次关闭。
- **2026-10-19 第三批开工（双线并行）**：线 A = 第 19–22 章写作（票 30–33：并发基础 / 原子与内存序 / async 基础 / Pin 与自引用；脚手架 = 占位章文件 + SUMMARY 接链）；线 B = 进阶篇补图（票 34–40，ch12–18，清单在票内）。11 份子代理已派；协议 = WRITING §8（写作）/ §9（图示）；两线文件不重叠。
- **2026-10-19 线 B 完成（进阶篇补图，票 34–40）**：7 票全报 —— ch12 3 / ch13 3（vtable 布局，最高优先）/ ch14 2 / ch15 1 / ch16 3 / ch17 5 / ch18 2 = **19 张**（全局计数：ch12–18 = 3/3/2/1/3/5/2 ✓；基础篇 36 张未变）。每票结构自查全过 + 栅格化目视（ch13/ch17 用双主题实变量；ch16 另做 CJK 量宽）；ASCII 草图按「逐项覆盖」删 **5 处**（ch12 密封树、ch13 vtable 草图 + 多态路线草图、ch15 GAT `text` 块、ch17 Rc 计数块），保留的均逐条论证（ch16 三层框图等）。票面口径纠偏 1 处：ch12「允许/允许/E0117/E0210」按事实改为 3 允许 + 1 错误格（E0117/E0210 分列）。**待收尾 build 统一确认 HTML 渲染**。
- **2026-10-19 线 A 进度**：✓ Ch22（`src/ch22-pin-self-reference.md`，5247 汉字；15 个测试 = 11 真跑 + 3 `compile_fail` + 2 `ignore`，`mdbook test -c` exit 0；注入反证 exit 101 → 还原；snippets/ch22/selfref.rs；新术语：自引用 / 地址敏感 / pin 投影 / 结构化 pinning）。✓ Ch21（`src/ch21-async-basics.md`，4935 汉字；17 doctest = 15 通过 + 2 `ignore`；注入反证 101 → 还原；snippets/ch21/{executor.rs,cancel.rs}，std-only `block_on`；`join!` 1.98 仍 Experimental 已注明；async 闭包按 1.85 稳定改写并实测）。✓ Ch20（`src/ch20-atomics-memory-ordering.md`，5356 汉字；12 doctest = 7 跑 + 3 `compile_fail` + 1 `no_run` + 1 `ignore`；注入反证 101 → 还原；练习 1 缺 `compile_fail` 被门禁抓住已修；snippets/ch20/litmus.rs（SB litmus 200 万轮 × 三序）+ `--emit asm` 对照 + `-O` 实测；自画 1 张 happens-before 时间轴 SVG（§9 合规待收尾统一核）；新术语一大串待并表；顺手清掉自己误建的仓库根文件）。✓ Ch19（`src/ch19-concurrency-basics.md`，5453 汉字；注入反证 `21 passed; 1 failed; 1 ignored` → 还原 exit 0；snippets/ch19/wordcount.rs；覆盖 I9 行 + §5.2#14 WARNING + §4.2 scoped 1.63；探针抓出并修掉一处会卡死的 channel 写法；新术语：通道（channel）/ 交会（rendezvous）；未写语言对照角（可选，宁缺勿错））。**线 A 4/4 完成**；下一步：派 4 份交叉审校（票 30–33 各一份，四轴）。
- **2026-10-19 预检查**：`mdbook build` → exit 0、**0 ERROR / 0 警告**（含 19 张新 SVG + ch20 自画图 + 4 新章全部链接）；4 份审校已派（RvCh19–RvCh22，探针 `.scratch/tmp-r3N-chNN/`）。
- **2026-10-19 审校 4/4 全收（合计 2 blocker + 2 major + 30 minor，全部转原作者修）**：RvCh19 = 7 minor（`LocalKey` Send/Sync 表格错、引文计数/弯引号/截断、`join` 返回类型缺 `+ 'static`、四条排序说明、练习 2 输出缺代码）；RvCh20 = **2 blocker**（CAS 失败序「会 panic」实为**编译错误**（`invalid_atomic_ordering` lint，常量写法）+ 小结同错；建议补成报错现场第三例）+ 1 major（`type.AtomicUsize.html` 404）+ 6 minor；RvCh21 = 1 major（`block_on_all` 实为两次睡/醒，插桩已证）+ 4 minor（`Poll` 归属、`join!` 措辞、`ready` 标识符、报错块标注/caret 空格）；RvCh22 = 13 minor（栏位宽度/删行标记/术语首现格式/`容器`·`转移` 违规/22.6 标题/Drop guarantee 措辞/`Pin::new` 归类等）。
- **2026-10-19 第三批收尾（修回后全量链）**：4 篇修回全完（Ch19 7/7、Ch20 8/8——两个 blocker 按实测重写并新增「报错现场第三例」（`compile_fail`+`should_panic`，注入反证）、Ch21 5/5——major 睡/醒次数按插桩改写、Ch22 13/13）→ **全量链**：`mdbook build` 0 ERROR/0 警告；`mdbook test` **24 章 exit 0**（0 FAILED）；examples 19+2 passed。术语并表 **+25 条**（语言与运行时 +20：通道/交会/原子类型/RMW/CAS/先行发生/顺序一致性/伪失败/无锁/无等待/存储缓冲/自旋锁/ABA/完整屏障/执行器/任务/运行时/唤醒·Waker/状态机/取消；类型与建模 +5：自引用/地址敏感/pin/pin 投影/结构化 pinning）→ GLOSSARY 共 **167 条**。**待**：用户验收（第 19–22 章 + 进阶篇图）→ 票 30–40 写 `## Answer` + `resolved`。
- **2026-10-19 用户验收通过（第三批）**：票 30–40 已写 `## Answer` + `resolved`（共 11 张）；地图/README 同步（SUMMARY 已挂到第 22 章）；批次关闭。后续可选：项目 2/3、系统层（第 23 章起）、工程实践、补充章节。
- **2026-10-19 第四批开工（三线并行，票 41–49）**：线 A 系统层写作（41 ch23 unsafe/UB、42 ch24 provenance、43 ch25 布局/repr、44 ch26 FFI——素材按 research/02 I13–I16）；线 B 项目（45 `examples/expr-eval` 表达式求值器、46 `examples/task-queue` 迷你任务队列；workspace members 已加、stub 验过 `cargo check`）；线 C 第三批补图（47 ch19 三张、48 ch21 两张、49 ch22 两张；ch20 票面明示不再补）。9 份子代理已派；协议 §8/§9。
- **2026-10-19 线 C 完成（票 47–49）**：ch19 3 张（Send/Sync 路径、spawn vs scope、mpsc 拓扑）/ ch21 2 张（状态机展开、poll/waker 时序；两张分离并论证）/ ch22 2 张（Pin 三层+移动对照、Unpin 传播；带注入反证）= **7 张**；各票结构自查全过 + 栅格化目视（ch21 另有 `xml:space` 预证）；`text` 块审计：三章全是真实输出，**0 删**。待收尾 build 统一确认 HTML 渲染。
- **2026-10-19 线 A 进度（系统层）**：✓ Ch24（5113 汉字；12 passed/3 ignored；provenance API 逐条核到 1.84/1.89/1.91 稳定面 + nightly 表；5 个片段真跑）；✓ Ch25（5380 汉字；8 doctest；4 片段 + 2 自画图；**抓出跨章勘误**，见下）；✓ Ch26（4258 汉字；`13 passed/7 ignored`；**真 C 端到端跑通**（MinGW gcc → `c26.dll` → MSVC rustc 链接）；2024 两条 hard error（`extern` 必须 `unsafe`、`#[no_mangle]` 必须 `unsafe(...)`）实测；`unsafe_op_in_unsafe_fn` 在 2024 实测为 warn）。✓ Ch23（5592 汉字（同口径≈5320）；五能力（std/Book）与 Reference 八条差集；UB 演示 debug vs `-O` 差异 + 撒谎 `unsafe impl` → `0xC000001D`；注入反证 ×3；Miri 无 nightly 未跑——正文 WARNING + 命令给全，作者已把尝试安装的 nightly 卸载复原）。**线 A 4/4 完成**。术语裁定：GLOSSARY `unsafe` 条改「五类超能力」补「访问联合体字段」（std `keyword.unsafe.html` 原句逐字核过）。在写：仅线 B 两项目。
- **2026-10-19 线 B 进度**：✓ 项目 2（`examples/expr-eval/`，零依赖 + `#![forbid(unsafe_code)]`；**50 测试全绿** = 35 单测 + 9 e2e + 6 doc，clippy/fmt 干净；章 `src/project2-expr-eval.md` 5482 汉字、31 处 `{{#include}}`、1 张自画 SVG、3 报错现场；顺带实测 **edition 2024 下 `fn lex(&str) -> Vec<Token>` 不再报 E0106**（改 lint `mismatched_lifetime_syntaxes`），正文按实况改）。在写：仅项目 3。**收尾动作（协调者）**：`examples/README.md` 成员表补 `expr-eval`/`task-queue` 两行。
- **2026-10-19 线 B 2/2 完成**：✓ 项目 3（`examples/task-queue/`：`ThreadPool`/`Queue`/`TaskHandle`/`Timeout`，零依赖无 unsafe/async；**15 测试全绿** = 2 单测 + 10 集成 + 3 doc；**并发用例连跑 20 次 0 失败** + `--test-threads=1`/`--release` 复跑；章 `src/project3-task-queue.md` 5424 汉字、1 张自画 SVG、3 报错现场（E0373 / `Rc` 非 `Send` / `UnwindSafe`，rustc 实编取原文））。**第四批 9 项落地物全齐**（4 章 + 2 项目 + 7 图）。在跑：RvCh23–26 四份审校；RwProj2/3 两份项目复审已派。
- **2026-10-19 审校回执（6 份中的 2 份）**：✓ RvCh25 = **6 major + 6 minor**（`Option<Option<bool>>` 编码值 3 为**伪测**（实测 255）、`TwoU64` Option 解释写反且违练习三、ch17 回指过时（勘误后未同步）、「前三个都要求 `T: Sized`」对 `offset_of!` 不成立（容器可 `?Sized`）、`size_of_val`/`align_of_val` 只收引用（裸指针版 nightly）；minor：细指针/判别式括注/`repr` 限定/小结 7 条/SVG 字号 10×16/`repr(C)` 唯一性/「官方原话」标记）。✓ RvCh26 = 0 major + 5 minor（CStr 引文出处错标、`"system"` 变参例外遗漏、术语括注、变参声明块可去 `ignore`（评审复跑 exit 0）、`next_arg` 是 unsafe）。两篇已转原作者修。待：RvCh23/24、RwProj2/3。
- **2026-10-19 RvCh23 回执**：1 major + 13 minor（练习 2 答案「挨个报 `unnecessary unsafe block`」与实测相反且自相矛盾、且 2024 下该代码先因 `static_mut_refs` error；std 派生括号被改写；五/八差集漏「静态条含读 + unsafe external」；UB 检查口径（1.80 起有、受 debug-assertions 控制、尽力而为）；Cargo Book 引文位置「第一句」错；Miri 死锁/泄漏措辞 ×2；回指悬空/名不副实；「契约（contract）」括注；三处粘贴块内部省略未标注；练习 1 答案缺代码；**跨章：ch24:5/:19 与 ch22:91 的「四个超能力」→「五个」**）。分流：Ch23 修 #1–12+2 条（已转）；Ch26 附带 #14（已转）；**ch22:91 协调者已直改「五个」**；ch24 两处待 RvCh24 合并转。待：RvCh24、RwProj2/3。
- **2026-10-19 RvCh24 回执 + Ch26 修完**：RvCh24 = **4 major + 6 minor**（别名反例在 stable 编译不过（`invalid_reference_casting` 默认 deny）且「跑起来没事」需先 `allow`、`NonNull::with_exposed_provenance` 的 const 声明不成立（#154215）、「四个」→「五个」×2、删 `self.init = false;` 的反事实描述不准；minor：nightly 表缺 WARNING、术语首现次序、`:43` 无依据从句、`ptr::to_raw_parts` 不存在、`:193` 两类/三条例举、`:47` Miri 承诺落空）→ 已转 Ch24Provenance 修（含建议 (b) 改反例写法）。✓ **Ch26 修稿完成**：RvCh26 5 条 + 跨章 #15 全修（回指收窄为「23.6 第 2 条（悬垂读写）」）；门禁 exit 0、**14 passed / 6 ignored**（变参声明块去 `ignore` 后进门禁）+ 注入反证；26.8 主动对齐版**裁定保留**（「23.2 的结论」替代伪引文）；26.6→23.6#8 加回指**裁定不加**（两章措辞已一致，避免收尾期增改）。待：RwProj2/3；修稿中：Ch23/24/25。
- **2026-10-19 RwProj3 回执**：crate 边角全查（双重关闭/丢失唤醒/发送端计数/panic 隔离/中毒不可达均确认没问题）+ 1 major + 12 minor（把 `:275`「第 20 章锁中毒」→ 第 19 章、`:9` 公开类型 3 个、`:147/:206` `thread::Result` 补 `+ 'static`、`:416` 「help 两条」不成立、`:290` 概念落点与设计相反、`:3` worker 首现口径、`:84/:460` 结构差异（**裁定保留两节**，票里注明有意扩展）、SVG 宽 560/字号 11 修正、终端记录省略写全；crate：`Timeout.waited` 存请求值、`bounded(cap>0)` 零覆盖→搬探针用例、`pool.rs:152` expect 消息不可达；票块统计 5→6）→ 已转 Proj3Queue 修。待：RwProj2。
- **2026-10-19 Ch25 修稿完成**：12/12（伪测改 `0xff` + 「取值是实现细节别背」；`TwoU64`/`BigList` 解释与练习三统一；ch17 回指与勘误版一致；`offset_of!` 字段 Sized/容器 DST + 可跑例；`size_of_val` 仅引用 + `*_raw` nightly；瘦指针；discriminant 括注；niche 关停「只限带字段」；小结 6 条；SVG 16 处字号 10→12；`repr(C)` 唯一性；引文补原文并把三处「引号里放译文」改成间接叙述）。复跑 exit 0、doctest 8→**9**、注入反证 101→还原、篇幅 5624→**5472**。待：RwProj2；修稿中：Ch23/24、Proj3。
- **2026-10-19 RwProj2 回执（7/7 审校全收）**：**7 major + 11 minor**（「八个变体」实为 7、E0106 的「edition 2024 变化」站不住（lint 1.89 起 warn、四 edition 同输出）、现场一/二 transcript 行号各差 1、`--var x` 假终端记录、练习 2 的 `PREFIX_BP=3` 例子算错、多字节输入覆盖声明无测试且列宽 bug（全角按 1 列）、「闭集 vs 开集」误指第 12 章（应在 13）、「寿命」×4 等）→ 已转 Proj2Expr 修。**7/7 全收后修稿进度：Ch25 ✓、Ch26 ✓；在修：Ch23、Ch24、Proj2、Proj3。**
- **2026-10-19 Ch23 修稿完成**：14/14（练习 2 答案重写 + 2021/2024 实测对照；派生括号还原；差集补静态条；UB 检查改引 1.78/1.80 原文 + **修正协调者转述矩阵一行**（`debug-assertions=off` 实测不触发）；Cargo Book 措辞；死锁/泄漏以 UB 为锚；「契约（contract）」；三处省略标注；练习 1 补代码（`rust,no_run`）；23.6#7 补平台限定）。复跑 exit 0、块数 10→**11**、注入反证绿、篇幅同口径 ≈**5465**。**GLOSSARY「五类」已由协调者早前裁定并更新**（ch23/ch22 均已对齐）。修稿进度：Ch23 ✓/Ch25 ✓/Ch26 ✓；在修：Ch24、Proj2、Proj3。
- **2026-10-19 Ch24 修稿完成**：11/11（别名反例改 (b)：`&raw mut` + 共享借用，`rust,ignore`→`rust` 真进门禁、零警告跑通；const 口径三处改正（`NonNull::with_exposed_provenance` 非 const、入 nightly 表）；「四项→五项」；`checked_uninit` 反事实按实测重写（「要害是 `drop Noisy(first)` 两次」）；nightly 表加 WARNING；「严格来历/别名模型」首现；`:43` 换文档出处 + 逐字原文；`<ptr>::to_raw_parts`；「三类场合」+ 第二例对齐 Reference；NOTE 改「本机只装 stable，本章不含 Miri 实测输出」；`:334` 与表格统一）。复跑 exit 0、`13 passed; 2 ignored`、注入反证 101→绿、5434 汉字。修稿进度：Ch23 ✓/Ch24 ✓/Ch25 ✓/Ch26 ✓；在修：Proj2、Proj3。
- **2026-10-19 术语并表（第四批，+25 → 192 条）**：语言与运行时 +18（线程池 / 工作线程(worker) / 任务队列 / 句柄 / 背压 / 队头阻塞 / FFI / ABI / 调用约定 / 符号修饰 / 空指针优化 / 不透明句柄 / 契约 / 健全的·不健全的 / 裸指针 / 可变静态 / 未初始化 / 裸借用运算符）；类型与建模 +4（对齐 / 填充 / 判别式 / 空位(niche)）；所有权与借用 +3（来历 / 严格来历·暴露来历 / 别名模型）。跳过：可选 UB 检查（章内一次性概念）、声明·兑现（并入 `unsafe` 与「契约」条）。
- **2026-10-19 Proj3 修稿完成**：13/13（锁中毒回指 19、公开三类型、`thread::Result` 补 `'static`、报错三按真实 help、概念表 worker 表述、首现「工作线程（worker）」、结构裁定保留并注明、SVG 600/字号≥12、省略说明写全；crate：`Timeout.waited` 口径、**新增 3 个 `bounded(cap>0)` 用例**（集成 10→13）、`enqueue` expect 消息；票块统计 6）。顺带修掉 include 行段漂移（148:172 / 175:197）。自验：**18 passed**（2+13+3）、20× 连跑 0 失败、`--test-threads=1`/`--release` 复跑、5409 汉字、门禁 exit 0。**修稿进度：Ch23 ✓/Ch24 ✓/Ch25 ✓/Ch26 ✓/Proj3 ✓；仅剩 Proj2。**
- **2026-10-19 Proj2 修稿完成（第六份）**：18/18（变体数改 8：新增 `TooDeep` + 分组句重排；E0106 框架删除（四 edition 同输出、lint 1.89 起）；现场一/二行号与 gutter 按实跑重贴；裸 `--var` 演示；练习 2 按 `PREFIX_BP`=5/3/1/0 实跑重写；练习 1 删「数字也复制」；**真 bug 修复**：parser 加 `MAX_DEPTH=128`（400 层括号从 exit 253 栈溢出变干净报错 exit 2）、`columns_of` 宽字符/制表符列宽修正；「绝不 panic」验收标准落实；`call_builtin` 补公开面；「闭集 vs 开集」改指 ch13；「寿命」清零）。自验：**59 测试全绿**（39+3+11+6）、clippy/fmt 干净、5498 汉字、门禁 exit 0。**全部 6 份修稿完成。**
- **2026-10-19 第四批收尾（全量链）**：**build 0 ERROR / 0 警告**；`mdbook test` **30 章 exit 0**（0 FAILED）；examples **全绿**（todo 19+2、expr-eval 59、task-queue 18）。图：新 12 张 HTML 计数全对 + 浏览器实机抽查 ch25 repr 布局图与 proj3 流水线图正常。**审校总量（7 份）= 73 条全修**（ch23 14 / ch24 10 / ch25 12 / ch26 6 / proj2 18 / proj3 13）。术语 +25 → GLOSSARY **192 条**。**待**：commit + 用户验收（票 41–49）。
- **2026-10-19 用户验收通过（第四批）**：票 41–49 已写 `## Answer` + `resolved`（共 9 张）；地图/README 同步（SUMMARY 已挂到第 26 章 + 项目 3）；批次关闭。后续：项目 4 + 工程实践（第 27–33 章）与补充章节（34–41）。
- **【协调者勘误 · ch17:142】**：Ch25 实测 `List`（i32 载荷）**有独立判别字段**（`Cons` 首 4 字节为判别字段、`i32`@4、`Box`@8；`Nil` 字节 `01 00 00 00` 打头），niche 折叠只出现在 u64 载荷形状（`enum {T(u64, Box<u64>), N}`：16 字节无判别字段、`Option` 24）。协调者用字节转储探针复核定案，已按实测重写 ch17:142 并加「细节第 25 章」回指；当年 RvCh17 该条的依据（u64 形状的探针）套错了形状。
- **2026-10-19 第五批开工（三线并行，票 50–60）**：线 A 工程实践写作（50 ch27 宏 macro_rules!、51 ch28 过程宏、52 ch29 性能与剖析、53 ch30 Cargo/workspace、54 ch31 测试进阶、55 ch32 文档与 lint、56 ch33 Edition；素材 research/02 I17–I23）；线 B 项目 4（57 `examples/sys-probe` 封装 Windows 系统 API，带非 Windows 降级保 CI；ch28 另配 `examples/macro-demo`（proc-macro）+ `macro-demo-use` 两 stub）；线 C 第四批补图（58 ch23 两种用法、59 ch24 四形态×地址/来历、60 ch26 FFI panic 三防线）。11 份子代理已派；脚手架已 `SUMMARY` 接链（至 33 章 + 项目 4）。
- **2026-10-19 第五批落稿与审校（三线全落）**：线 A 7 章全落稿（汉字 5450–6599，口径见各票；均过 `mdbook test -c` + 注入反证）；线 B 项目 4（`sys-probe`：12 单测 + 6 集成 + 9 文档全绿；真信号控制台注销对照，`x86_64-linux-android --all-targets` 编译过）；线 C 补图 4 张（ch23×2/ch24/ch26，结构自查 + 栅格化目视）。**8 份审校共 63 条 findings**（RvCh27 12 / RvCh28 6 / RvCh29 9 / RvCh30 8 / RvCh31 5 / RvCh32 6 / RwProj4 9 / RvCh33 8 = 3 blocker + 13 major + 47 minor + 2 存疑）**全部修回**（无 1 条被驳回；审校另有 15+ 条「原怀疑不成立」撤回）。关键 blocker：RwProj4 抓出 `CtrlGuard::drop` 的 NULL/FALSE **并未真注销**（真信号类比实测：一次事件闭包被调两次）→ 改为传同一函数指针；RvCh29 抓出 `lto = "yes"` 其实**不报错**（原样透传）→ 改写为事实版；RvCh31 抓出 `should_panic` 口径与 ch11 矛盾。**篇幅裁定**：ch29 5882（+7%，先例 ch19=6534）、ch33 8476→**6599**（压缩至 ≤6600 达标），均接受。
- **2026-10-19 整合期协调者发现（批次级）**：`mdbook build` 扫出 ch30 两处 linkcheck2「Potential incomplete link」——根因 = 行内代码里用 `\``（反斜杠+反引号）「转义」反引号，**本机 pulldown-cmark 不认**（反斜杠原样显示 + 代码段配对错位）。全仓扫描 6 处：ch30×3、ch32×1 已坏；ch13:122 旧批同类错位（2-run 与 3-run 相邻）一并修；ch05:562、project1:611 经渲染核对良性（span 内字面反斜杠）。修法 = 双反引号包裹（内容以反引号结尾时首尾各留半角空格成对剥除）；**`WRITING.md` §7 已增补该坑**。**Windows 10 → Windows 11 口径统一**（ch28/ch31；10.0.26200 是 Win11 构建号）。修后 build **0 ERROR / 0 警告**。
- **2026-10-19 第五批全量链**：`mdbook build` **0 ERROR/0 警告**；`mdbook test` **33 章 exit 0**；examples 全 crate 绿（todo-cli / expr-eval / task-queue / sys-probe / macro-demo-use）。插图 **+12 → 79 张**：HTML 计数全对 + 浏览器逐张目视（12/12 通过）。**术语并表 +28 → GLOSSARY 220 条**（宏系统 11 / 模块与工程 12 / 测试 5）：过程宏统一 procedural macro、build script 保留英文、新增「语言方言（edition）」「假绿」「插桩」「MSRV」等；README/地图同步。**待**：commit + 用户验收（票 50–60）。
- **2026-10-19 用户验收通过（第五批）**：票 50–60 已写 `## Answer` + `resolved`（共 11 张，含 3 张补图票）；提交 `8b4d224`（内容）+ 收票提交；地图/README 同步（已挂到第 33 章 + 项目 4）；批次关闭。后续：项目 5（工程化改造，SUMMARY 草稿条目）与补充章节（第 34–41 章）。
- **2026-10-19 第六批开工（9 票并行，票 61–69）**：项目 5（61：`examples/release-drill` 工程化到可发布 + 以本仓 `.github/workflows/book.yml` 为样板并真实加固 + `cargo package` 验证）；补充章节（62 ch34 手写集合 D1 / 63 ch35 别名模型 D2 / 64 ch36 高级类型系统 D3 / 65 ch37 编译器与代码生成 D4 / 66 ch38 极简 async 运行时 D5 / 67 ch39 宏系统极限 D6 / 68 ch40 生产工程 D7 / 69 ch41 领域地图）。脚手架 = 9 占位章 + SUMMARY 接链（33 章 + 5 项目全挂）+ `release-drill` stub（工作区验过）。工具链：thumbv7m/thumbv7em/android 系 target 已装（真交叉编译可用）；wasm32 未装（文档化 + 标注）。nightly 侧一律 WARNING 标注、不伪造输出。
- **2026-10-19 第六批落稿与审校（9 票全落）**：项目 5（release-drill 可发布态：11+3 测试、`cargo package` 7 文件、book.yml 加 fmt/clippy/doc/package 4 作业）；ch34–41 八章全落稿（汉字 4451–5497；均过 `mdbook test -c` + 注入反证；ch40/41 真交叉编译；ch41 55 条外链全验证）。**9 份审校共 88 条 findings**（7 blocker + 15 major + 66 minor）**全部修回**。关键 blocker：ch34 `MutexGuard` 缺显式 `Sync` → 数据竞争（审校双线程实测丢 175 万次）→ 手工 `unsafe impl<T: Sync> Sync`；ch35 TB `foreign_read` 对 `ReservedFrz` 规律写错（11 处联动修）；ch36 E0117/E0210 分工判据写反（未覆盖参数判据）；ch37 两处（`u64::MAX-1` 不溢出 + 练习 asm 用 SysV `rdi`）；ch39 `count`/`index` 方向（1.76 已统一）+ `length`→`len`；ch41 第 40 章介绍/Arrow/AoS 等 3 处不实。
- **2026-10-19 第六批整合（协调者）**：术语并表 **+20 → GLOSSARY 240 条**（重借/借用栈/许可树/保护符/子对象来历/retag/FCP/守卫/重叠/负实现/特化/元变量表达式/调用点/中间表示/基本块/终结指令/代码生成单元/值槽/交叉编译/目标三元组；fundamental 补 `Pin` 与「不能覆盖内部参数」）；跨章微修：project1 的 `FromStr/Error` 章指修正、ch34 `niche` 统一「空位（niche）」；snippets 卫生（ch34/38/39/40/41 的 lock/target 清理）。
- **2026-10-19 第六批全量链**：`mdbook build` **0 ERROR/0 警告**；`mdbook test` 全书 exit 0；examples 7 crate 全绿（含 release-drill 11+3）；新 4 个 CI 作业命令本地逐条验证（`fmt --check` / `clippy -D warnings` / `doc -D warnings` / `package` 全 0）；插图 **+14 → 93 张**，浏览器逐张目视 14/14 通过。**待**：commit + 用户验收（票 61–69）。
- **协作事故（已处置，2026-10-18）**：共享探针目录 `.scratch/tmp-f22/` 被多个补图代理共用 → 互相覆盖（FigCh03 的图被别人的覆盖、FigCh07 撞到、目录后被清掉）。已广播：改私有 `.scratch/tmp-f22-<章号>/`，并要求用共享目录目视过的代理重跑。**正文未受影响**（一章一代理）；收尾校验将含「结构合规脚本 + 抽样栅格化目视 + build/HTML 计数」。
- 补图落地进度：**12/12 章全部落图**（共 35 张 + ch06 样张 1 张 = 36）；已回报 11 份（ch04 的图已在文件里、报告未到）。项目 1 的 4 张提示「与第 4 章 §4.5 时间轴词形对齐」——收尾目视时一并核对。
- **ASCII 示意图裁定（协调者，跨章）**：只删「被 SVG 完整取代的手绘示意图」；目录树 / 真实输出 / 一行式映射 / 未被 SVG 覆盖的 ASCII 管线保留。扫描定位候选 3 处（ch04 `:403`、`:587`；ch07 `:343`）→ 已转 FigCh04 / FigCh07 核对执行；ch01/ch08/ch11 的块均为目录树或一行式映射，保留。
- **线 B 完成（票 22 resolved）**：36 张图；结构合规 36/36、build 0 WARN/ERROR、HTML 计数 12/12 章一致、浏览器实机抽查 2 张复杂图渲染正确；ASCII 草图删 3 处（ch04 ×2、ch07 ×1）；`WRITING` §9 定稿。
- **票 22 Comments 并发追加风险（已对账）**：本票只收到 3 章小节（FigCh09/FigCh07/FigCh04），其余 9 章记录以各代理回报与本报台账为准；协调者汇总节已补。
- FigCh07 的裁定执行 ✓：`:343` 的 3 行 ASCII 错误链草图确认被 SVG 取代 → 已整块删除（引出句合并无断句；删前删后各复跑 exit 0；栅栏 56 偶数）。
