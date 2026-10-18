# 17 第 8 章 模块、包与依赖

Type: task
Status: resolved
Blocked by: 06

## Question

写基础篇第 8 章：模块系统（mod/use/pub 与可见性）、crate 与包、Cargo.toml 基本项、引入外部依赖与版本约定。

- 心智模型：crate 边界与 API 可见性 —— 第二篇的 trait 设计会重用这套语言。
- 章末 2–3 道理解题（带答案）。DoD 见地图 Notes + 06。

## Comments

- 2026-10-18 初稿：文件 `src/ch08-modules-packages.md`（H1 `# 第 8 章 模块、包与依赖`；八节模板 + 可选「语言对照角」；正文中文 3541 字，不含代码块）+ `snippets/ch08/shop/{main.rs, catalog.rs, catalog/items.rs}`（纯源码，无 Cargo 工程/无编译产物；`main.rs` 是 crate 根，`catalog/items.rs` 演示 `foo.rs + foo/bar.rs` 布局）。
  自验：
  1. `mdbook test -c ch08-modules-packages.md` → exit 0。门禁有效性对照：临时追加一个必错块后 `test result: FAILED. 13 passed; 1 failed; 4 ignored`（**本章 13 个 doctest 全过、4 个 ignore**），删除后与备份 `diff` 为空、复跑 exit 0。
  2. `rustc --edition 2024 main.rs -o ../../../.scratch/tmp-17/shop-verify.exe`（在 `snippets/ch08/shop/` 内，产物落 scratch）→ 无警告；运行 → `2 件商品，合计 288 元` / `- 键盘：199 元` / `- 鼠标：89 元`。
  3. 正文两处 `$ cargo run` 输出块，用**逐字从章文件抽取代码**建的临时 cargo 工程真跑 → `|p| = 5`、`key = rust`。
  实测输出清单（rustc/cargo 1.98.0，Windows）：
  1. E0603 `module \`inner\` is private`（含 `function \`greet\` is not publicly re-exported` 与 note 行）
  2. E0603 `function \`secret\` is private`（`pub(in crate::a)` 变体）
  3. E0432 `unresolved import \`net\``（2018 统一路径；含 edition-guide note 与 `use crate::net::http;` 建议）—— 放在「路径与 use」的 WARNING 盒里，不进报错现场
  4. E0433 `cannot find module or crate \`helpers\` in this scope`
  5. E0583 `file not found for module \`stats\``；嵌套版 `config`（help 给出 `util\config.rs` / `util\config\mod.rs`）
  6. `cargo add itoa --offline` → `Adding itoa v1.0.18 to dependencies` / `Features:` / `- no-panic` / `Locking 1 package to latest Rust 1.98.0 compatible version`
  7. `cargo build --offline` → `Compiling itoa v1.0.18` / `Compiling dep-demo v0.1.0` / `Finished \`dev\` profile … in 1.25s`
  8. `cargo run --offline --quiet` → `2026`（依赖用法示例，`rust,ignore` 块）
  9. `Cargo.lock` 真身（`version = 4`、itoa 1.0.18 + checksum）摘进正文 `toml` 块
  10. `cargo new dep-demo --vcs none` → `Creating binary (application) \`dep-demo\` package`
  遗留：
  - 本机无外网：依赖实操用 `--offline`（crate 来自本机 registry 缓存），正文已注明「联网时去掉该参数即可」。联网时 `cargo add` 是否额外打印索引更新行**未验证**，正文未声称。
  - `itoa 1.0.18` 是缓存中可用的版本；联网读者可能装到更新的 1.0.x。正文版本号是实测原样，semver 解释不受影响。
  - 未跑 `mdbook build`（批次协议禁止）：`{{#rustdoc_include}}` 放在 `rust,ignore` 栅栏内是 mdbook 官方 testsuite 同形用法（`mdbook-0.5.4/tests/testsuite/includes/all_includes/src/rustdoc.md`），但**渲染展开未目视确认**，请协调者 build 时确认三段是否展开成文件内容。
  - 报错输出按 WRITING.md §2 裁掉了 `--> 文件:行:列` 行；E0583 的本机路径用 `<crate 目录>` 替代（正文已注明）。
  新术语（建议进 GLOSSARY）：模块（module）｜模块；可见性（visibility）｜可见性；命名空间（namespace）｜命名空间；重导出（re-export）｜重导出；依赖（dependency）｜依赖；工作区（workspace）｜工作区；特性（feature）｜建议 `features` 保留英文（Cargo `[features]` 表，第 30 章定）。
  邻章衔接：第 11 章（`tests/` 与 `#[cfg(test)] mod tests` 布局）与第 30 章（features / workspace）各留一句预告；第 12、18 章回指「祖先链全可见才对外可达」；第 33 章接 `extern crate` 与 edition。
  图示需求：「模块树 ↔ 文件树」现用 `text` 树 + 表格；若 08 票统一图示体系，这里值得一张「crate 边界 + 私有链断点（E0603 图景）」示意图。
- 2026-10-18 审校修订（RwCh08 1×major+6×minor，RwCh09 追加 1 条）：
  1. **补 E0616**（补上 ch05 `:595` 的跨章承诺）：在「可见性」小节 `inventory` 例子后新增 `rust,compile_fail` 块（直读私有字段 `item.cost`）+ 1.98 真实输出 `error[E0616]: field \`cost\` of struct \`Item\` is private`（含 `^^^^ private field`），并用 NOTE 收尾，点明「读字段 E0616 / 构造写字段 E0451」（E0451 亦本机实测：`field \`cost\` of struct \`Item\` is private`），并回指第 5 章那句话。该块用**逐字从章文件抽取**的代码复编译，行号/列/插入符与正文粘贴一致（`from_md2.rs:16:25`）。
  2. 「跨 crate 优化」不准确 → 改为「私有项不属于对外接口，编译器可以放心地消除或内联它们；真正跨 crate 的优化要靠单态化与 LTO（第 29 章）」。
  3. `Cargo.lock` 块补回第二行样板注释 `# It is not intended for manual editing.`。
  4. `workspace` → **工作区（workspace）**；**features 决定保留英文**（正文已写「Cargo 术语，本书保留英文」）——请并表时记「保留英文」。
  5. 报错现场小节标题「私有模块：E0603」→「**私有项：E0603**」（该节第二个例子是私有函数）。
  6. `:589` 含混句 → 「如果文件已经放在 `src/config.rs`，它不会作为 `util` 的子模块被找到：要么挪到 `src/util/config.rs`，要么在 `main.rs` 里 `mod config;` 把它升成 crate 根的模块……」。
  7. +RwCh09：shop 运行命令统一 Windows 口径 → `rustc --edition 2024 main.rs -o shop.exe` / `.\shop.exe`（注释给出 Linux/macOS 的 `-o shop` / `./shop`）。注意 `-o` 的名字 rustc **不会**补 `.exe`。
  复跑：`mdbook test -c ch08-modules-packages.md` → **exit 0**（现 14 个 doctest：8 个 `compile_fail` + 6 个 `rust`，4 个 `ignore`）；正文中文 **3668** 字（不含代码块，仍在 3000–4000）。`snippets/ch08/` 仍只有 3 个 `.rs`，无产物。

## Answer

2026-10-18 批次收尾（用户验收通过）。

- 交付：`src/ch08-modules-packages.md`（3668 汉字）+ `snippets/ch08/shop/`（`main.rs` + `catalog.rs` + `catalog/items.rs`，多文件布局）。
- 自验：`mdbook test -c ch08-modules-packages.md` exit 0（14 doctest：8 `compile_fail` + 6 `rust`，4 `ignore`）；三段 include 在 `book/html` 里确认已展开。
- 交叉审校 8 条全修（**major：补 E0616 + E0451，闭合第 5 章的跨章承诺缺口**；跨 crate 优化归因、Cargo.lock 节选标注、features/workspace 体例、标题、根模块措辞、Windows 口径）。全量链：`mdbook build` 0 WARN/ERROR（含 linkcheck2）｜`mdbook test` exit 0（13 章）｜`cargo test --manifest-path examples/Cargo.toml` 19 + 2 passed。
- 遗留不确定：依赖实操走 `--offline`（本机无外网，正文已注明）。
