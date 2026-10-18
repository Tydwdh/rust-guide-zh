# 项目 5：工程化改造

前四个项目解决的是「代码怎么写」。这一个解决的是「代码写完然后呢」：怎么把一份能跑的代码变成一份能交出去的发布件 —— 元数据、README、变更日志、文档站点、打包检查、持续集成（CI）门禁。

样板是仓库里的 `examples/release-drill/`：一个零依赖的小库，把字节数变成人能读的大小字符串，再读回来。业务逻辑五分钟写完，剩下的是「发布」这件事本身。

这一章还有第二个对象：本仓库真实的 `.github/workflows/book.yml`。它是给这本书用的工作流，我们逐段读它，然后**真的**给它加上几条门禁，每条命令都在本机跑过。仓库目前没有公开远端，所以「工作流真的在 GitHub 上跑起来」这件事要等仓库公开之后。

## 这一章解决什么问题

### 要做的东西

`release-drill` 的公开接口三项，都在 `examples/release-drill/src/lib.rs` 里：

| 接口 | 作用 |
|---|---|
| `format(bytes: u64) -> String` | `1536` → `1.5 KiB` |
| `parse(text: &str) -> Result<u64, ParseError>` | `1.5 KiB` → `1536` |
| `ParseError` | 四类输入错误，实现 `Display` 与 `std::error::Error` |

要补齐的发布件有五类：

1. **crate 本体**：公开项都带文档注释与示例，示例走文档测试（doctest，第 31 章）；
2. **清单元数据**：`description`、`license`、`readme`、`keywords`、`categories`、`rust-version`；
3. **给人看的文件**：`README.md` 与 `CHANGELOG.md`（Keep a Changelog 格式）；
4. **docs.rs 元数据**：`[package.metadata.docs.rs]`；
5. **检查链**：`cargo package --list` 与 `cargo package` 真跑，CI 里新增四个门禁作业。

先把三个词摆正（术语表）：crate 是编译单元，包是一个或多个 crate 的发布单位，本项目一个包一个 crate；`examples/` 是一份虚拟清单工作区，`release-drill` 是它的成员之一（第 30 章）。交出去的是**包**。

整条路是这样的：

<svg viewBox="0 0 640 252" width="100%" role="img" aria-label="发布路径：源码树经本地四道门禁与打包验证生成 .crate 包，再由 cargo publish 上传到 crates.io 并生成 docs.rs 文档" xmlns="http://www.w3.org/2000/svg">
<text x="0" y="16" font-size="14" fill="currentColor">从源码到发布：门禁全部落在打包之前</text>
<g fill="none" stroke="currentColor" stroke-width="1.5">
<rect x="8" y="38" width="140" height="104" rx="6" />
<rect x="164" y="38" width="140" height="104" rx="6" />
<rect x="320" y="38" width="140" height="104" rx="6" />
<rect x="476" y="38" width="140" height="104" rx="6" />
</g>
<g font-size="13" fill="currentColor" text-anchor="middle">
<text x="78" y="58">源码树</text>
<text x="234" y="58">门禁</text>
<text x="390" y="58">打包验证</text>
<text x="546" y="58">发布项</text>
</g>
<g font-size="11" fill="currentColor" text-anchor="middle" font-family="monospace">
<text x="78" y="78">Cargo.toml</text>
<text x="78" y="94">README.md</text>
<text x="78" y="110">CHANGELOG.md</text>
<text x="78" y="126">src/lib.rs</text>
<text x="234" y="78">cargo fmt --check</text>
<text x="234" y="94">cargo clippy -D</text>
<text x="234" y="110">cargo test</text>
<text x="234" y="126">cargo doc -D</text>
<text x="390" y="78">cargo package</text>
<text x="390" y="94">解开 .crate</text>
<text x="390" y="110">重新编译</text>
<text x="390" y="126">7 个文件</text>
<text x="546" y="78">crates.io</text>
<text x="546" y="94">docs.rs</text>
<text x="546" y="110">（上传之后）</text>
</g>
<g stroke="var(--links)" stroke-width="2" fill="none">
<line x1="148" y1="86" x2="156" y2="86" />
<line x1="304" y1="86" x2="312" y2="86" />
<line x1="460" y1="86" x2="468" y2="86" />
<line x1="546" y1="142" x2="546" y2="164" />
</g>
<g fill="var(--links)">
<polygon points="164,86 154,81 154,91" />
<polygon points="320,86 310,81 310,91" />
<polygon points="476,86 466,81 466,91" />
<polygon points="546,164 541,154 551,154" />
</g>
<text x="238" y="156" font-size="11" fill="var(--links)" text-anchor="middle">本地与 CI 同一套命令</text>
<g fill="none" stroke="var(--warning-border)" stroke-width="2" stroke-dasharray="5 3">
<rect x="380" y="164" width="240" height="46" rx="6" />
</g>
<text x="500" y="184" font-size="12" fill="currentColor" text-anchor="middle">cargo publish：本书不执行</text>
<text x="500" y="201" font-size="11" fill="currentColor" text-anchor="middle">没有 crates.io 凭据，发布不可逆</text>
<text x="0" y="226" font-size="12" fill="currentColor">本地先跑一遍，CI 在干净机器上再跑一遍；打包之后的 .crate 解开会重新编译。</text>
<text x="0" y="242" font-size="12" fill="currentColor">重新编译那一步专门抓「漏文件」：本地磁盘上有的文件，包里不一定有。</text>
</svg>

### 验收标准

做完要满足四条，缺一条都是没做完：

1. `cargo test --manifest-path examples/Cargo.toml -p release-drill` 全绿：11 个单元测试加 3 个文档测试；
2. `cargo package` 通过 —— 它会把打包产物解开、在干净目录里重新编译一遍；
3. `mdbook test -c project5-engineering.md` exit 0（本章的代码块）；
4. CI 新增的四条门禁在本机逐条跑过，命令与输出写进下面各节。

版本口径：本机 rustc 1.98.0、cargo 1.98.0、edition 2024。下面的命令都可原样复制；cargo 的中间产物落在工作区自己的 `target/` 里，`.gitignore` 已经忽略它。

最终效果先看一遍。下面这些终端记录都真跑过，只省掉了 cargo 每次都打的重复状态行：

```console
$ cargo test --manifest-path examples/Cargo.toml -p release-drill
   Compiling release-drill v0.1.0 (D:\Rust\examples\release-drill)
running 11 tests
…
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests release_drill

running 3 tests
…
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
$ cargo package --manifest-path examples/Cargo.toml -p release-drill --allow-dirty
warning: manifest has no documentation, homepage or repository
   Packaging release-drill v0.1.0 (D:\Rust\examples\release-drill)
    Packaged 7 files, 14.4KiB (5.9KiB compressed)
   Verifying release-drill v0.1.0 (D:\Rust\examples\release-drill)
   Compiling release-drill v0.1.0 (…\target\package\release-drill-0.1.0)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.16s
```

两条不太好看的地方后面都要交代：`--allow-dirty` 的原因在「报错现场」第三条，那条 warning 的原因在「设计取舍」第四条。

## 设计取舍

**1. 接口给两个函数，还是给一个类型？** 三条路：两个自由函数；`ByteSize(u64)` 新类型加 `Display` 与 `FromStr`；返回一个实现 `Display` 的包装类型。选第一条：调用点几乎都是「立刻拼进日志」，返回 `String` 最省事；新类型要起名字、要声明关联错误类型，对只有两个动作的小库是纯负担。包装类型能少一次分配，但那是第 29 章的话题，等真有热路径调用点再改；`#[must_use]` 先管住「返回值别丢」。代价是 `parse` 和 `str::parse` 撞语境，所以错误类型叫 `ParseError`。

**2. 单位：输出一套，输入两套。** 1024 进制的 IEC 后缀（`KiB`）与 1000 进制的 SI 后缀（`kB`）差 2.4%（到 `MiB` 一档就是 4.8%），这是两套约定，不是两种写法。输出的日志只该有一种读法，所以 `format` 固定输出 IEC：程序员说「1 MB」时心里想的是 1048576 字节，而且 `KiB` 这个后缀不会让读日志的人再猜一遍。输入的来源五花八门，所以 `parse` 两套都收 —— 读入宽容、写出单一。**但不做换算**：`1.5MB` 就按 1000 进制算成 1500000，不会「猜」你想说的是 `1.5 MiB`；这个差异在 `parse` 的文档注释与 README 里各写了一遍。

**3. 零依赖是承诺，不是现状。** `[dependencies]` 这张空表是显式写出来的：每多一个依赖，发布件就多一份要跟着升级、可能带漏洞、可能与下游打架的东西（第 30 章 30.1 那类 feature 冲突，源头常常是依赖）。这个库的全部需求 —— 数字格式化、单位查表、错误类型、`Display` —— 标准库都给了。代价是浮点行为要自己讲清楚：中间值用 `f64`，`u64::MAX` 会显示成 `16 EiB`（写在 `format` 的文档注释里）。

**4. 元数据：官方要求什么就写什么，不编没有的东西。** Cargo Book 的 manifest 页把两条写成 note：`description` 必须设（「crates.io requires the description to be set.」），`license` 与 `license-file` 至少给一个（「crates.io requires either license or license-file to be set.」）。发布页另给一份更全的平铺清单（`license`、`description`、`homepage`、`repository`、`readme`），`keywords` 与 `categories` 不是必需但「It would also be a good idea to include some keywords and categories」。本项目把能写的都写上，**`repository` 与 `homepage` 空着** —— 仓库没有公开远端，填一个 URL 等于写不实信息，代价就是 `cargo package` 那条 warning。`rust-version = "1.85"` 取 edition 2024 的下界（第 30 章 30.4）；本机只有 1.98，「真能在 1.85 上构建」要靠 CI 装那条工具链验证，这一条留给读者。

**5. docs.rs 元数据只写有实际效果的那项。** 官方 Metadata 页列了一串键：`all-features`、`features`、`no-default-features` 管 feature 组合，本 crate 没有 feature；`rustdoc-args` 是给 rustdoc 追加参数的地方，我们也没有需要 `docsrs` cfg 的地方（第 32 章 32.5）；`targets = []` 的含义是「只构建默认目标」，而这个纯计算、无平台分支的 crate 不需要五份文档。所以只留最后一项。

**6. 门禁写在清单里还是 CI 里？** 分工不同。清单的 `[lints]` 表（第 30 章 30.3）本地就生效：`missing_docs = "deny"` 让 `cargo build` 失败，`[lints.rustdoc] broken_intra_doc_links = "deny"` 让 `cargo doc` 失败 —— 这是「这个包永远不许松」。CI 再补 `-D warnings` 这类严格档：clippy 与 rustdoc 的**全部**警告都当错误，跑在干净机器上。清单管下限，CI 管上限。

## 逐块实现

### 1. crate 本体：`src/lib.rs`

先是两张数据表：输出的单位表，与输入认识的后缀表。

```rust
{{#include ../examples/release-drill/src/lib.rs:21:42}}
```

`IEC_UNITS` 用数组是因为它只按顺序遍历；`SUFFIXES` 用切片里的元组是因为查表要按名字找，而表本身是静态的（第 6 章）。注意键全部小写：查之前先把输入 `to_ascii_lowercase()`，这样 `MiB`、`mib`、`MIB` 都认。

输出的核心是一个「除到合适档位」的循环：

```rust,ignore
{{#include ../examples/release-drill/src/lib.rs:62:77}}
```

`while` 的条件带 `index + 1 < IEC_UNITS.len()`：`u64` 的上限是 16 EiB，除到 `EiB` 就该停，不然会越界。舍入规则是「尾数小于 10 保留一位小数，否则取整」—— 这一条让 `1.0 KiB` 与 `10 KiB` 同时好看。`#[must_use]` 是给发布者的便宜保险：调用方忘了用返回值，编译器会警告。

输入方向长一些，因为「宽容」是要付出代价的：

```rust,ignore
{{#include ../examples/release-drill/src/lib.rs:99:136}}
```

拆输入用了 `find` 与 `split_at`（第 4、6 章）：找到「第一个既不是数字也不是小数点的字符」，从那里切成数字段与单位段。数字段交给 `.parse::<f64>()`（第 7 章），失败报 `InvalidNumber`；单位段查表（`iter().find` 加闭包，第 10 章），查不到报 `UnknownUnit`；乘积越过 `u64` 上限报 `TooLarge` —— 四个错误各有名字，调用方能分开处理。

错误类型本身只有十几行，剩下的都是给标准库补的三块拼图：

```rust
# use std::fmt;
{{#include ../examples/release-drill/src/lib.rs:138:176}}
```

三处值得停一下。`#[non_exhaustive]`：以后加第五个变体不算破坏性变更（SemVer 里 `0.x` 的 minor 可以带破坏性变更，但能少一次就少一次）；`Display` 把每个变体写成一句人话，测试里直接断言这句话；`impl std::error::Error for ParseError {}` 是空的 —— `source()` 没有底层错误可挂，空实现正是「这个错误就是根因」的表达。

**这块用到第 4、6 章的切片与字符串、第 7 章的 `.parse()` 与自定义错误（`impl std::error::Error`、`?`）、第 9 章的 `Display` 实现、第 10 章的闭包与迭代器。**

> [!NOTE]
> 上面两个 `ignore` 块是跨块互相引用的片段（`format` 用第一块的 `IEC_UNITS`，`parse` 还用到第三块的 `ParseError`）。`mdbook test` 会把每个块当独立程序编译，自然编不过，所以标了 `ignore`；片段与文件逐字一致，整份代码由 `cargo test` 真跑验证。

### 2. 发布件：README 与 CHANGELOG

README 是绝大多数人会看到的第一份文档，也是 crates.io 页面上直接渲染的那一份。它要回答三个问题：这是什么、怎么用、单位怎么算。

```markdown
{{#include ../examples/release-drill/README.md:13:20}}
```

README 里的例子不参与测试 —— 这是取舍的另一面。第 32 章 32.1 实测过另一条路：用 `include_str!` 把 README 接进 crate 文档，README 里的代码块就变成文档测试，改错会被 `cargo test` 抓住。本项目没走那条路，因为那会把同一段例子在 crate 文档里再展示一遍；取而代之的是一条纪律：README 的例子与 `lib.rs` 里的文档测试保持同一组断言，改一处要改两处，而这两处都在 `examples/release-drill/` 里，评审时一眼能看见。

`CHANGELOG.md` 用 Keep a Changelog 的骨架，起点就是这次要发的版本：

```markdown
{{#include ../examples/release-drill/CHANGELOG.md:10:21}}
```

它有两个作用：发布时给用户一份「这版变了什么」，以及逼作者在动代码时就想清楚这次改动属于哪一类。Cargo Book 的发布页把这条写进了建议：「A changelog entry, preferably manually curated though a generated one is better than nothing」。文件顶部还留着 `Unreleased` 一节：下一个版本先往那里写，发布时改名成版本号加日期。

### 3. 清单元数据

清单分三块：给人看的元数据、零依赖的声明、门禁。

```toml
{{#include ../examples/release-drill/Cargo.toml:1:16}}
```

`keywords` 是搜索用的（最多 5 个、每个不超 20 字符的 ASCII），`categories` 必须从 crates.io 的固定表里挑 —— 本 crate 用的是 `value-formatting`。`readme = "README.md"` 这一项不只是给打包清单看的，crates.io 页面会直接渲染这个文件。

第二块是门禁与 docs.rs 元数据：

```toml
{{#include ../examples/release-drill/Cargo.toml:18:40}}
```

`unsafe_code = "forbid"` 与 `dbg_macro = "deny"` 这类条目的取舍原则在第 32 章；这里只说它们对本项目的意义：这个库一行 `unsafe` 都没有，「以后也不许悄悄加」写成清单里的 `forbid`，评审时改清单比抓代码容易。`[package.metadata.docs.rs]` 只写 `targets = []` 的理由见「设计取舍」第 5 条。

### 4. 打包检查：`cargo package`

发布前先看清包里到底装了什么 —— `cargo package --list` 就是干这个的：

```console
$ cargo package --manifest-path examples/Cargo.toml -p release-drill --list --allow-dirty
.cargo_vcs_info.json
CHANGELOG.md
Cargo.lock
Cargo.toml
Cargo.toml.orig
README.md
src/lib.rs
```

七个文件分成两类 —— 你写的三个，Cargo 生成的四个：

- `Cargo.toml`：归一化过的清单（下一段展开）；
- `Cargo.toml.orig`：你写的那份原文的副本；
- `Cargo.lock`：解开 .crate 后按包重新生成的锁（这个库没有依赖，里面只有它自己）；
- `.cargo_vcs_info.json`：打这个包时所在提交的 sha 与「工作树是否脏」。

<svg viewBox="0 0 640 214" width="100%" role="img" aria-label=".crate 包的构成：三个由作者写的文件，四个由 cargo 生成的文件，共七个" xmlns="http://www.w3.org/2000/svg">
<text x="0" y="16" font-size="14" fill="currentColor">一个 .crate（tgz）里有七个文件，四分之三不是手写的</text>
<rect x="8" y="30" width="624" height="140" rx="6" fill="none" stroke="currentColor" stroke-width="1.5" />
<text x="24" y="50" font-size="12" fill="currentColor" font-family="monospace">release-drill-0.1.0.crate</text>
<rect x="24" y="62" width="276" height="92" rx="6" fill="none" stroke="var(--links)" stroke-width="1.5" />
<rect x="316" y="62" width="300" height="92" rx="6" fill="none" stroke="var(--quote-border)" stroke-width="1.5" />
<text x="162" y="82" font-size="12" fill="var(--links)" text-anchor="middle">你写的（3）</text>
<text x="466" y="82" font-size="12" fill="currentColor" text-anchor="middle">cargo 生成的（4）</text>
<g font-size="11" fill="currentColor" font-family="monospace" text-anchor="middle">
<text x="162" y="102">README.md / CHANGELOG.md</text>
<text x="162" y="120">src/lib.rs</text>
<text x="466" y="102">Cargo.toml.orig / Cargo.lock</text>
<text x="466" y="120">Cargo.toml（归一化）</text>
<text x="466" y="138">.cargo_vcs_info.json</text>
</g>
<text x="0" y="192" font-size="12" fill="currentColor">清单只有两个来源，一眼能数清：漏文件看左边，看不懂的东西看右边。</text>
<text x="0" y="210" font-size="12" fill="currentColor">crates.io 对单个 .crate 有 10MB 上限，所以「数文件」这一步不是形式主义。</text>
</svg>

归一化后的清单值得看一眼，它和 `Cargo.toml.orig` 的差别就是 Cargo 替你做掉的事：

```toml
[package]
edition = "2024"
rust-version = "1.85"
name = "release-drill"
version = "0.1.0"
build = false
autolib = false
autobins = false
autoexamples = false
autotests = false
autobenches = false
…
[lib]
name = "release_drill"
path = "src/lib.rs"
```

`build = false` 与一串 `auto* = false` 是「这个包没有构建脚本、没有自动发现的目标」，明确写死比留给下游的 Cargo 去猜可靠；`[lib]` 段把 crate 名（带下划线）与路径写全。清单里的 `[lints]` 表被原样保留 —— 但按第 30 章的结论，它只对**本包自己的**构建生效，不会传给下游。

然后是真正会拦人的那一步：

```console
$ cargo package --manifest-path examples/Cargo.toml -p release-drill --allow-dirty
   Packaging release-drill v0.1.0 (D:\Rust\examples\release-drill)
    Packaged 7 files, 14.4KiB (5.9KiB compressed)
   Verifying release-drill v0.1.0 (D:\Rust\examples\release-drill)
   Compiling release-drill v0.1.0 (…\target\package\release-drill-0.1.0)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.16s
```

输出对应三件事：打包（`Packaging` 与 `Packaged` 两行）→ 解开到 `target/package/` 下 → 在那里**重新编译一遍**（`Verifying` 与 `Compiling` 两行）。第三步是整条链上最值钱的一步：如果 `src/` 里少了某个被 `include_str!` 或 `mod` 引用的文件，本地 `cargo build` 看不出问题（文件就在磁盘上），但解开后的目录里没有，编译立刻失败。`cargo publish --dry-run` 做的是同一件事（官方文档说 `cargo package` 等价于它），差别只是多一步「假装上传」：

```console
$ cargo publish --manifest-path examples/Cargo.toml -p release-drill --dry-run --allow-dirty
    Updating crates.io index
   Packaging release-drill v0.1.0 (D:\Rust\examples\release-drill)
    Packaged 7 files, 14.4KiB (5.9KiB compressed)
   Verifying release-drill v0.1.0 (D:\Rust\examples\release-drill)
   Compiling release-drill v0.1.0 (…\target\package\release-drill-0.1.0)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.95s
   Uploading release-drill v0.1.0 (D:\Rust\examples\release-drill)
warning: aborting upload due to dry run
```

最后那行 `aborting upload due to dry run` 是这条命令的全部价值：本机既没有 crates.io 账号也没有 token（`~/.cargo/credentials.toml` 不存在），它照样把前三步跑完，然后在真正上传前停下。本书到此为止，`cargo publish` 不执行 —— 发布是不可逆的（后面「发布清单」会引官方原话）。

### 5. CI 加固：读一遍 `book.yml`，再给它加门禁

工作流文件在 `.github/workflows/book.yml`。它本来就是给这本书用的，先看既有部分：

```yaml
{{#include ../.github/workflows/book.yml:1:44}}
```

逐段读（`check` 作业里 `actions/checkout` 之外的四步）：

- **触发条件**：`push` 到 `main` 与任何 `pull_request`。
- **第一步**装 mdBook 与链接检查器，两个 `cargo install` 都钉了版本 —— 工具链漂移会把 CI 弄红（同第 30 章「CI 上固定工具链」）。
- **第二步是这本书特有的坑**：`mdbook build` 遇到非法 `[output.html]` 字段、缺失的 include 指令都只打 `ERROR` 然后 exit 0，所以必须抓住日志再搜一遍 `ERROR`（本仓实测）。
- **第三步** `mdbook test` 跑章内代码块，失败 exit 101，是可靠的退出码门禁。
- **第四步**跑示例工作区测试；`examples/` 万一还没就绪就跳过 —— 布局过渡期的保护。

现在加门禁。新增的四个作业：

```yaml
{{#include ../.github/workflows/book.yml:40:77}}
```

每个作业背后都是一条能在本机跑的 cargo 命令，逐条真跑：

```console
$ cargo fmt --manifest-path examples/Cargo.toml -p release-drill --check
$ cargo clippy --manifest-path examples/Cargo.toml -p release-drill --all-targets -- -D warnings
    Checking release-drill v0.1.0 (D:\Rust\examples\release-drill)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.12s
$ RUSTDOCFLAGS="-D warnings" cargo doc --manifest-path examples/Cargo.toml -p release-drill --no-deps
 Documenting release-drill v0.1.0 (D:\Rust\examples\release-drill)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.30s
$ cargo package --manifest-path examples/Cargo.toml -p release-drill --allow-dirty
    Packaged 7 files, 14.4KiB (5.9KiB compressed)
   Verifying release-drill v0.1.0 (D:\Rust\examples\release-drill)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.16s
```

`fmt` 那一条没有任何输出、退出码是 0 —— `--check` 只在有差异时才说话。`clippy` 的 `--all-targets` 会把测试与示例目标一起查（第 32 章 32.6 讲过分组与取舍，这里用的是「全部警告当错误」这条最严的档）。`doc` 那条的严格档放在环境变量里：`RUSTDOCFLAGS="-D warnings"`，这样 rustdoc 的任何警告都变成错误，包括链接写错、可疑的 HTML —— 本章「报错现场」的第一条就是被它拦下来的。`package` 在 CI 里不需要 `--allow-dirty`：`actions/checkout` 拿到的是一棵干净的工作树；本地才需要（第三条报错）。

三件必须说清楚的事：

1. **为什么只覆盖 `release-drill`。** `cargo fmt --all --check` 现在会红：本机实测它在 `macro-demo`、`task-queue` 等示例 crate 里各报出一串格式差异，而那些文件不归这次改动管 —— 门禁先落地在能保证绿的范围内（一次只改一处），格式统一留给专门的一次改动。
2. **CI 上的 rustfmt 与 clippy 从哪来。** GitHub 托管的 Ubuntu runner 预装 rustup 与 stable 工具链，`cargo fmt` 与 `cargo clippy` 直接可用；不在 CI 里额外 `cargo install` 这两个工具，避免版本漂移。
3. **这份工作流还没被真实触发过。** 仓库没有公开远端（清单元数据里的 `repository` 也空着）；本地能验证的是「命令跑得过」，验证不了 runner 环境、缓存这些只属于 GitHub 的东西。

## 报错现场

### 报错一：文档里的链接指到了宏（`format` 既是函数又是宏）

crate 级文档里第一版是这么写的（片段，不是能独立编译的程序 —— 这条错误由 rustdoc 报出，不是 rustc，所以标 `ignore`）：

```rust,ignore
//! - [`format`]：`1536` → `"1.5 KiB"`；

pub fn format(bytes: u64) -> String {
    bytes.to_string()
}
```

`cargo doc` 的报错（Rust 1.98；这条不需要 `RUSTDOCFLAGS`，清单里的 `[lints.rustdoc]` 就把它拦下了）：

```text
error: `format` is both a function and a macro
 --> release-drill\src\lib.rs:5:9
  |
5 | //! - [`format`]：`1536` → `"1.5 KiB"`；
  |         ^^^^^^ ambiguous link
  |
  = note: requested on the command line with `-D rustdoc::broken-intra-doc-links`
help: to link to the function, add parentheses
  |
5 | //! - [`format()`]：`1536` → `"1.5 KiB"`；
  |               ++
help: to link to the macro, add an exclamation mark
  |
5 | //! - [`format!`]：`1536` → `"1.5 KiB"`；
  |               +
```

定位路径：`format` 在同一个作用域里有两个候选 —— 我们刚定义的同名函数，和标准库预导入的 `format!` 宏。rustdoc 不猜：两个都指得通就报歧义（术语表里 intra-doc link 那条说的相对链接解析）。修法照它给的两条：链到函数补一对圆括号，链到宏补一个叹号 —— 上面 `help` 段的替换文本就是最终写法。本项目把两处都改成了带圆括号的那种。

这条报错的价值在于它证明了门禁真的在工作：`broken_intra_doc_links` 写进了清单的 `[lints.rustdoc]`，`cargo doc` 直接失败；而 rustdoc 把「这条 note」也打出来了 —— `requested on the command line with -D rustdoc::broken-intra-doc-links`，说明它是被显式 deny 的，不是碰巧。

### 报错二：公开函数忘了写文档（`missing_docs`）

往库里加一个没有文档注释的公开函数：

```rust
pub fn undocumented() {}
```

`cargo build` 立刻停下：

```text
error: missing documentation for a function
   --> src\lib.rs:289:1
    |
289 | pub fn undocumented() {}
    | ^^^^^^^^^^^^^^^^^^^^^
    |
    = note: requested on the command line with `-D missing-docs`

error: could not compile `release-drill` (lib) due to 1 previous error
```

这条 error 是清单里那行 `missing_docs = "deny"` 带来的（末句 note 就是证据）。同一份文件里再塞一条坏链接、跑 CI 的文档门禁，两个问题会一起报出来：

```text
error: missing documentation for a function
   --> src\lib.rs:289:1
…
error: unresolved link to `missing_item`
   --> src\lib.rs:291:10
    |
291 | /// 参见 [`missing_item`]。
    |            ^^^^^^^^^^^^ no item named `missing_item` in scope
    |
    = help: to escape `[` and `]` characters, add '\' before them like `\[` or `\]`
    = note: requested on the command line with `-D rustdoc::broken-intra-doc-links`
```

修法：写文档注释。这个库没有一处需要 `#[allow(missing_docs)]` 的例外 —— 真需要开例外时，把理由写在属性旁边，否则下一次评审没人知道那条 allow 还在不在。

### 报错三：`cargo package` 拒绝在脏工作树上打包

第一次跑 `cargo package --list`（不加任何 flag）：

```text
warning: manifest has no documentation, homepage or repository
  |
  = note: see https://doc.rust-lang.org/cargo/reference/manifest.html#package-metadata for more info
error: 4 files in the working directory contain changes that were not yet committed into git:

examples\release-drill\CHANGELOG.md
examples\release-drill\Cargo.toml
examples\release-drill\README.md
examples\release-drill\src\lib.rs

to proceed despite this and include the uncommitted changes, pass the `--allow-dirty` flag
```

两个信息叠在一起：上面的 warning 是「本仓没有公开远端、`repository` 与 `homepage` 空着」的后果；下面的 error 是打包特有的检查 —— 包里会带 `.cargo_vcs_info.json`（记着这次打包对应哪个提交、工作树脏不脏），所以 Cargo 不让你在没提交时就造包，免得发布件与仓库状态对不上。

修法三选一：先提交（正式发布该这样，发布后还要打 tag）；本地临时验证加 `--allow-dirty`；CI 里什么都不用加 —— `actions/checkout` 出来的树是干净的。本机这次是给书写样例，用第二种，`dirty` 的真实情况留在 `.crate` 的 `.cargo_vcs_info.json` 里：

```json
{
  "git": {
    "sha1": "1cbcac46c51cd807a5144d8e2ca3b49040041af7",
    "dirty": true
  },
  "path_in_vcs": "examples/release-drill"
}
```

## 测试

测试分三层，各管一段。

**第一层：单元测试（11 个）**，都在 `src/lib.rs` 的 `#[cfg(test)] mod tests` 里（第 11 章）：

- **正常路径**：数字与单位的各种写法（`1.5 KiB`、`1.5KiB`、`  2.5  kb `）、没有后缀的裸数字、两套单位各自的换算；
- **边界**：1023 字节不升档、1024 字节升到 `1.0 KiB`、档位一直走到 `EiB`、`u64::MAX` 显示成 `16 EiB`、`1.1 KiB` 舍入到 1126 字节；
- **错误路径**：空输入、非数字、负号、坏单位、超过 `u64` 上限、几百位数字的输入（在 `f64` 里溢出成 `inf`，同样落到 `TooLarge`）。

两个用例值得看：往返一致性把两个方向的规则绑在一起；

```rust,ignore
{{#include ../examples/release-drill/src/lib.rs:228:234}}
```

另一个是「大数不许回绕」——这类断言是发布件里最该有的：

```rust,ignore
{{#include ../examples/release-drill/src/lib.rs:264:276}}
```

**第二层：文档测试（3 个）。** crate 级文档一个、`format` 一个、`parse` 一个，都由 `cargo test` 的 `Doc-tests` 段运行（第 31 章）。它们不是装饰：`format` 文档里那句「尾数小于 10 保留一位小数」如果哪天被改成两位，文档测试会当场变红，因为断言写的就是 `1.5 KiB`。

**第三层：发布门禁。** `fmt --check`、`clippy --all-targets -D warnings`、`RUSTDOCFLAGS=-D warnings cargo doc`、`cargo package` —— 不测行为，测发布件本身是否合格：文档有没有缺、有没有调试残留、包能不能在干净目录里编译。

一次完整运行的样子：

```console
$ cargo test --manifest-path examples/Cargo.toml -p release-drill
running 11 tests
test tests::bad_numbers_and_bad_units_are_reported_separately ... ok
test tests::bytes_below_one_kibibyte_keep_the_exact_count ... ok
test tests::empty_input_gets_its_own_error ... ok
test tests::error_display_names_the_offending_part ... ok
test tests::format_and_parse_agree_on_whole_kibibytes ... ok
test tests::format_walks_all_the_way_up_to_eib ... ok
test tests::jumbo_values_are_rejected_instead_of_wrapping ... ok
test tests::one_decimal_below_ten_integers_above ... ok
test tests::parse_accepts_both_unit_families ... ok
test tests::parse_rounds_to_the_nearest_byte ... ok
test tests::parse_treats_a_bare_number_as_bytes ... ok

test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests release_drill

running 3 tests
test release-drill\src\lib.rs - (line 8) ... ok
test release-drill\src\lib.rs - format (line 49) ... ok
test release-drill\src\lib.rs - parse (line 84) ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

（libtest 并行跑用例，顺序每次可能不同；这里是一次真跑的完整输出。）

> [!WARNING]
> 上面那两个 `rust,ignore` 的测试片段，本章的 `mdbook test` 不会去编译它们 —— 含 `#[test]` 的代码块被 cfg 排除，这是「假绿」的老坑（第 31 章有专门一节讲它）。测试这件事在本章只有一个可信来源：`cargo test` 的输出，也就是上面这一段。

## 发布清单

把上面所有零件串成一份可以照着做的清单。前六步都能在没有凭据的机器上做完，第七步开始需要账号。

1. **定版本**：改 `version`，按 SemVer 走（`0.x` 期间 minor 可以带破坏性变更）。这个库现在是 `0.1.0`。
2. **收尾变更日志**：把 `CHANGELOG.md` 的 `Unreleased` 一节改名成 `版本号 - 日期`，新的空 `Unreleased` 留在上面。
3. **补齐元数据**：`license`、`description`、`readme` 必须在，`repository`、`homepage` 该填了（本仓还没有公开远端，所以留着，代价是那条 warning）。`license` 只是 SPDX 声明：正式发布时仓库里要有对应全文，本项目样板没附。
4. **本地四条门禁全绿**：`cargo fmt --check`、`cargo clippy --all-targets -- -D warnings`、`cargo test`、`RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`。
5. **看清单**：`cargo package --list` 确认没漏文件、也没把测试数据塞进包；单个 `.crate` 有 10MB 上限（Cargo Book 原文：「crates.io currently has a 10MB size limit on the .crate file.」）。
6. **干跑一次**：`cargo publish --dry-run`，它会打包、解开、重新编译，但不上传。本机实测见「逐块实现」第 4 节。
7. **登录并发布**：`cargo login` 填 token（存在 `~/.cargo/credentials.toml`，是秘密，泄露要立刻吊销），然后 `cargo publish`。**本书不执行这一步**：没有凭据，而且发布不可逆 —— Cargo Book 的原话是「Take care when publishing a crate, because a publish is generally permanent. The version can never be overwritten, and the code cannot be deleted.」
8. **发布之后**：打一个指向该提交的 git tag（官方建议里明写了这条：「A git tag pointing to the published commit」）；docs.rs 会按 `[package.metadata.docs.rs]` 的配置自动构建文档；发现版本坏了用 `cargo yank --version 0.1.0`，它不删代码，只让新解析不再选中这个版本（第 30 章 30.7 讲了它的语义）。

这份清单里只有第 7 步不可逆，其余六步都能在本机反复跑 —— 把这两类分开，正是 CI 存在的意义。

## 小结

- 发布件不只是代码：元数据、README、变更日志、docs.rs 配置、打包检查各管一段，缺哪一件都会在别人手里变成问题。
- `cargo package --list` 先看清包里有什么：文件只有两个来源 —— 你写的和 Cargo 生成的。
- `cargo package` 的「解开再编译」是整条链上最值钱的一步：本地磁盘上有的文件，包里不一定有。
- 元数据按官方两档来：`license` 与 `description` 必填，其余「建议」；没有的东西不编（`repository` 空着，代价是一条 warning）。
- 门禁分两层：清单的 `[lints]` 管下限（本地也生效），CI 的 `-D warnings` 管上限（干净机器上全严）。
- 发布不可逆：可反复的检查放 CI，不可逆的那一步人工执行，并且先干跑。

## 练习

1. 为什么 `parse` 两套单位都收，而 `format` 只输出 IEC 那一套？把 `format` 改成输出 SI，会牵动哪些文件与断言？
2. `cargo package` 在 CI 里不需要 `--allow-dirty`，本地却需要。差别的来源是什么？这个 flag 让 `.crate` 里多了什么信息？
3. `missing_docs = "deny"` 写在清单的 `[lints]` 里，与在 CI 里跑 `RUSTDOCFLAGS="-D warnings" cargo doc` 相比，各自的失效场景是什么？

<details><summary>参考答案</summary>

**1.** 两个方向的读者不同：`format` 的输出要给人看日志，读者没有上下文去猜单位，所以只能有一种写法（选 IEC 是因为程序员说的「1 MB」几乎总是指 1048576 字节，而 `KiB` 这个后缀本身就把两种约定分开了）；`parse` 的输入来自别人的配置、别的工具的输出、人手抄的字符串，两套都可能出现，拒绝一种只会逼调用方自己写兼容层。把 `format` 换成 SI 要动：`IEC_UNITS` 换成 1000 进制的单位表、除法基准改成 1000、`format` 文档测试里的断言（`1.0 KiB` → `1.0 kB`、`1.5 KiB` → `1.5 kB`）、README 的单位规则表与示例，以及 `format_and_parse_agree_on_whole_kibibytes` —— 它按 `kib * 1024` 造数据，而 1024 字节在新规则下会格式化成 `1.0 kB`、再解析回 1000，往返直接断掉，用例得按 1000 进制重写。

**2.** CI 里 `actions/checkout` 拿到的是某个提交的干净拷贝；本机工作树里有刚写完还没提交的文件，Cargo 因此拒绝打包。多出来的信息是 `.cargo_vcs_info.json`：打包时的提交 sha 与 `dirty` 标记。它的意义是让发布件与仓库状态可对账 —— 脏树上打出的包 `dirty` 是 `true`，谁都能看出它不对应任何提交。所以正式发布先提交（再打 tag），`--allow-dirty` 只留给本地验证。

**3.** 清单里那条会被 `cargo` 翻译成 rustc 参数（错误里那句 `requested on the command line with -D missing-docs` 就是证据），**任何**构建这个包的场合都生效，包括本地；失效场景是别人把清单里那行删掉 —— 它管的是「这个包的构建」，不是「这个仓库的 CI」。`RUSTDOCFLAGS` 只管 CI 这一次运行，本地不生效，好处是管得更宽（rustdoc 的全部警告）且换机器统一。两者互补：清单保下限，CI 保每次合并都被最严的档筛过。

</details>

## 延伸阅读

- Cargo Book《Publishing on crates.io》（打包、发布、yank、10MB 限制）：<https://doc.rust-lang.org/cargo/reference/publishing.html>
- Cargo Book《The manifest format》（`[package]` 各键与 `[package.metadata]`）：<https://doc.rust-lang.org/cargo/reference/manifest.html>
- Cargo Book《Workspaces》（`examples/` 这种多 crate 布局）：<https://doc.rust-lang.org/cargo/reference/workspaces.html>
- Cargo Book《Continuous Integration》（CI 里的常用命令与缓存建议）：<https://doc.rust-lang.org/cargo/guide/continuous-integration.html>
- docs.rs《Metadata》（`[package.metadata.docs.rs]` 的全部键与默认值）：<https://docs.rs/about/metadata>
- docs.rs《Builds》（构建环境与 `docsrs` cfg）：<https://docs.rs/about/builds>
- rustdoc book《Linking to items by name》（intra-doc link 的解析规则）：<https://doc.rust-lang.org/rustdoc/write-documentation/linking-to-items-by-name.html>
- rustdoc 的 lint 列表（`broken_intra_doc_links` 等）：<https://doc.rust-lang.org/rustdoc/lints.html>
- Keep a Changelog（变更日志的格式约定）：<https://keepachangelog.com/zh-CN/1.1.0/>
- SemVer / 语义化版本（`0.x` 期间的规则）：<https://semver.org/lang/zh-CN/>
- Rust API Guidelines（库的接口与文档惯例）：<https://rust-lang.github.io/api-guidelines/>
