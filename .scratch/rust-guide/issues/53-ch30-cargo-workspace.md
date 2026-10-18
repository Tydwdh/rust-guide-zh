# 53 第 30 章 Cargo 与 workspace 工程化

Type: task
Status: resolved

## Question

写第五篇第四章：**Cargo 与 workspace 工程化**。

- **范围**：features（定义、`dep:` 语法、可选依赖、默认 feature、feature 统一与 resolver）；workspace（members、`workspace.dependencies`/`package`/`lints` 继承、共享 `Cargo.lock`/`target`）；`[lints]` 表与 lint 层级（衔接到第 32 章）；`rust-version`（MSRV）与 MSRV 感知解析（resolver `"3"`）；build script（`cargo:rerun-if-changed`、`cargo::rustc-cfg` 的 1.77 新写法与旧写法之别——核 Cargo Book）；CI 模板（GitHub Actions 最小可用）；发布与 yank（说明性）。
- **素材**：`research/02` 的 I20 行（`cargo/reference/features.html`、`workspaces.html`、`build-scripts.html`、`lints.html`、`cargo/guide/continuous-integration.html`）；**本仓 `examples/` 是现成的真实 workspace 示例（只读引用，勿改）**；探针可在 `.scratch/tmp-53/` 建独立 workspace 实测。
- **边界**：lint 细节 → 第 32 章；Edition 迁移 → 第 33 章；性能档位 → 第 29 章（回指）；宏 → 第 27/28 章（不展开）。
- **篇幅**：4000–5500 汉字；八节模板；练习 2–3 题带答案；能真跑的代码必须真跑（探针 workspace），说明性的用 `ignore` 并注明原因。
- **DoD**：§8 —— 独占 `src/ch30-cargo-workspace.md` + `snippets/ch30/` + 本票；禁 `git`/禁 build；自验 `mdbook test -c ch30-cargo-workspace.md` 绿；探针 `.scratch/tmp-53/`（独立 workspace 的全套 Cargo.toml 放这里）；收尾 `claimed` + `## Comments`。

## Comments

### 文件（只动了票内列出的）

- `src/ch30-cargo-workspace.md`：占位 H1 保留，正文按八节模板写满；汉字数 5497。
- `snippets/ch30/feature_gate.rs`：feature/cfg 演示（完整可编译、含 `main`），正文用 `{{#rustdoc_include}}` 引入。
- `snippets/ch30/build_script.rs`：build script 示例，正文以 `rust,no_run` 引入（它由 Cargo 在构建期执行，作 doctest 运行没有意义，故只编译不运行）。
- `.scratch/tmp-53/`：探针（独立 workspace + 若干小包），收尾已删除。

### 自验命令与结果

```console
$ mdbook test -c ch30-cargo-workspace.md
 INFO Testing chapter '第 30 章 Cargo 与 workspace 工程化': "ch30-cargo-workspace.md"
exit 0

$ python -c "import re,pathlib;print(len(re.findall(r'[\u4e00-\u9fff]',pathlib.Path('src/ch30-cargo-workspace.md').read_text(encoding='utf-8'))))"
5497
```

全量 `mdbook build` / `mdbook test` 按 §8 留给协调者；我只跑单章。全程禁 `git`。

### 实测过的行为（探针真跑，输出已粘进正文）

1. features 三态与隐式 feature：`cargo run -q -p imgfmt` → `["bmp"]`；`--no-default-features` → `[]`；`--features bmp,png,avif` → `["bmp","png","avif"]`；`--features webp` → `["bmp","webp"]`。
2. feature 统一（并集）：`app` 只请求 `png`，但 `app → tool → imgfmt(avif)`，`cargo run -q -p app` 打出 `imgfmt : ["bmp","png","avif"]`；`cargo tree -e features -i imgfmt`、`cargo tree -f "{p} {f}"` 输出见正文。
3. 统一是**按调用**发生的（练习三依据）：同一个依赖在不同 `-p`/`--workspace` 调用下 feature 集合不同。
4. Cargo 传 feature 的方式：`cargo build -vv -p imgfmt` 抓到 `--cfg "feature=\"bmp\""`（另有 `feature="default"`、`--cfg imgfmt_gpu`）。
5. `dep:` 抑制隐式 feature：`cargo run -q -p imgfmt --features gif` → `error: package ... does not have feature \`gif\`` + help 段（exit 101）→ 报错现场一。
6. `[features]` 里写与可选依赖同名的空条目（`webp = []`）会顶掉隐式 feature：清单加载失败，`optional dependency \`webp\` is not included in any feature`（正文括注）。
7. `[lints]` 生效且**不自动继承**：`cargo check -p a`（有 `[lints] workspace = true`）→ `error: usage of an \`unsafe\` block` + note `requested on the command line with \`-F unsafe-code\``；`cargo check -p b`（无 `[lints]`）→ 通过。
8. `[lints]` 不能「继承 + 覆盖」：`cannot override \`workspace.lints\` in \`lints\`` → 报错现场二。
9. `priority` 落到命令行顺序：`warnings = { level = "deny", priority = -1 }` + `unused = { level = "allow", priority = 1 }` → 构建通过，`cargo build -vv` 抓到 `--deny=warnings --allow=unused`（后者覆盖前者）。
10. Cargo 自己的 lint 系统 nightly-only：`cargo check -Zcargo-lints` → “the `-Z` flag is only accepted on the nightly channel of Cargo”。
11. `rust-version` 门槛：`rust-version = "1.99"` → `error: rustc 1.98.0 is not supported by the following package: rv@0.1.0 requires rustc 1.99`（exit 101）；加 `--ignore-rust-version` 后正常构建。
12. edition 与 MSRV 下界冲突：`edition = "2024"` + `rust-version = "1.60"` → `rust-version 1.60 is incompatible with the version (1.85.0) required by the specified edition (2024)`（exit 101，清单解析阶段）。
13. MSRV 感知解析（resolver `"3"`）：`rust-version = "1.60"` + `itoa = "1"`，`cargo generate-lockfile` → `Locking 1 package to latest Rust 1.60 compatible version` / `Adding itoa v1.0.15 (available: v1.0.18, requires Rust 1.68)`；`CARGO_RESOLVER_INCOMPATIBLE_RUST_VERSIONS=allow` → `itoa v1.0.18`。
14. build script 新旧前缀：`cargo:` 旧写法在 1.98 仍可用、**不报警告**，新旧混用也不报错；未知指令 `cargo::bogus-directive=1` → 硬错误 `invalid output in build script ... Unknown key: \`bogus-directive\`` → 报错现场三。
15. 重跑语义：`cargo build -vv -p imgfmt`，只改 `src/lib.rs` 时**没有** `CARGO_CRATE_NAME=build_script_build` 的 Running 行，改 `build.rs` 时才有；脚本环境里可见 `CARGO_CFG_TARGET_OS=windows`、`CARGO_FEATURE_BMP=1`。
16. 自定义 cfg 与 `unexpected_cfgs`：`app` 里 `cfg!(feature = "bmp")`（app 无此 feature）触发 `warning: unexpected \`cfg\` condition value: \`bmp\``（含 `-F unsafe-code` 之外的第二类落地证据）。
17. 工作区继承限制：成员 `{ workspace = true, default-features = false }` → `error inheriting ... \`default-features = false\` cannot override workspace's \`default-features\``。
18. 共享 lock 与 target：构建后 `ws/Cargo.lock`、`ws/target/` 都在工作区根，成员目录干净。
19. CI 相关：`cargo fmt --all --check` → exit 0；`CARGO_BUILD_WARNINGS=deny cargo build -p app` → exit 101 + `error: warnings are denied by \`build.warnings\` configuration`。
20. 本机工具与文档快照：cargo/rustc 1.98.0、rustfmt 1.9.0-stable、clippy 0.1.98、mdbook 0.5.4；Cargo Book 取自 `…\share\doc\rust\html\cargo\`（features / workspaces / build-scripts / manifest（`[lints]` 表在 manifest 章，不在 lints 章）/ resolver / rust-version / publishing / continuous-integration / faq）。

### 引文

英文原话逐字核对后引用共 9 处（features additive、union、`dep:` 关闭隐式 feature、workspace 定义、继承依赖只能加 `optional`/`features`、`fallback` 语义、`priority` 定义、build script 按 `cargo::` 行解释、yank 两条、`[lints]` 只作用于当前包），全部标注来源为 The Cargo Book；其余为译述并注明。

### 遗留不确定

- `[lints]`「继承与覆写互斥」为 1.98 实测行为，Cargo Book 未记载；正文标注实测并点名上游 issue（rust-lang/cargo#13157，仍开放）。协调者若不愿点名 issue，可删该括注。
- 30.6 的 GitHub Actions YAML **未在真实 runner 上执行**（本机无 CI）。`fmt --check` 与 `build.warnings=deny` 在探针单独跑过；`--locked`、`rustup show`、`actions/checkout@v6` 属模板说明，未实跑。
- 30.7 按票内要求为说明性：`cargo publish --dry-run` / `cargo package --list` 未执行（需凭据与上传动作），yank 语义全部引自 Cargo Book。
- 沙箱不允许直接执行构建产物（`./x.exe` → exit 127），`snippets/ch30/feature_gate.rs` 的运行输出改由「把它作为 `src/main.rs` 放进带 `bmp`/`png` 的小包 + `cargo run -q`」取得（等价，且更贴近正文语境）。
- 30.2 的 `default-members` 缺省行为（虚拟工作区=全体成员；有根包=根包）引自 Cargo Book，未单独建探针实测。

### 新增术语（请协调者并表）

- 构建脚本（build script）
- 虚拟清单（virtual manifest）
- 加法性（additive）
- 统一（unification）
- MSRV（最低支持 Rust 版本）
- yank（保留英文）

### 衔接提示

- 第 31 章可直接把本仓 `examples/` workspace 当统一入口（`cargo test --workspace` 或 `--manifest-path examples/Cargo.toml`）；30.6 已把「测试作为 CI 门禁」接上。
- 第 32 章接 `[lints]` 的**内容**（lint 清单、`#[expect]`、clippy 分组、`deny(warnings)` 取舍）；本章只讲清单层与命令行落点。
- 第 33 章接 edition 迁移；注意 30.4 的「edition 2024 ≥ rust-version 1.85」是清单级硬约束。
- 第 29 章 profile、第 27/28 章宏已按票内边界回指，未展开。

### 图示需求

无。本章可画的空间关系很少，feature 流向用 `cargo tree` 的真实输出（30.1）比示意图更准；按 §9「一张表或三行文字能说清的就不画」处理。

## Comments 追加：RvCh30 审校修复（第二轮）

审校 2 major + 6 minor 全部接受，逐条处理（未反驳任何一条）。新增实测证据均在 `.scratch/tmp-53/` 重建探针后跑出，收尾已删。

### F1（major，30.6 `--locked`）已改

原写法「不写 `--locked` 会按最新兼容版本重新解析」是错的，且与 30.4 自相矛盾。改写为：锁文件满足清单时不写 `--locked` 也不会升级，`--locked` 的语义是「锁文件需要变动时直接失败」。实测（`lock` 探针）：

```console
$ cargo generate-lockfile
     Locking 1 package to latest Rust 1.60 compatible version
      Adding itoa v1.0.15 (available: v1.0.18, requires Rust 1.68)
$ CARGO_RESOLVER_INCOMPATIBLE_RUST_VERSIONS=allow cargo tree -q   # 不写 --locked
lockprobe v0.1.0
└── itoa v1.0.15                    # 锁住就是锁住，没有偷偷升级
$ printf '\nryu = "1"\n' >> Cargo.toml && cargo metadata --locked --format-version 1
error: cannot update the lock file … because --locked was passed to prevent this   # exit 101
```

「锁文件进版本库 → 可复现构建 / `git bisect` 有意义」的论证保留。

### F2（major，30.7 发布）已改

原句「用 `publish = false` 排除纯 `path` 依赖」不成立。实测（`pub` 探针，依赖包 `child` 已写 `publish = false`）：

```text
error: failed to verify manifest at `…\pub\parent\Cargo.toml`

Caused by:
  all dependencies must have a version requirement specified when packaging.
  dependency `child` does not specify a version
```

改写为「要么 `path` 与 `version` 并列（`child = { path = "../child", version = "0.1" }`，打包时 `path` 被剥掉、按版本去 crates.io 找，所以依赖也得先发布）；要么这个包自己也别发布（`publish = false`）」。并列写法的行为也实测过：清单检查通过，随后因 `child` 不在 crates.io 而报 `no matching package named \`child\` found — location searched: crates.io index`，印证「`path` 被剥掉、按版本查 registry」。

### F3（minor，30.2 `resolver`）已改

补上代价句并保留手册原话。实测（`noresolver` 探针：虚拟清单不写 `resolver`，成员 `edition = "2024"`）：

```text
warning: virtual workspace defaulting to `resolver = "1"` despite one or more workspace members being on edition 2024 which implies `resolver = "3"`
```

即：漏写只有警告 + 退回 `"1"`，不是硬错误。小结同步为「虚拟清单要显式 `resolver`（漏写只警告并退回 `"1"`）」。

### F4（minor，30.5 指令表）已改

行改为：`rustc-link-arg=FLAG` / `rustc-link-arg-bin=BIN=FLAG`，并注明 `-bins`、`-tests`、`-examples`、`-benches`、`-cdylib` 同族、`-bin`/`-bins` 形式要带目标名。`rustc-flags`/`metadata` 未加（票内为「视取舍」，本章篇幅与主线不需要）。

### F5（minor，build script 警告可见性）已改

`snippets/ch30/build_script.rs` 注释与 30.5 指令表同步改精确：本地包的 `cargo::warning` **默认可见**，被 Cargo 藏起来的只是脚本的裸 stdout（要看得 `-vv`）。实测（`warn` 探针，普通 `cargo build`，非 `-vv`）：

```console
$ cargo build
   Compiling warnprobe v0.1.0 (…)
warning: warnprobe@0.1.0: hello from build script
    Finished `dev` profile …
```

### F6（minor，feature 名约束）已改

改为：允许 Unicode XID 字符，可用 `_` 或数字开头；**首字符之后**才允许 `-`、`+`、`.`（对应手册 "allows starting with `_` or digits `0` through `9`, and after the first character may also contain `-`, `+`, or `.`"）。

### F7（minor，`build.warnings` MSRV）已改

本机 1.98 快照 config 章原文为 "MSRV: Respected as of 1.97."，正文从「1.98 的配置项」改为「配置项 `build.warnings` 的环境变量形式（Cargo 1.97 起可用）」。

### F8

术语并表口径属协调者，正文未动。

### 修复后自验

```console
$ mdbook test -c ch30-cargo-workspace.md
 INFO Testing chapter '第 30 章 Cargo 与 workspace 工程化': "ch30-cargo-workspace.md"
exit 0
$ python -c "…统计 [\u4e00-\u9fff]…"
5496
```

汉字数仍落在 4000–5500 内（改动净增约 11 字，靠同步压缩 30.3/30.4/练习答案等处的冗句抵消）。本轮只动了 `src/ch30-cargo-workspace.md`、`snippets/ch30/build_script.rs` 与票 53。

## Comments 追加：行内代码配对修正（第三轮，协调者 build 实测反馈）

问题：我原先用 `\``（反斜杠 + 反引号）想在单反引号代码段里「转义」内部反引号，但本机 pulldown-cmark 不认——反斜杠原样渲染、代码段在内部反引号处提前闭合，`["dep:webp"]` 之类的方括号掉出代码段被 linkcheck2 当成潜在链接（Potential incomplete link）。

修正：三处改为**双反引号包裹、内部去反斜杠**：

- `:439` → ``error: warnings are denied by `build.warnings` configuration``
- `:546` → ``optional dependency `webp` is not included in any feature``
- `:638` → ``cannot override `workspace.lints` in `lints` ``（这行以反引号结尾，收尾双反引号前留了一个半角空格，避免三连反引号）

### 渲染自查（私有探针书 `.scratch/tmp-53/book/`，`mdbook build` + linkcheck2，收尾已删）

```console
$ mdbook build            # 探针书含这三行原文；exit 0
 INFO Running the html backend
 INFO Running the linkcheck2 backend
$ grep -c '\\' book/html/t.html
0                                        # 渲染结果里没有残留反斜杠
$ grep -o '<code>error: warnings are denied by [^<]*</code>' book/html/t.html
<code>error: warnings are denied by `build.warnings` configuration</code>
$ grep -o '<code>optional dependency [^<]*</code>' book/html/t.html
<code>optional dependency `webp` is not included in any feature</code>
$ grep -o '<code>cannot override [^<]*</code>' book/html/t.html
<code>cannot override `workspace.lints` in `lints` </code>
$ mdbook build 2>&1 | grep -i "incomplete\|warning"   # 空
```

三处都是单个代码段、无反斜杠；linkcheck2 不再报 Potential incomplete link。另全章复查 `\`` 计数为 0。

### 自验

```console
$ mdbook test -c ch30-cargo-workspace.md
 INFO Testing chapter '第 30 章 Cargo 与 workspace 工程化': "ch30-cargo-workspace.md"
exit 0
```

汉字数 5496 不变（改的是行内代码写法，不涉及中文）。本轮只动 `src/ch30-cargo-workspace.md` 与票 53。

## Answer

2026-10-19 批次收尾（用户验收通过）。

- 交付：`src/ch30-cargo-workspace.md` + `snippets/ch30/`（feature_gate.rs 受测；build_script.rs `no_run` 并注明原因）。
- 自验：`mdbook test -c` exit 0（含注入反证）；15 组行为在独立探针 workspace 真跑（feature 并集/`dep:`/lints 继承与覆写/priority/`rust-version`/resolver 3/`cargo::` 指令/`rerun-if-changed`/`CARGO_BUILD_WARNINGS` 等）。
- 交叉审校 RvCh30（2 major + 6 minor）全修：`--locked` 语义纠正、`publish = false` 不豁免路径依赖、resolver 退回 `"1"` 的代价句、`rustc-link-arg-bin=BIN=FLAG`、`cargo::warning` 可见性、feature 名字符位置约束、1.97 口径。
- 整合期：行内代码 3 处配对修正（linkcheck2 警告清零）；术语并表（模块与工程 12 条：build script/MSRV/虚拟清单/加法性/统一/yank 等）。
- 全量链（批次级）全绿。
- 遗留：CI YAML 未在真实 runner 跑；`cargo publish` 真实上传未执行（依赖无版本时的报错已实测）。
