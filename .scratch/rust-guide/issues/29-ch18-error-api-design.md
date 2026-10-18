# 29 第 18 章 错误处理与 API 设计

Type: task
Status: claimed

## Question

写第 18 章：**错误处理与 API 设计** —— 第 7 章的深化，面向「库作者」的取舍。

- **范围**：库 vs 应用的错误策略（具体错误 enum vs `Box<dyn Error>`/`anyhow` 式；「库不该强推日志/全局状态」）；`From` 转换策略与 `source()` 链的构建（第 7 章的深化）；`#[non_exhaustive]`（错误枚举的演进兼容）；builder 模式与 typestate（编译期状态机）；参数取舍（`&str` vs `String`、`AsRef`/`impl Into` 的边界）；SemVer 与破坏性变更清单（错误类型变大算不算破坏？）；`#[must_use]`、`#[deprecated]` 等 API 防呆手段。
- **素材**：`research/02`（I8）；`rust-lang/api-guidelines`（一手规范；注意它不覆盖 2024 后的语法风格，引用时说明）；std 的 `Error`/`From`/`TryFrom` 页；`reference` 的 attributes 页。
- **边界**：错误处理基础 → 7；`dyn` → 13；文档与 lint 工程 → 32；Edition 迁移 → 33。
- 篇幅 4000–5500 汉字；八节模板；练习 2–3 题带答案；nightly-only 只标注。
- DoD：按 `WRITING.md` §8 批次协议（自验 `mdbook test -c ch18-error-api-design.md`）。

## Comments

- 2026-10-18 初稿：文件 `src/ch18-error-api-design.md`（866 行；正文 5486 汉字，不含围栏代码块）＋片段 `snippets/ch18/{port_config.rs,builder.rs,typestate.rs}`。八节模板齐全（这一章解决什么问题 / 为什么这样设计 / 机制与规则 18.1–18.7 / 动手看看 / 报错现场 ×2 / 小结 / 练习 3 题带答案 / 延伸阅读）。
  自验：`mdbook test -c ch18-error-api-design.md` → **exit 0**（20 个 doctest 全绿，含 5 个 `rust,compile_fail`）。复核门禁有效：临时注入一个坏块 → `running 21 tests` / `20 passed; 1 failed` / exit 101，撤掉后回到 exit 0。
  实测输出（本机 rustc/cargo/clippy **1.98.0**，edition 2024，Windows）：
  1. `port_config.rs` 运行输出：`""` → `配置里缺少端口`（`Missing` 无 source，链在此结束）；`"port = 80a0"` → `端口 "80a0" 不是数字` + `└─ invalid digit found in string`；`"port = 8080"` → `Ok(port = 8080)`；`Box<dyn Error>` 边界两条。
  2. `builder.rs` 输出 `example.com:9000 (重试 3)` / `example.com:443 (重试 0)`；`typestate.rs` 输出 `example.com:8080 (重试 3)` / `example.com:9000 (重试 0)`。
  3. 跨 crate `#[non_exhaustive]`：lib 与 user 两个文件分别真编 → 捕获 E0004 全文（含 note “`PortError` is marked as non-exhaustive, so a wildcard `_` is necessary to match exhaustively”）；另测**变体上**的属性 → E0603（`unit variant A is private` / `tuple variant B is private`）、**struct 与结构体变体上** → E0639（`cannot create non-exhaustive … using struct expression`）；并确认**标在枚举整体上不阻止外部构造既有变体**（`let e = portlib::PortError::Missing;` 通过）。
  4. 现场一：忘了 `From` 时的 `?` → E0277 全文（`` `?` couldn't convert the error to `AppError` `` + `` the trait `From<std::io::Error>` is not implemented for `AppError` `` + note `AppError` needs to implement `From<std::io::Error>`）。
  5. 现场二：库加变体后漏分支 → E0004 全文（含 `help: … adding a match arm with a wildcard pattern`）。
  6. typestate 误用 → E0599（note “the method was found for `ConnBuilder<Present>`”）。
  7. APIT turbofish → E0107（note “`impl Trait` cannot be explicitly specified as a generic argument”）。
  8. `#[must_use]`：函数返回值被丢 → `unused return value of \`write_all\` that must be used`（消息原样进诊断）；方法上同样生效 → `unused return value of \`Token::issue\` that must be used`；类型上 → `unused \`Prepared\` that must be used`；uninhabited 例外据实验证（`#![deny(unused_must_use)]` + `Result<(), Never>` / `ControlFlow<Empty>` 通过、无警告）。
  9. `#[deprecated(since, note)]` 警告全文；`main -> Result<(), Box<dyn Error>>` → 打印 `Error: MyErr`、退出码 1（chapter 里那段 `no_run` 块的源码原样跑过）。
  10. `clippy::result_large_err`（默认 warn；诊断 “the `Err`-variant is at least 256 bytes”、help 建议装箱）与 `clippy::ptr_arg`（`&String` → help `change this to &str`）。
  11. 参数可接受性：`impl AsRef<str>` 收 `&String`/`String`/`&str`；`impl AsRef<Path>` 收 `&str`/`String`/`&String`/`&Path`/`PathBuf`；`impl Into<String>` 收 `&str`/`String`；`&String` 经解引用强转进 `&str`（表中「调用者能传」一列全部实测）。
  12. `dyn Error::downcast_ref` 取回具体类型（`impl dyn Error` 上的签名要求 `T: Error + 'static`；另在 `dyn Error + Send`、`dyn Error + Sync + Send` 各有转发）；`impl<'a, E: Error + 'a> From<E> for Box<dyn Error + 'a>` 证实「`?` 通吃」的理由。
  一手来源核对：std `Error` / `From` / `TryFrom` / `io::Error::new` 的签名与说明取自本机 1.98 文档快照（`share/doc/rust/html/`）与 rust-src；Cargo 手册 semver 章、Reference 的 `attributes/type_system` 与 `attributes/diagnostics` 页逐字核对后引用（引号内均为原文）；api-guidelines 的 C-GOOD-ERR / C-CONV-TRAITS / C-BUILDER / C-CALLER-CONTROL / C-GENERIC 原文取自在线一手站（about 页自称 “only guidelines … not … a mandate”，已按票要求写进 `> [!NOTE]` 适用范围盒）。
  遗留不确定：
  - Cargo semver 页在 `#[non_exhaustive]` 的后果里写了一句 “Casting enum variants to their discriminant with `as` is not allowed.” —— 1.98 **复现不出**：fieldless 的 `#[non_exhaustive]` 枚举做 `as u8` 正常编译（E0605 只针对非 fieldless 枚举）。正文回避了这条断言，未写进表。
  - Reference 的 `must_use` 允许位置列表（Struct/Enumeration/Union/Function/Trait）**未列方法**，但 1.98 实测方法有效；正文按实测写并说明。
  - api-guidelines 正文里引用的 `error-chain` / `reqwest` 链接明显过旧，未沿用；只取规则文字。
  - 「库不该推日志 / 全局状态」这条按票要求写了，但它不是 Guidelines 的条目：正文以 The Book「让使用者自己决定」的原话作支撑，其余作为该原则的推论陈述。
  - MSRV 与 Cargo feature 只进了 SemVer 表，未展开（→ 第 30 章）；`Try` trait / `error_generic_member_access` 未展开（nightly）。
  新术语（English | 建议中文；均未自建文件，由协调者并表）：
  - typestate | 保留英文（首现给「用类型表示值的状态」的解释）；候选别名「类型状态」
  - builder 模式 | 保留英文 builder（候选「构建器模式」）
  - sealed trait | 密封 trait（候选「封闭 trait」）
  - SemVer / semantic versioning | 语义化版本（缩写 SemVer 保留）
  - breaking change | 破坏性变更
  - uninhabited | 无值类型
  - 建议**不收录**「防呆」；正文改用「把误用挡在编译期」表述
  - error chain / error propagation / supertrait / lint / span / 孤儿规则 / derive | 已在 GLOSSARY，未改
  邻章衔接：第 7 章（基础错误处理；本章回指 7.3/7.4/7.6 共 3 处，并把首页那句「错误链怎么被上层利用属于 API 设计，第 18 章展开」兑现）；`dyn` 的不透明度与 `downcast` → 第 13 章；sealed trait 写法 → 第 12 章；`PhantomData` 零开销 → 第 15 章；孤儿规则 → 第 9 章；Cargo feature / `[lints]` / MSRV → 第 30、32 章；Edition 迁移 → 第 33 章。正文只用「（第 N 章）」式提示，未展开他章内容。
  图示需求（本轮按票要求只用表格/`text` 块，未自画 SVG）：
  - §18.1「库 ↔ 应用 的责任分工与错误类型形状」——现为 5 行表格，适合升级成一张分层示意图（库侧具体 enum / 应用侧 `Box<dyn Error>`，中间是 `source()` 与 `?`）。
  - §18.3「`#[non_exhaustive]` 在定义 crate 内 / 外的语义差异」——现为 3 行表格（枚举 / 变体 / struct 三种位置），适合画成「同一份代码，两个视角」的对照图。
  - §18.6「一次改动 → major / minor / possibly-breaking」判定——现为 17 行表格，图非必需。
  主动取舍：SemVer 表只保留与错误类型/API 形状相关的条目（未抄全表）；`#[non_exhaustive]` 的 `as` 转换一条因复现不出而略去；未设「语言对照角」（跨语言断言需另核一手文档，本轮从略）；`must_use` 的 uninhabited 例外保留为一个可跑断言块（不是纯叙述），因此章节的 doctest 全绿即是该断言的验证。
