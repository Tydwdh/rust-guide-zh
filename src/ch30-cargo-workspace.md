# 第 30 章 Cargo 与 workspace 工程化

## 这一章解决什么问题

前 29 章的内容都在一个 crate 内部：类型、所有权、trait、并发、unsafe。真实项目里，代码外面还有一层同样会出错的东西——**构建配置**。它要回答的问题长这样：

- 一个库想同时支持「精简」和「全功能」两种用法，怎么把功能拆成可组合的开关，而不是到处铺 `#[cfg]`？
- 项目长到十几个 crate，每个 `Cargo.toml` 都抄一遍版本号、edition、依赖版本，改一处漏两处，怎么统一？
- 你在 README 里写的「最低支持 Rust 1.8x」怎么变成机器能检查的约束，而不是一句承诺？
- 编译前必须先跑的那一步（生成代码、编一段 C、探测平台）写在哪里，Cargo 又怎么知道什么时候该重跑它？
- CI 上到底跑什么命令，才能把上面这些约束变成门禁？

读完这一章你应当能：

- 给库设计一套**加法性** features，分清可选依赖的隐式 feature 与 `dep:` 的关系；
- 把多个 crate 收进工作区（workspace），用 `workspace.package`、`workspace.dependencies`、`workspace.lints` 统一版本、依赖与 lint；
- 写一份成员能继承的 `[lints]` 表，知道 `priority` 怎么落到 rustc 命令行上；
- 声明 `rust-version`，并让解析器按 MSRV 选依赖版本（resolver `"3"`）；
- 写 build script：`cargo::` 指令、`rerun-if-changed` 的重跑语义、自定义 cfg 与 `rustc-check-cfg`；
- 搭一份最小 CI，并说清 `cargo publish` 与 `cargo yank` 的语义差别。

**边界**：lint 的完整清单与取舍在第 32 章；edition 与迁移在第 33 章；`profile`（`dev`/`release` 档位、优化设置）在第 29 章；宏不在本章展开（第 27、28 章）。本章的 Cargo 行为以 Rust 1.98 / Cargo 1.98 实测为准（快照文档取自本机 1.98 文档目录）。

## 为什么这样设计

**features 是「编译期开关 + 可选依赖」，代价是它必须可叠加。** Cargo 手册把这条规则叫加法性：原文是 "A consequence of this is that features should be additive. That is, enabling a feature should not disable functionality, and it should usually be safe to enable any combination of features."（引自 The Cargo Book）为什么？因为同一个依赖在一次构建里只会被编译一次，它的 feature 是**所有使用者要求的并集**。手册的另一句写得很直接："When a dependency is used by multiple packages, Cargo will use the union of all features enabled on that dependency when building it."（引自 The Cargo Book）

这条规则推翻两种直觉。其一，**feature 不能表达「二选一」**（如「用 OpenSSL 还是用 rustls」）：你没法保证别人的构建图不会同时打开两者。其二，**功能要用正向名字**：同时支持 `std` 与裸机的库该暴露 `std = []`，而不是 `no_std` feature——「打开」对别人的构建图永远安全，「关掉」不是。

**工作区解决的是「一份锁、一份 target、一套版本」。** Cargo 手册对它的定义是 "A workspace is a collection of one or more packages, called workspace members, that are managed together."（引自 The Cargo Book），其中两条关键约定是：所有包共享位于工作区根的 `Cargo.lock` 与 `target` 目录。共享锁意味着整张依赖图被一次性解析成**一套确定版本**：成员之间不会出现「同一个依赖的两个 SemVer 不兼容版本」。代价是成员必须相容——你最低支持 1.70、他 1.85，工作区整体只能按最保守的那个来。

**继承表解决抄写问题。** 版本号、edition、license 写一次，成员用 `version.workspace = true` 引用；依赖版本写一次，成员按需**叠加** feature——是叠加不是覆写，成员侧能加的键只有 `optional` 与 `features`，"Other than optional and features, inherited dependencies cannot use any other dependency key (such as version or default-features)."（引自 The Cargo Book）。

**MSRV 是一份承诺，Cargo 把它变成三件机器可做的事。** 写 `rust-version = "1.85"` 之后：**门槛**——用更旧的工具链构建直接报错；**选择**——`cargo add` 挑与它兼容的依赖版本；**解析**——resolver `"3"` 把「依赖的 rust-version 是否超过我声明的 MSRV」纳入版本挑选。MSRV 不只是文档，还是解析器的一条输入。

**build script 是「编译前的一段程序」，协议是 stdout 而不是 API。** 有些事编译器干不了：调 `cc` 编 C、从 `.proto` 生成代码、探测平台。这类步骤需要一个约定的执行时机（本包编译之前）与一份约定的输入输出（环境变量进、指令行出）。用 stdout 传指令是刻意的低耦合设计，代价是**没有类型检查**——指令名打错要等到运行才暴露（见「报错现场」）。

**发布是不可逆的，所以 yank 不是删除。** 手册写得很清楚："A yank does not delete any code."（引自 The Cargo Book），"The semantics of a yanked version are that no new dependencies can be created against that version, but all existing dependencies continue to work."（引自 The Cargo Book）。crates.io 的定位是「永久档案」：删版本会让依赖过它的构建在某个时刻无声崩掉。

## 机制与规则

### 30.1 features：定义、可选依赖与统一

features 定义在 `[features]` 表里，每条是一个名字加一串「它打开什么」：

```toml
[features]
default = ["bmp"]
bmp = []
png = []
ico = ["bmp", "png"]
```

名字允许 Unicode XID 字符，可用 `_` 或数字开头；首字符之后还允许 `-`、`+`、`.`（crates.io 收紧到 ASCII 字母数字与 `_`、`-`、`+`）。`default` 是普通 feature，只是默认打开。

Cargo 打开 feature 的方式就是给 rustc 传 cfg；在探针里用 `cargo build -vv` 抓命令行，能看到：

```text
--cfg "feature=\"bmp\""  --cfg "feature=\"default\""  --cfg imgfmt_gpu
```

（后一条来自 build script，见 30.5。）所以代码里用 `#[cfg(feature = "bmp")]` 判断，用 `cfg!(feature = "bmp")` 在运行期读同一个值：

```rust
{{#rustdoc_include ../snippets/ch30/feature_gate.rs}}
```

把这份文件放进一个声明了 `bmp`、`png` 两个 feature 的包里跑，三种命令行给出三种结果（本机实测）：

```console
$ cargo run -q
$ cargo run -q --features bmp
$ cargo run -q --features bmp,png
```

```text
backend = cpu
enabled = (无)
backend = bmp
enabled = bmp
backend = bmp
enabled = bmp png
```

三个开关是 `--features`（多个用逗号或空格分隔）、`--no-default-features`、`--all-features`；工作区里还能用 `包名/feature` 指定给谁开：`cargo build -p app -p tool --features app/foo,tool/bar`。从 resolver `"2"` 起，`--features` 作用于命令行上选中的任何包；`"1"` 只认当前目录那个包。

**可选依赖与隐式 feature。** 把依赖标成 `optional = true`，它默认不参与编译：

```toml
[dependencies]
gif = { version = "0.11.1", optional = true }
```

Cargo 会为此自动补一条同名 feature，等价于 `gif = ["dep:gif"]`。于是 `--features gif` 既能打开这个 feature、也会拉进这个依赖。但有时你不想暴露这个同名开关（它是内部细节，或你想把多个可选依赖打包成一个 feature），这时用 `dep:` 明确引用它：

```toml
[dependencies]
ravif = { version = "0.6.3", optional = true }
rgb = { version = "0.8.25", optional = true }

[features]
avif = ["dep:ravif", "dep:rgb"]
```

手册的原话是："If you specify the optional dependency with the dep: prefix anywhere in the features table, that disables the implicit feature."（引自 The Cargo Book）——从此 `ravif`、`rgb` 只是依赖名，不再是 feature 名。顺带提醒：`dep:` 语法从 Rust 1.60 才有。

还有两个语法值得记住：

- `包名/feature`：打开某个依赖的 feature，**同时**会打开那个可选依赖本身（如果它还是可选的）。
- `包名?/feature`：只在该依赖已被别处打开时，才给它附加这个 feature。适合「我的序列化支持要顺带打开 `rgb` 的 `serde`，但不该因为这就把 `rgb` 拉进来」。

**统一（unification）与它的结果。** 前面说过依赖的 feature 取并集，下面这个探针场景把后果摆出来：`app` 依赖 `imgfmt`（默认 features + `png`），`tool` 依赖 `imgfmt`（`avif`），而 `app` 又依赖 `tool`：

```console
$ cargo run -q -p app
```

```text
app    : bmp = false
gif    : GIF
tool   : AVIF+GIF
imgfmt : ["bmp", "png", "avif"]
backend: gpu
target : x86_64-pc-windows-msvc
```

`app` 从没要求过 `avif`，但 `imgfmt` 在这次构建里带着 `bmp`、`png`、`avif` 一起被编了出来。`app` 第一行还说明：**feature 属于定义它的包**——`app` 自己没有 `bmp` 这个 feature，所以 `cfg!(feature = "bmp")` 是 `false`（1.98 为此会发 `unexpected_cfgs` 警告，见 30.5）。

想看清「谁开了什么」，用 `cargo tree`：

```console
$ cargo tree -e features -i imgfmt
```

```text
imgfmt v0.1.0 (…\ws\imgfmt)
├── imgfmt feature "avif"
│   └── tool v0.1.0 (…\ws\tool)
│       └── tool feature "default" (command-line)
│           └── app v0.1.0 (…\ws\app)
│               └── app feature "default" (command-line)
├── imgfmt feature "bmp"
│   └── imgfmt feature "default" (command-line)
│       ├── app v0.1.0 (…\ws\app) (*)
│       └── tool v0.1.0 (…\ws\tool) (*)
├── imgfmt feature "default" (command-line) (*)
└── imgfmt feature "png"
    └── app v0.1.0 (…\ws\app) (*)
```

`-e features` 展开 feature 节点，`-i imgfmt` 反着显示「feature 怎么流进 imgfmt 的」。另外两个常用写法：`cargo tree -f "{p} {f}"` 列出每个包打开的 feature；`cargo tree --duplicates` 找出被编译多次的依赖。

> [!NOTE]
> resolver `"2"`（edition 2021 起的默认值）在三种情况下**不**统一：目标平台不匹配的条件依赖被忽略；build-dependencies 与过程宏不和普通依赖共享 feature；dev-dependencies 除非真要构建测试/示例，否则不激活 feature。这样「构建期需要 std」和「运行期 no_std」的组合才不会被并集毁掉，代价是同一个包可能编译两遍（用 `--duplicates` 找）。

### 30.2 workspace：成员、继承与共享

工作区有两种形态。根清单里同时有 `[package]` 和 `[workspace]`，根包就是成员之一；只有 `[workspace]` 没有 `[package]`，叫**虚拟清单**，用于「没有主包，一堆平行 crate」。本章的探针用的是虚拟清单：

```toml
[workspace]
resolver = "3"
members = ["gif", "webp", "imgfmt", "app", "tool"]

[workspace.package]
version = "0.1.0"
edition = "2024"
rust-version = "1.85"
license = "MIT OR Apache-2.0"

[workspace.dependencies]
gif = { path = "gif" }
webp = { path = "webp" }
imgfmt = { path = "imgfmt" }

[workspace.lints.rust]
unsafe_code = "forbid"

[workspace.lints.clippy]
dbg_macro = "deny"
```

`members` 支持 `crates/*` 这样的 glob；工作区目录里的 path 依赖自动成为成员；`exclude` 踢掉路径；`default-members` 决定在根目录敲 `cargo build` 时构建谁（不写就是全体成员；有根包时只有根包）。

虚拟清单里的 `resolver` **必须显式写**：它没有 `package.edition` 可以推断（手册的说法是"they have no package.edition to infer it from"）。漏写不是硬错误——实测只给一条警告，然后**退回 `"1"`**，feature 统一行为跟着变回老样子：

```text
warning: virtual workspace defaulting to `resolver = "1"` despite one or more workspace members being on edition 2024 which implies `resolver = "3"`
```

edition 2024 对应的默认值是 `"3"`，所以我们写出来。

`workspace.package` 能继承的键是固定的一批：`version`、`edition`、`rust-version`、`authors`、`license`、`repository`、`description`、`keywords` 等。成员侧写 `{键}.workspace = true`：

```toml
[package]
name = "imgfmt"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
```

依赖同理，但成员侧只能加 `optional` 和 `features`。下面这条曾经让我在探针里栽了一跤——成员想关掉继承来的默认 features：

```toml
[dependencies]
imgfmt = { workspace = true, default-features = false, features = ["png"] }
```

Cargo 拒绝：

```text
  error inheriting `imgfmt` from workspace root manifest's `workspace.dependencies.imgfmt`

Caused by:
  `default-features = false` cannot override workspace's `default-features`
```

想关默认 features，就把它写进 `[workspace.dependencies]` 那一条，全体成员一致：

```toml
[workspace.dependencies]
imgfmt = { path = "imgfmt", default-features = false }
```

工作区共享 `Cargo.lock` 与 `target`，实测确认：在根目录构建之后，`Cargo.lock` 与 `target/` 都出现在工作区根，成员目录里是干净的。

> [!TIP]
> 本仓库 `examples/` 就是一个同构的真实工作区（`resolver = "3"`，六个成员）。在 `examples/` 目录里跑 `cargo test`，或在仓库根跑 `cargo test --manifest-path examples/Cargo.toml`，就能看到「一条命令跑全部成员」。

### 30.3 `[lints]`：把 lint 配置写进清单

`[lints]` 把 lint 配置从源码顶部的 `#![warn(...)]` 挪进清单，并按**工具**分表：`::` 之前的部分就是表名，`unsafe_code` 归 `rust`，`clippy::enum_glob_use` 归 `clippy`（也有 `rustdoc`）：

```toml
[lints.rust]
unsafe_code = "forbid"
unused = { level = "allow", priority = 1 }
warnings = { level = "deny", priority = -1 }

[lints.clippy]
enum_glob_use = "deny"
```

写字符串是简写，等价于 `{ level = "…", priority = 0 }`。`level` 就是 rustc 的四档：`forbid`、`deny`、`warn`、`allow`。`priority` 是**有符号整数**，手册说 "priority is a signed integer that controls which lints or lint groups override other lint groups"：数字小（尤其负数）的优先级低，会被数字大的覆盖，并按由小到大的顺序出现在传给 rustc 的命令行上。

上面那段配置正是「命令行顺序」的实例。实测 `cargo build -vv` 抓到：

```text
--deny=warnings --allow=unused
```

`warnings` 是组、`unused` 是组，两者在命令行上一前一后，**后面的赢**——所以未使用变量不会因为「deny warnings」而失败。这就是 `priority` 的全部意义：它是给「组与组、（继承来的）条目之间打架」排顺序用的。

`[lints]` 也可以交给工作区统一。`[workspace.lints]` 与 `[lints]` 结构相同，成员用一行接上：

```toml
[lints]
workspace = true
```

有两点实测过才会记住。

第一，**不继承就是真的不继承**。探针里有两个成员：`a` 写了 `workspace = true`，`b` 什么都没写，而工作区的 `unsafe_code = "forbid"`。结果是 `a` 直接编译失败：

```text
error: usage of an `unsafe` block
 --> a\src\lib.rs:3:5
  |
3 |     unsafe { *p }
  |     ^^^^^^^^^^^^^
  |
  = note: requested on the command line with `-F unsafe-code`
```

`b` 照常编过。注意 `-F unsafe-code` 这一行——它就是 `[lints]` 的落地方式：Cargo 把配置翻译成 rustc 的 lint 级命令行参数。Cargo 自己有一条 lint 专门提醒这个坑（`missing_lints_inheritance`：工作区有 `workspace.lints` 而成员没有 `[lints]` 表），但它属于 `-Zcargo-lints` 系统，1.98 **只在 nightly 可用**（stable 上会报 "the `-Z` flag is only accepted on the nightly channel of Cargo"）。

第二，**继承与覆写不能并存**。想在成员里改一条继承来的 lint 是常见需求，但 Cargo 1.98 直接拒绝：

```text
error: failed to load manifest for workspace member `…\lints\c`
referenced by workspace at `…\lints\Cargo.toml`

Caused by:
  failed to parse manifest at `…\lints\c\Cargo.toml`

Caused by:
  cannot override `workspace.lints` in `lints`, either remove the overrides or `lints.workspace = true` and manually specify the lints
```

规则是二选一：要么只写 `workspace = true`（全盘继承，不能加任何 lint 表），要么放弃继承、手写完整的一套。（上游仍在讨论「继承 + 局部覆写」的共存 —— rust-lang/cargo#13157。）

另外两点：`[lints]` 只作用于**当前包**——手册的措辞是 "Cargo only applies these to the current package and not to dependencies."；反过来，你的包被别人引用时 Cargo 会用 `--cap-lints` 压掉 lint 输出，别把「lint 全开」当成对下游的强制。lint 项的取舍与分组策略在第 32 章。

### 30.4 `rust-version`（MSRV）与 resolver 3

`rust-version` 写的是一个**裸版本号**，至少一个分量，不许带 `>=` 之类的运算符或预发布标识：

```toml
[package]
name = "rv"
version = "0.1.0"
edition = "2024"
rust-version = "1.85"
```

它的第一个效果是门槛。把上面那个包写成 `rust-version = "1.99"`，在本机 Cargo 1.98 上构建：

```text
error: rustc 1.98.0 is not supported by the following package:
  rv@0.1.0 requires rustc 1.99
```

exit code 101。想强行构建可以加 `--ignore-rust-version` 跳过这个检查。另一条容易撞上的约束是**与 edition 的下界冲突**：edition 2024 至少要求 Rust 1.85，写 `edition = "2024"` 加 `rust-version = "1.60"` 会在解析清单时报错：

```text
error: failed to parse manifest at `…\edition-msrv\Cargo.toml`

Caused by:
  rust-version 1.60 is incompatible with the version (1.85.0) required by the specified edition (2024)
```

第二个效果是**影响依赖版本的选择**，这由 resolver 版本与一个配置项共同决定。resolver `"3"`（edition 2024 的默认值，需要 Cargo 1.84+）只做了一件事：把 `resolver.incompatible-rust-versions` 的默认值从 `allow` 改成 `fallback`。`fallback` 的含义是：挑选依赖版本时，**优先选那些 rust-version 不高于你自己的**。手册的说明是 "With the fallback setting, the resolver will prefer packages with a Rust version that is less than or equal to your own Rust version."（引自 The Cargo Book）

实测最直观。一个 `rust-version = "1.60"`、`resolver = "3"` 的包依赖 `itoa = "1"`：

```console
$ cargo generate-lockfile
     Locking 1 package to latest Rust 1.60 compatible version
      Adding itoa v1.0.15 (available: v1.0.18, requires Rust 1.68)
```

它没挑最新的 1.0.18，而是挑了对 1.60 友好的 1.0.15，并明确告诉你「最新的那个为什么要不了」。把开关拨回去：

```console
$ CARGO_RESOLVER_INCOMPATIBLE_RUST_VERSIONS=allow cargo generate-lockfile
     Locking 1 package to latest compatible version
```

这次锁的是 `itoa v1.0.18`。所以「MSRV 感知的解析」不是一个隐式行为，而是一个有名字、可开关的策略。

三点补充：

- **它只影响「重新解析」的那一刻**：有 `Cargo.lock` 时按锁文件走，改配置也不会悄悄升降级；想重新挑版本要 `cargo update`。
- **MSRV 与 edition 是两件事**：resolver `"3"` 也能给非 2024 的包显式指定（上面那个包用的就是 `edition = "2021"`）。
- **声明不等于验证**：`rust-version` 只是承诺，验证要靠 CI 用该版本工具链跑一遍（见 30.6）。

### 30.5 build script：编译前的那一步

包目录下有 `build.rs` 就会被自动编译执行（也可用 `build = "build.rs"` 指定、`build = false` 关掉）。它在**本包编译之前**运行，工作目录是本包根目录，通信方式是把 `cargo::` 开头的行打到 stdout；手册说："Cargo will interpret each line that starts with cargo:: as an instruction that will influence compilation of the package."（引自 The Cargo Book）

下面这份是探针里真跑过的 build script：

```rust,no_run
{{#rustdoc_include ../snippets/ch30/build_script.rs}}
```

它用到 `rerun-if-changed`、`rustc-check-cfg` + `rustc-cfg`、`rustc-env` 与 `warning` 几组指令：前者缩小重跑条件，中间两条打开一个自定义 cfg，`rustc-env` 把目标三元组写进编译期环境变量。构建本包时，Cargo 会把 `--cfg imgfmt_gpu` 加进 rustc 命令行（30.1 抓到的就是它），`env!("IMGFMT_TARGET")` 也就能在代码里读到。

常用指令一览（都是新写法）：

| 指令 | 作用 |
|---|---|
| `rerun-if-changed=PATH` | 只有这个路径（文件或目录）变了才重跑 |
| `rerun-if-env-changed=VAR` | 只有这个环境变量变了才重跑 |
| `rustc-cfg=KEY[="VALUE"]` | 给本包传 `--cfg` |
| `rustc-check-cfg=CHECK_CFG` | 声明「会有这个 cfg」，供 `unexpected_cfgs` 检查 |
| `rustc-env=VAR=VALUE` | 编译本包时设置环境变量（代码里 `env!` 读） |
| `rustc-link-lib=LIB` / `rustc-link-search=[KIND=]PATH` | 交给链接器（第 26 章的 `c26` 库就是这么链的） |
| `rustc-link-arg=FLAG` / `rustc-link-arg-bin=BIN=FLAG` | 给链接器加参数；`-bins`、`-tests`、`-examples`、`-benches`、`-cdylib` 是同族变体，`-bin`/`-bins` 的写法要带目标名 |
| `warning=MESSAGE` / `error=MESSAGE` | 在终端打警告/错误（`error` 会停构建）；本地包的 `warning` 默认可见，平时被藏起来的只是脚本的裸 stdout |

`cargo::` 前缀（双冒号）从 1.77 起可用，此前只有单冒号的 `cargo:`。1.98 实测**旧写法仍然可用、不会报警告**，新旧混用也不报错；新项目直接用双冒号（它同时是 `cargo::metadata` 这类指令的唯一写法）。但两种前缀对「不认识的键」态度不同，这是本章「报错现场」之一。

**重跑语义**必须实测才清楚。默认情况下，包里任何文件变化都会让 build script 重跑；一旦你输出了任何 `rerun-if-changed`，就只看这些路径。探针里用 `cargo build -vv` 看判定：

```console
$ cargo build -vv -p imgfmt          # 只改了 src/lib.rs
       Dirty imgfmt v0.1.0 (…\ws\imgfmt): the file `imgfmt\src\lib.rs` has changed (…)
   Compiling imgfmt v0.1.0 (…\ws\imgfmt)
```

注意这一轮**没有** `build_script_build` 的 "Running" 行：build script 没重跑，只有 rustc 重跑。改成 `build.rs` 之后：

```console
$ cargo build -vv -p imgfmt          # 改了 build.rs
       Dirty imgfmt v0.1.0 (…\ws\imgfmt): the file `imgfmt\build.rs` has changed (…)
     Running `… CARGO_CRATE_NAME=build_script_build …`
```

这次脚本重跑了。顺带可见：脚本进程的环境里有 `CARGO_CFG_TARGET_OS=windows`、`CARGO_FEATURE_BMP=1`（打开的 feature，大写、`-` 变 `_`）这类变量——build script 判断平台与 feature 靠它们，**不要**在这个上下文用 `cfg!`/`#[cfg]`：那判断的是宿主机，交叉编译时就会答错。

**自定义 cfg 必须成对。** 你让 Cargo 传 `--cfg imgfmt_gpu`，编译器却不知道这个 cfg 是预期中的——用它就会触发 `unexpected_cfgs`。所以要先 `rustc-check-cfg=cfg(imgfmt_gpu)`。这条 lint 对 feature 也一样生效：探针的 `app` 包里写了 `cfg!(feature = "bmp")`，但 `app` 自己没声明过 `bmp` 这个 feature，于是：

```text
warning: unexpected `cfg` condition value: `bmp`
 --> app\src\main.rs:2:40
  |
2 |     println!("app    : bmp = {}", cfg!(feature = "bmp"));
  |                                        ^^^^^^^^^^^^^^^ help: remove the condition
  |
  = note: no expected values for `feature`
  = help: consider adding `bmp` as a feature in `Cargo.toml`
```

这不是噪音而是免费的检查：它会替你抓住「feature 名打错」和「在错误的包里查 feature」。

> [!WARNING]
> build script 只能往 `OUT_DIR`（Cargo 提供的环境变量）里写文件。它会拖慢构建——重活尽量少做，并记得补 `rerun-if-changed`，否则改任何源文件都要重跑。

### 30.6 CI：把上面的约束变成门禁

最小可用的 CI 只做四件事：格式、lint、测试、构建。下面这份 GitHub Actions 模板就是 30.1–30.5 那些约束的落地（本仓 `.github/workflows/book.yml` 是同一套思路）：

```yaml
name: ci

on:
  push:
  pull_request:

env:
  CARGO_TERM_COLOR: always
  CARGO_BUILD_WARNINGS: deny   # 等价于 build.warnings = "deny"

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v6
      - run: rustup show                 # 有 rust-toolchain.toml 时按它装工具链
      - run: cargo fmt --all --check
      - run: cargo clippy --workspace --all-targets --all-features --keep-going
      - run: cargo test --workspace --locked
```

几个要点：

- `CARGO_BUILD_WARNINGS=deny` 是配置项 `build.warnings` 的环境变量形式（Cargo 1.97 起可用），把「本包 lint 警告」升级为失败。实测在探针工作区里，被警告的包会让构建以 101 退出，并打印 ``error: warnings are denied by `build.warnings` configuration``。它只管 lint 警告，不会把依赖那边的输出也变成错误。
- 锁文件满足清单时，不写 `--locked` 也**不会**帮你升级：锁在 1.0.15 就是 1.0.15（把 MSRV 策略拨成 `allow` 也一样）。`--locked` 的作用是「锁文件需要变动时**直接失败**」，实测清单加了新依赖后它报 `error: cannot update the lock file … because --locked was passed to prevent this`。CI 靠它钉住版本。库的锁文件不影响下游（下游只看 `Cargo.toml`），所以「要不要提交锁文件」对应用和库不一样。
- `--workspace` 在虚拟工作区里等于构建全体成员；只测某个包用 `-p`。feature 统一是**按调用**发生的（30.1 实测过）：`-p app` 与 `--workspace` 可能在同一个依赖上得到不同的 feature 集合。
- MSRV 值得单独一条 job：装目标版本工具链跑 `cargo check`（多数 MSRV 问题是 API 不存在而非行为不符）。手册的例子用第三方 `cargo hack check --rust-version --workspace --all-targets --ignore-private`。
- `cargo fmt --all --check` 与 `clippy` 在 CI 上固定工具链版本更稳：新版本会加新 lint，可能在没人改代码时把 CI 弄红。

### 30.7 发布与 yank（说明性）

发布只需要 `cargo publish`，但清单得先满足几件事：`version` 按 SemVer 走、`description`/`license` 要有、**每个依赖都得写 `version` 要求**。纯 `path` 依赖过不了检查，给依赖加 `publish = false` 也不豁免：

```text
error: failed to verify manifest at `…\pub\parent\Cargo.toml`

Caused by:
  all dependencies must have a version requirement specified when packaging.
  dependency `child` does not specify a version
```

修法是让 `path` 与 `version` 并列（`child = { path = "../child", version = "0.1" }`——打包时 `path` 被剥掉、按版本去 crates.io 找，所以那个依赖也得先发布），或者这个包也别发布（`publish = false`）。发布前建议先：

```console
$ cargo publish --dry-run
$ cargo package --list
```

`--dry-run` 打包但不上传，产物在 `target/package/` 下；`--list` 列出最终进包的文件，用来抓「漏了文件」或「把大资产塞进包」。

发布之后如果发现某个版本是坏的（语法错误、漏文件、忘了带上必需资源），用 yank：

```console
$ cargo yank --version 1.0.1
$ cargo yank --version 1.0.1 --undo
```

它的语义前面说过：不删代码，只是让**新的**依赖解析不再选中它，已有的 `Cargo.lock` 不受影响——所以它救不了「手滑上传密钥」这类事故，那种情况必须立刻作废密钥。协作靠 `cargo owner --add/--remove`；另外 yank 过的版本号不能重用，修复要发新版本。

## 动手看看

下面这套工作区（四个成员加一个可选依赖包）就是探针，全部输出都在本机跑出来过。根清单见 30.2；`imgfmt` 的清单是这样的：

```toml
[package]
name = "imgfmt"
version.workspace = true
edition.workspace = true
rust-version.workspace = true

[features]
default = ["bmp"]
bmp = []
png = []
avif = ["dep:gif"]

[dependencies]
gif = { workspace = true, optional = true }
webp = { workspace = true, optional = true }

[lints]
workspace = true
```

注意 `gif` 用 `dep:gif` 引用（同名隐式 feature 被抑制），而 `webp` 没在 `[features]` 里出现（隐式 feature `webp` 保留）。这就是同一份清单里两种风格并存的样子：

```console
$ cargo run -q -p imgfmt
imgfmt : ["bmp"]
backend: gpu
$ cargo run -q -p imgfmt --no-default-features
imgfmt : []
backend: gpu
$ cargo run -q -p imgfmt --features bmp,png,avif
imgfmt : ["bmp", "png", "avif"]
backend: gpu
$ cargo run -q -p imgfmt --features webp
imgfmt : ["bmp", "webp"]
backend: gpu
```

`backend: gpu` 来自 build script 打开的 cfg（30.5）；`webp` 那轮说明隐式 feature 仍能拉进可选依赖，而 `gif` 没得选：

```console
$ cargo run -q -p imgfmt --features gif
error: package `imgfmt v0.1.0 (…\ws\imgfmt)` does not have feature `gif`
```

完整报错见下一节。feature 怎么流动看 `cargo tree -e features -i imgfmt`（30.1）；构建脚本何时重跑看 `cargo build -vv`（30.5）。想看继承的效果，把 `imgfmt/Cargo.toml` 的 `[lints] workspace = true` 删掉再 `cargo build`。

> [!TIP]
> 本仓库 `examples/` 是同一套结构的真实工程。想练手就在它上面试：在 `examples/` 目录里敲 `cargo tree -e features`、`cargo build -p task-queue -vv`，看真实的 feature 树和 rustc 参数。

## 报错现场

### 现场一：`--features` 指向被 `dep:` 抑制的名字

`imgfmt` 里 `gif` 是可选依赖，但 `[features]` 用 `avif = ["dep:gif"]` 引用了它，于是没有名为 `gif` 的 feature。这时按依赖名去开它：

```console
$ cargo run -q -p imgfmt --features gif
error: package `imgfmt v0.1.0 (…\ws\imgfmt)` does not have feature `gif`

help: an optional dependency with that name exists, but the `features` table includes it with the "dep:" syntax so it does not have an implicit feature with that name
Dependency `gif` would be enabled by these features:
	- `avif`
```

**定位路径**：报错说得很细——存在同名可选依赖，但 `features` 表里用 `dep:` 引用了它，所以没有同名隐式 feature，最后还列出「哪些 feature 会打开它」。修法按意图选：保留「不暴露依赖名」的设计就让用户写 `--features avif`；想暴露就补一条 `gif = ["dep:gif"]`（此时 `avif` 也该写成 `avif = ["gif"]`，别让同一份清单里混用两种引用方式）。

（如果 `[features]` 里出现同名条目，比如 `webp = []`，而可选依赖也叫 `webp`：Cargo 直接拒绝加载清单，报 ``optional dependency `webp` is not included in any feature``——同名条目**取代**了隐式 feature，你得自己写 `webp = ["dep:webp"]`。）

### 现场二：成员想覆写继承来的 lint

工作区设了 `[workspace.lints.rust] unsafe_code = "forbid"`，成员里只想放宽这一条：

```toml
[lints]
workspace = true

[lints.rust]
unsafe_code = "allow"
```

```text
error: failed to load manifest for workspace member `…\lints\c`
referenced by workspace at `…\lints\Cargo.toml`

Caused by:
  failed to parse manifest at `…\lints\c\Cargo.toml`

Caused by:
  cannot override `workspace.lints` in `lints`, either remove the overrides or `lints.workspace = true` and manually specify the lints
```

**定位路径**：报错发生在**解析清单**阶段——代码一行都没编，说明这是清单结构问题。Cargo 给的两条路：删掉覆写（用工作区的值），或放弃继承、手写整份 lint 表。实测连「新增一条工作区里没有的 lint」也会触发同一条错误。

### 现场三：build script 指令名打错

build script 的协议没有类型检查，但 Cargo 会核对指令名。把 `rustc-env` 打成 `bogus-directive`：

```rust,ignore
// 探针里的 build.rs（本节实测用的就是这个）
fn main() {
    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rustc-check-cfg=cfg(legacy_style)");
    println!("cargo::rustc-cfg=legacy_style");
    println!("cargo::rustc-env=LEGACY_STYLE=1");
    println!("cargo::bogus-directive=1");
}
```

```text
error: invalid output in build script of `oldbuild v0.1.0 (…\oldbuild)`: `cargo::bogus-directive=1`
Unknown key: `bogus-directive`.
See https://doc.rust-lang.org/cargo/reference/build-scripts.html#outputs-of-the-build-script for more information about build script outputs.
```

**定位路径**：报错把「哪一步、哪一行输出、哪个键」全给了——`invalid output in build script` + 原文那一行 + `Unknown key`。修法就是拼对指令名。注意这里用的是双冒号前缀：同一个脚本把 `cargo::` 换成 `cargo:` 后，未知键会被当成 metadata 默默收下（旧前缀的兼容策略），实测不报错——这是本章唯一一个「写错也不一定告诉你」的地方，也是建议统一用双冒号的理由。

## 小结

- **features 必须可叠加**：同一个依赖的 feature 是所有使用者的并集，所以不能用它表达互斥选择，功能要用正向名字（`std` 而不是 `no_std`）。可选依赖自带同名隐式 feature，`dep:` 引用一次就把它收起来。
- **工作区 = 一份锁 + 一份 target + 一套继承表**：`workspace.package`/`dependencies`/`lints` 让成员只写差异；成员对继承来的依赖只能加 `optional`/`features`（`default-features` 得回根表改）。虚拟清单要显式 `resolver`（漏写只警告并退回 `"1"`）。
- **`[lints]` 按工具分表、可以继承**：`workspace = true` 是「全盘继承」，与任何本地 lint 表互斥；`priority` 决定传给 rustc 的顺序，顺序决定谁赢（实测 `--deny=warnings --allow=unused`）。
- **`rust-version` 是门禁也是解析输入**：门槛不满足直接报错（可 `--ignore-rust-version`），与 edition 下界冲突会在解析清单时失败；resolver `"3"` 把 `incompatible-rust-versions` 默认设为 `fallback`，优先挑 MSRV 友好的版本（实测 `itoa 1.0.15` 对上 `1.0.18`）。
- **build script 靠 stdout 指令通信**：`cargo::`（1.77+）为主，`cargo:` 仍可用；`rerun-if-changed` 决定重跑条件，自定义 cfg 必须配 `rustc-check-cfg`，指令名拼错是硬错误。判断目标平台要读 `CARGO_CFG_*`，别用 `cfg!`。
- **CI 把约束变成门禁**：`fmt --check` + `clippy` + `test --locked` + MSRV 一条线；`build.warnings = "deny"` 让本包警告直接失败。发布不可逆，yank 只影响新解析、不删代码。

## 练习

**一、解释。** 有人给库设计了两个 feature：`openssl` 和 `rustls`，并在代码里用 `compile_error!` 拦住「两个同时打开」。为什么 Cargo 手册把这种做法归为「应尽量避免」？如果确实要支持两种后端，更稳妥的做法是什么？

**二、找错。** 一个虚拟工作区里，成员 `svc` 的清单片段如下。它想做的是「继承工作区的版本号与 lint，并把日志依赖固定成不带默认 features」：

```toml
[package]
name = "svc"
version.workspace = true
rust-version.workspace = true

[dependencies]
tracing = { workspace = true, default-features = false }

[lints]
workspace = true

[lints.rust]
unsafe_code = "allow"
```

这段清单有两个问题，分别在哪、怎么改？

**三、预测。** 工作区里有 `a`、`b` 两个成员，`a` 依赖 `imgfmt`（默认 features + `png`），`b` 依赖 `imgfmt`（`features = ["avif"]`）。回答两问：（1）`cargo build -p a` 与 `cargo build --workspace` 分别会让 `imgfmt` 带上哪些 feature？（2）用哪条命令能直接看到「哪个包开了哪个 feature」？

<details><summary>参考答案</summary>

**一、** 因为 feature 是**并集**语义：`--all-features`、或依赖图上任意一个包同时要求两者，都会让两个 feature 同时为真，`compile_error!` 那时只能把构建打断——这不是「检测到误用」，而是「这个设计在别人的构建图里站不住」。手册给的三条出路：拆成两个包（各自成为 crate，由用户选依赖）；用运行时配置选后端（编译两份，配置决定用哪份）；确有优先关系时用一个 feature 覆盖另一个。`compile_error!` 只能兜底，而且会卡住下游整条构建链。

**二、** 两个问题都在「成员覆写继承」上：

- `tracing = { workspace = true, default-features = false }`：继承来的依赖只能加 `optional` 与 `features`，`default-features` 必须在 `[workspace.dependencies]` 里声明（报错是 `default-features = false` cannot override workspace's `default-features`）。改法：把 `default-features = false` 移到根清单的 `[workspace.dependencies] tracing = { …, default-features = false }`，成员侧只留 `tracing.workspace = true` 或再加 `features = [...]`。
- `[lints] workspace = true` 和 `[lints.rust] unsafe_code = "allow"` 并存：实测会报 `` cannot override `workspace.lints` in `lints` ``，成员侧不能既继承又覆写。改法二选一——要么删掉 `[lints.rust]`（接受工作区的 `forbid`），要么去掉 `workspace = true`、在成员里手写完整的 lint 表。

另外顺带一提：`version.workspace = true` 与 `rust-version.workspace = true` 是合法写法，工作区根必须真有这两个键。

**三、** （1）构建是**按调用**确定 feature 并集的：`cargo build -p a` 只把 `a` 的子图算进来，`imgfmt` 得到 `default`（`bmp`）+ `png`；`cargo build --workspace` 把 `b` 也算进来，于是并集变成 `bmp`、`png`、`avif`。（2）`cargo tree -e features -i imgfmt` 看 feature 从哪些包流进来；`cargo tree -f "{p} {f}"` 看每个包当前打开的 feature 列表；怀疑重复编译时用 `cargo tree --duplicates`。

</details>

## 延伸阅读

- features 的全部规则（隐式 feature、`dep:`、`?`、统一、加法性）：[Features](https://doc.rust-lang.org/cargo/reference/features.html)
- 工作区：成员、`workspace.package`/`dependencies`/`lints`、共享锁与 target：[Workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html)
- manifest 的 `[lints]` 表（`level`/`priority` 与工具分表）：[The manifest format](https://doc.rust-lang.org/cargo/reference/manifest.html)
- build script 的全部指令与环境变量：[Build Scripts](https://doc.rust-lang.org/cargo/reference/build-scripts.html)、[Build Script Examples](https://doc.rust-lang.org/cargo/reference/build-script-examples.html)
- MSRV：`rust-version` 的语义、支持策略与多策略工作区：[Rust Version](https://doc.rust-lang.org/cargo/reference/rust-version.html)
- resolver 版本与 `incompatible-rust-versions`：[Dependency Resolution](https://doc.rust-lang.org/cargo/reference/resolver.html)
- CI 模板：最新依赖、MSRV、警告门禁三条线：[Continuous Integration](https://doc.rust-lang.org/cargo/guide/continuous-integration.html)
- 发布与 yank：[Publishing on crates.io](https://doc.rust-lang.org/cargo/reference/publishing.html)
