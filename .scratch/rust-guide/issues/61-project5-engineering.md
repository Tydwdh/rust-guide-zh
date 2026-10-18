# 61 项目 5：工程化改造

Type: task
Status: claimed

## Question

写第五篇收尾项目：**工程化改造** —— 把 `examples/release-drill`（协调者已建 stub）工程化到「可以发布」，并以**本仓库的真实工程**为样板。

- **目标**：
  1. `release-drill` crate：实现一个零依赖小库（建议「字节数 → 人类可读字符串」一类，接口自定），带单元测试与文档示例；补齐发布件：`Cargo.toml` 元数据（description / license / readme / keywords / categories / rust-version）、`README.md`、`CHANGELOG.md`（Keep-a-Changelog 模板 + 起始条目）、docs.rs 元数据（`[package.metadata.docs.rs]`）。
  2. 「发布前打包检查」：`cargo package --list` 与 `cargo package`（构建验证）真跑真粘；`cargo publish` 明确不在本书执行（无凭据），给一份「发布清单」。
  3. CI：以本仓库真实工作流 `.github/workflows/book.yml` 逐段讲，并**真实加固**它（二选一：A. 同 workflow 增加 `fmt`/`clippy`/`doc -D warnings` 作业；B. 新增独立 workflow 文件）。新命令必须在本机逐条真跑并写进票；不破坏既有三个步骤；说明「真实运行要等仓库公开到 GitHub」。
- **素材**：`.github/workflows/book.yml`（现成）、`WRITING.md` §3、ch30/32 已定稿（`[lints]`/clippy/文档门禁——回指不重讲）、Cargo Book `publishing.html` 与 manifest 页（快照）、docs.rs 文档。
- **边界**：不实际 `cargo publish`；不给 `release-drill` 加依赖；工具细节回指 ch29/30/32。
- **篇幅**：4000–5500 汉字（全文 CJK 计数）；章结构照项目 1 成稿（要做的东西 / 验收标准 / 设计取舍 / 逐块实现 / 报错现场 / 测试 / 小结 / 练习 2–3 题带答案 / 延伸阅读）。
- **DoD**：§8 —— 独占 `src/project5-engineering.md` + `examples/release-drill/**` + `.github/workflows/**`（按你的方案）+ 本票；禁 `git`；自验：`CARGO_TARGET_DIR=.scratch/tmp-61/target cargo test --manifest-path examples/Cargo.toml -p release-drill` 全绿 + `cargo package`（私有 target）通过 + `mdbook test -c project5-engineering.md` exit 0；探针 `.scratch/tmp-61/`；收尾 `Status: claimed` + `## Comments`。

## Comments

（写作者记录，2026-10-19，rustc/cargo 1.98.0 stable）

**交付物**

- `src/project5-engineering.md`：成稿，**5480 汉字**（CJK 计数，票要求 4000–5500）。结构照项目 1 成稿：这一章解决什么问题（要做的东西 / 验收标准）→ 设计取舍（6 条）→ 逐块实现（1 crate 本体 / 2 README+CHANGELOG / 3 清单元数据 / 4 打包检查 / 5 CI 加固）→ 报错现场（3 条）→ 测试 → 发布清单 → 小结 → 练习 3 题带答案 → 延伸阅读。内嵌 2 张手写 SVG（发布路径、`.crate` 构成）。
- `examples/release-drill/`（4 个文件）：`src/lib.rs`（286 行；`format` / `parse` / `ParseError`，11 个单元测试 + 3 个文档测试，公开项全带文档与示例）、`Cargo.toml`（description / license / readme / keywords / categories / `rust-version = "1.85"` + `[lints]` + `[package.metadata.docs.rs] targets = []`）、`README.md`、`CHANGELOG.md`（Keep a Changelog，含 0.1.0 起始条目）。零依赖，未动 `examples/Cargo.toml`。
- `.github/workflows/book.yml`：既有 `check` 作业（含原 4 个步骤）一字未动，新增 `fmt` / `clippy` / `doc` / `package` 四个作业（票内方案 A）。PyYAML 解析通过，作业与步骤清单已核对。

**自验（全部真跑，输出已粘进正文）**

1. `CARGO_TARGET_DIR=.scratch/tmp-61/target cargo test --manifest-path examples/Cargo.toml -p release-drill` → `11 passed` + Doc-tests `3 passed`，exit 0。
2. `CARGO_TARGET_DIR=.scratch/tmp-61/target cargo package --manifest-path examples/Cargo.toml -p release-drill --allow-dirty` → `Packaged 7 files, 14.4KiB (5.9KiB compressed)` + `Verifying` + `Compiling`，exit 0。
3. `mdbook test -c project5-engineering.md` → exit 0（3 passed / 5 ignored；正文说明了哪些块标 `ignore` 及原因）。**另做反证**：临时追加一块坏代码后变 FAILED（`3 passed; 1 failed`），确认 include 进来的块真在被编译，不是假绿。
4. 新增门禁逐条本机真跑：`cargo fmt --manifest-path examples/Cargo.toml -p release-drill --check` exit 0；`cargo clippy ... -p release-drill --all-targets -- -D warnings` exit 0；`RUSTDOCFLAGS="-D warnings" cargo doc ... -p release-drill --no-deps` exit 0；`cargo package ... -p release-drill --allow-dirty` exit 0。
5. `cargo package --list --allow-dirty` 粘了完整 7 行清单（`.cargo_vcs_info.json` / `CHANGELOG.md` / `Cargo.lock` / `Cargo.toml` / `Cargo.toml.orig` / `README.md` / `src/lib.rs`），并解包核对了归一化 `Cargo.toml` 与 `.cargo_vcs_info.json` 内容。`cargo publish --dry-run --allow-dirty`（**未执行真实发布**）也真跑过，停在 `warning: aborting upload due to dry run`。
6. 报错现场三条全是真输出：intra-doc link 歧义（`format` 既是函数又是宏）来自本 crate 的第一版文档、被 `[lints.rustdoc] broken_intra_doc_links = "deny"` 拦下；`missing_docs` deny 与 `unresolved link` 用探针 `docsgate/` 复现；脏工作树拒绝来自本仓真实第一次运行。

**遗留 / 建议协调者收尾时看**

- `mdbook build`（含 linkcheck2）按协议没跑。本章的方括号文本全部落在行内代码或围栏内（与 ch30/ch32 同款），已按 WRITING §7 自查；仍建议收尾 build 时确认 0 ERROR / 0 warning。
- 2 张 SVG 已用 `read :img` 逐张目视核过几何、配色变量与断行；换主题后的 HTML 观感请收尾 build 时抽查。
- `[package.metadata.docs.rs]` 只写 `targets = []`（crate 无 feature、无平台分支）；docs.rs 上的真实构建本地无法验证，边界在第 32 章 32.5。
- `rust-version = "1.85"` 是声明（edition 2024 下界）：本机只有 1.98，「1.85 能构建」没有本机证据，正文与发布清单都写明要靠 CI 装那条工具链；本批**没加 MSRV 作业**（新命令无法本地真跑，按协议不写）。
- `repository` / `homepage` **故意空着**（本仓无公开远端）→ `cargo package` 会有一条 `warning: manifest has no documentation, homepage or repository`，正文（设计取舍 4 / 报错三 / 发布清单第 3 步）都交代了；仓库公开后补上即可消警。
- 术语：用到 `doctest`（文档测试）、`intra-doc link`（文档内链接）、`MSRV`，GLOSSARY 均已有条目，**未新增术语**。
- 衔接：正文回指第 4 / 6 / 7 / 9 / 10 / 11 / 29 / 30（30.1、30.3、30.4、30.6、30.7）/ 31 / 32（32.1、32.5、32.6）章，章节号逐条核过（审校前那两处「第 3 章」已按 F1 撤掉）。
- 探针 `.scratch/tmp-61/`（`target/`、`crate/`、`docsgate/`、`nometa/`、`svg/`、备份）已按协议删除。

**审校回执处理（F1–F10 + 存疑 1，2026-10-19）**

- **F1（major，回指错）已改**：`mdbook build` 扫日志那条的括注改「（本仓实测）」（核过 ch03 里确实没有这个坑）；「假绿」那条改指第 31 章（该章 31 节有「假绿：`mdbook test` 绿不等于测试跑过」专节）。
- **F2 已改**：`KiB`/`kB` 差 2.4%，并补一句「到 `MiB` 一档就是 4.8%」。
- **F3 已改**：练习 1 答案的换算例对齐真实值 —— 1024 字节在 SI 规则下是 `1.0 kB` → 解析回 1000；文档断言改写成 `1.0 KiB` → `1.0 kB`、`1.5 KiB` → `1.5 kB`。
- **F4 已改**：必填口径改挂 manifest 页，并引两条 note 原文（「crates.io requires the description to be set.」/「crates.io requires either license or license-file to be set.」，均从 1.98 快照逐字核对）；发布页只作为「平铺清单 + keywords/categories 建议」的来源。
- **F5 已改**：报错一成因写明「这条不需要 `RUSTDOCFLAGS`，清单里的 `[lints.rustdoc]` 就把它拦下了」（该次输出确实是普通 `cargo doc` 跑出来的）。
- **F6 已改**：发布清单第 3 步改为「正式发布时仓库里要有对应全文，本项目样板没附」。
- **F7 已改**：include 范围随重排后的行号改为 `1:39`（头注释 + `check` 作业）与 `40:77`（3 行注释引子 + 4 个新作业）。
- **F8 已改**：`.parse::<f64>()` 指第 7 章（ch07 大量使用 `.parse()`）；汇总行改为「第 7 章的 `.parse()` 与自定义错误（`impl std::error::Error`、`?`）、第 9 章的 `Display` 实现、第 10 章的闭包与迭代器」。
- **F9 已改**：正文所有命令去掉 `CARGO_TARGET_DIR=.scratch/tmp-61/target` 前缀，「版本口径」段改为「命令都可原样复制；产物落在 `examples/target/`，`.gitignore` 已忽略」；随后**按正文命令原样重跑**了全部被引用的命令（见下）。
- **F10 已改**：四步编号前补「（`check` 作业里 `actions/checkout` 之外的四步）」。
- **存疑 1 已核实，并顺手把 diff 收窄到最小**：初版我给 workflow 加过 workflow 级 `env: CARGO_TERM_COLOR`、`check:` 上方一条注释、并改过文件头三行 —— 现已全部撤回。最终 `.github/workflows/book.yml` 的改动 = 文件头与 `check` 作业**逐字不动** + 末尾新增 3 行注释 + 4 个作业（`fmt`/`clippy`/`doc`/`package`）。用脚本逐字比对过 check 作业原文与现文相同，且解析后顶层无 `env`、作业顺序为 check → fmt → clippy → doc → package。

**审校后重跑（命令即正文里的原样，均 exit 0）**

- `cargo test --manifest-path examples/Cargo.toml -p release-drill` → `11 passed` + Doc-tests `3 passed`。
- `cargo package --manifest-path examples/Cargo.toml -p release-drill --allow-dirty` → `Packaged 7 files, 14.4KiB (5.9KiB compressed)` + `Verifying` + `Finished`。
- `cargo publish --manifest-path examples/Cargo.toml -p release-drill --dry-run --allow-dirty` → 停在 `warning: aborting upload due to dry run`（未真实发布）。
- `cargo fmt … -p release-drill --check`、`cargo clippy … -p release-drill --all-targets -- -D warnings`、`RUSTDOCFLAGS="-D warnings" cargo doc … -p release-drill --no-deps` → 三条均无输出/正常结束、exit 0。
- `mdbook test -c project5-engineering.md` → exit 0（3 passed / 5 ignored）。
- 复核：YAML 解析通过（5 作业、check 步骤与原文一致、无顶层 `env`）；本章 include 范围 `1:39`/`40:77` 抽出来核对过首尾行；篇幅 5480 汉字（CJK，票要求 4000–5500）。
