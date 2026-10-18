# 09 前言：读者须知与使用约定

Type: task
Status: resolved
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
- 2026-10-18 按协调者裁定修订（票 09，Status 保持 claimed）：
  A. 消歧「已会 Rust」的跳读路径 → 现为「第 1、3 章可以快速翻过，第 2、4 章建议照读——诊断训练与所有权模型是本书主干；第 5–8 章按需跳读，第 9 章起正常往下」，不再与「按顺序读」冲突。
  B. 隐藏行说明按 mdbook 0.5.4 事实改写（协调者实测：`# ` 行被包进 `<span class="boring">`，渲染页面上看不到）→ 现为「有些片段带几行默认不显示的辅助行（源码里以 `# ` 开头），只为让片段保持完整、能通过编译检查；渲染页面上看不到它们，要完整文件就看仓库里该章的源文件或 `snippets/`」，不再说「你会看到」。
  另：为让「正文 900–1500 字」在**所有**计数口径下都成立，删了几处修辞性赘语（引言第二段末句、表格定位列的冗词等），未动任何需求点。
  自验（修订后）：`mdbook test -c preface.md` → `INFO Testing chapter '前言': "preface.md"`，**exit 0**；字数四口径：汉字 1202 / 汉字+拉丁词 1283 / 汉字+中文标点 1417 / 汉字+标点+拉丁词 **1498**（全部 ≤1500）；emoji 零命中；代码块标注不变（`console`/`rust`/`rust,compile_fail`/`text` 各 1）。
  裁定项 1–5 已确认无需改动（`unsafe` 首现不加括注、「包管理器」不算「包」首现、Reference 定位语保持、语言对照角保持可选、与 04 措辞不逐字对齐）。
- 2026-10-18 交叉审校修订（reviewer RwPreface：1 major + 10 minor，11 条全部处理；票 09 仍 claimed）：
  ① major `:35` 过度承诺 → 讲法③ 改为「**示例可验证**。绝大多数示例进自动检查：能跑的会真的被编译运行，故意报错的会被断言「必须编译失败」；少数标 `rust,ignore` 的片段（外部依赖、多文件）不编译」。前言不写具体条数，避免随章节变动过期（实测当前仓库共 26 处 `rust,ignore`：ch08×4、ch09×4、ch10×3、ch11×3、project1×12 —— 与 reviewer 报的 23 有出入，以其测量时点的仓库快照为准）。
  ② `:3` 起点线精度 →「不假设你会 Rust，也不假设你系统学过泛型（generics）、迭代器（iterator）或编译期概念——这些在书里现场建立」。
  ③ 补 04 落差 → 承诺① 加「（含底层模型）」；讲法② 加「性能与并发问题也按这条路径定位」。
  ④ `:40` 删「每章开头写明前置」（不在 §4 模板、未写章无从验证），保留「第六篇……可以跳读」。
  ⑤ `:46` edition 口径 →「语言 edition 为 **2024**（非编译器版本，1.85 起稳定）」。
  ⑥ `:48-51` console 块补 `$`（依据 WRITING.md §5 第 100 行的现行口径：提示符只用 `$`）。
  ⑦ `:53` →「只在 nightly 上可用的特性一律用警告框标出（仅 nightly）」（§2 的 `> [!WARNING]` 要求）。
  ⑧ 隐藏行 →「默认收起（源码里以 `# ` 开头，点代码块的眼睛按钮展开）」；`compile_fail` →「源码里标了 `compile_fail`，本来就编不过」（渲染页无可见标记，故点明是源码标注）。
  ⑨ 报错样例文件名 `moved.rs` → `main.rs`；重跑探针（`.scratch/tmp-09/main.rs`，cwd 在探针目录）复得 `error[E0382]` + `--> main.rs:4:16` 等关键行，粘的是重跑结果。
  ⑩ `:93` 补 `--std`、`--nomicon`、`--reference`、`--cargo`、`--edition-guide`、`--error-codes`（1.98 本机 `rustup doc --help` 逐个核到）。
  ⑪ `:99` →「未定义行为（UB, undefined behavior）」。
  另：为在新增内容下仍满足「正文 ≤1500 字」，再收紧若干冗词，并删掉收尾句「准备就绪后，翻到第 1 章。」（现在以资料表收尾）。探针目录 `.scratch/tmp-09/` 已再次整目录删除。
  自验（本轮修订后）：`mdbook test -c preface.md` → `INFO Testing chapter '前言': "preface.md"`，**exit 0**；字数四口径：汉字 1190 / 汉字+拉丁词 1272 / 汉字+中文标点 1415 / 汉字+标点+拉丁词 **1497**（全部 ≤1500）；emoji 零命中；代码块 4 个标注不变（`console`/`rust`/`rust,compile_fail`/`text`）；`src/preface.md` md5 = `1ab93d4fad510dac9445e8bc4cd5ecaf`。
  给协调者的提醒：正文里 `.\main` 是 Windows 命令写法（WRITING.md §5「Windows 实际输出里的反斜杠在首次出现处说明一次即可」），做「反斜杠 → 正斜杠」全局归一时请跳过这一处，否则会变成错误的 Windows 指路。
- 2026-10-18 收尾句回归（协调者请求）：加回「准备就绪后，翻到第 1 章。」，为腾字数从资料表「定位」列挤掉等量冗词——Book 行「从安装讲到」→「讲到」、Cargo 行「workspace 管理」→「workspace」、Edition Guide 行「之间改了什么」→「改了什么」、Error Codes 行「这类编号」→「编号」、std 行「查 API 第一站」→「查 API 首选」；Reference 行的「，供查证」保留（其余未动）。
  自验：`mdbook test -c preface.md` → `INFO Testing chapter '前言': "preface.md"`，**exit 0**；字数四口径：汉字 1189 / 汉字+拉丁词 1271 / 汉字+中文标点 1416 / 汉字+标点+拉丁词 **1498**（全部 ≤1500）；`tail -3` 确认末行为「准备就绪后，翻到第 1 章。」；`src/preface.md` md5 = `ad7f8b365103bfb29beba466082bd261`。

## Answer

2026-10-18 批次收尾（用户验收通过）。

- 交付：`src/preface.md`（1189 汉字，六小节）。
- 自验：`mdbook test -c preface.md` exit 0（两次）+ 负向对照（注入类型错误 → exit 101）。
- 交叉审校 11 条全修（1 major：讲法③「每段代码都进自动检查」系过度承诺）；裁定 5 条按稿（`unsafe` 不加括注 / 「包管理器」非术语首现 / Reference 定位语 / 对照角可选 / 措辞不与 04 逐字对齐）。全量链：`mdbook build` 0 WARN/ERROR（含 linkcheck2）｜`mdbook test` exit 0（13 章）｜`cargo test --manifest-path examples/Cargo.toml` 19 + 2 passed。
- 遗留不确定：无。明细见 `.scratch/rust-guide/batch-notes.md`。
