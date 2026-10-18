# 03 mdBook 能力盘点（findings）

> 票：`issues/03-mdbook-capabilities.md`
> 日期：2026-10-18
> 本机环境：Windows 10.0.26200（x64）、**mdbook v0.5.4**、rustc/cargo **1.98.0**（rustc `88d9e12ae 2026-08-18`）
> 一手来源：mdBook 官方文档（<https://rust-lang.github.io/mdBook/>，构建自仓库 master）、mdBook 仓库源码与 CHANGELOG、rustdoc book。
> **注意时效**：`rust-lang.github.io/mdBook` 的文档跟随 **master**，与本机 0.5.4 可能有微小漂移；凡本机实测过的，以实测为准（下文标「实测」）。所有实测命令均在 `tmp-mdbook/` 骨架（已删除，复现见文末）里执行。

---

## 0. 结论速览

| 能力 | 结论 |
|---|---|
| `mdbook test` | 真·rustdoc doctest；**只测 SUMMARY.md 里列出的章节**；退出码 0/101，可作为 CI 门禁 |
| 隐藏行 `#` | 渲染为 `<span class="boring">`，可展开；`##` 转义出字面 `#` |
| 代码块属性 | `ignore` / `no_run` / `should_panic` / `compile_fail` / `editable` / `noplayground` / `mdbook-runnable` / `editionXXXX` 全部可用（实测） |
| **无语言栅栏** | **会被当 Rust 测**，散文里的裸栅栏必须写 ` ```text ` |
| `[rust] edition` | **默认 2015**！必须显式设 `edition = "2024"`，否则 `async`/新版语法全灭（实测） |
| playground | 走公网 `play.rust-lang.org`，中国大陆网络下不可靠 |
| 数学 | `mathjax-support = true` → 从 **cdnjs CDN** 拉 **MathJax 2.7.1**（不是 KaTeX），`$`/`$$` 不被 mdBook 处理 |
| Mermaid | **0.5.4 无内置**，需第三方 preprocessor（`mdbook-mermaid`，活跃） |
| admonitions / 定义列表 | 0.5 内置且默认开启（`> [!NOTE]`），很适合做「提示/警告」盒 |
| 链接检查 | 原 `mdbook-linkcheck` 停滞（最后发版 2022-10），官方 0.5.4 起改推 **`mdbook-linkcheck2`** |
| `mdbook build` 静默陷阱 | ① 缺失 `{{#include}}` ② `[output.html]` 里有未知字段 → **都只打 ERROR 然后 exit 0**，配置还可能被整体丢弃（实测+源码） |
| 中文搜索分词 | 分词器**只按空白和 `-` 切**，中文整句成一个 token；`use-unicode-segmentation` 这个字段在 0.4/0.5 都**不存在** |

---

## 1. 代码校验：`mdbook test`

### 1.1 机制

官方：「mdBook supports a `test` command that will run all available tests in a book. At the moment, only Rust tests are supported.」
来源：<https://rust-lang.github.io/mdBook/cli/test.html>

即 `mdbook test` 把每个章节的 markdown 交给 **rustdoc 的 doctest 机制**跑，因此**所有 rustdoc doctest 规则与属性都适用**。本机还有一条佐证：错误输出里出现 rustdoc 的 `merged doctests compilation took ...`，说明就是 rustdoc 在干活（实测）。

CLI 只有三个入口（实测 `mdbook test --help`）：

```
[dir]                     书根目录（默认 cwd）
-c, --chapter <chapter>   只测某章（章节名或相对路径）
-L, --library-path <dir> 逗号分隔的额外库搜索路径（rustdoc -L）
```

**只测 SUMMARY.md 里列出的章节**：实测在 `src/` 放一个不在 SUMMARY 的 `orphan.md`（内含 `this_does_not_compile`），全量 `mdbook test` 的章节列表里**不含**它，`grep -c orphan` = 0，且整体退出 0。
⇒ 结论：**任何想被校验的示例，都必须作为章节出现在 SUMMARY.md 中**（或被 `{{#include}}` 进某个章节）。

### 1.2 哪些代码块会被测（实测）

| 写法 | 是否被 test | 渲染出的 HTML |
|---|---|---|
| ```` ```rust ```` | ✅ 测 | `<pre class="playground"><code class="language-rust edition2024">` |
| ```` ``` ````（无语言） | ✅ **测**（当 Rust） | `<pre><code>` |
| ```` ```rust,ignore ```` | ❌ 跳过 | `<pre><code class="language-rust ignore">`（无 play 按钮） |
| ```` ```rust,no_run ```` | ✅ 编译、不运行 | `<pre class="playground"><code class="language-rust no_run">` |
| ```` ```rust,should_panic ```` | ✅ 运行且期望 panic | `class="playground"` + `should_panic` |
| ```` ```rust,compile_fail ```` | ✅ 期望编译失败 | `class="playground"` + `compile_fail` |
| ```` ```rust,editable ```` | ✅ 测 | `class="playground"` + `editable`（附 ACE 编辑器） |
| ```` ```rust,noplayground ```` | ✅ 测（只是没按钮） | `<pre><code class="language-rust noplayground ...">` |
| ```` ```rust,ignore,mdbook-runnable ```` | ❌ 跳过 | **`<pre class="playground">`**（有按钮） |
| ```` ```python ```` 等非 rust | ❌ 跳过 | `class="language-python"` |

**「无语言栅栏」是最大的坑（实测）**：专门写了一个章节，裸栅栏里放 `this is not valid rust at all`，结果：

```
test ch9-bare.md - Bare (line 5) ... FAILED
error: expected one of `!` or `::`, found `is`
```

官方文档原文也警告过：「rustdoc *does* test code blocks which have no language specified」。
⇒ **规范要求：正文里所有非 Rust 的示意片段（伪代码、目录树、输出）必须显式标 `text`/`console`/`toml` 等语言，绝不能留裸栅栏。**

### 1.3 play 按钮的真实显示逻辑（源码级证据）

从**已构建产物** `book/book-*.js` 读出的实际逻辑：

```js
const playgrounds = Array.from(document.querySelectorAll('.playground'));   // 先对所有 .playground 建按钮
function update_play_button(pre_block, playground_crates) {
    const play_button = pre_block.querySelector('.play-button');
    // skip if code is `no_run`
    if (pre_block.querySelector('code').classList.contains('no_run')) {
        play_button.classList.add('hidden');   // no_run → 运行时隐藏按钮
        return;
    }
    // ... 解析片段里的 `extern crate X;`，与 https://play.rust-lang.org/meta/crates 比对，
    //     若用到了 playground 没有的 crate，也会隐藏按钮
}
```

要点（前三条为**已构建 JS 的源码事实**）：
- `ignore` / `noplayground`：**根本不生成**按钮（没有 `.playground` class）。
- `no_run`：生成了按钮但**运行时隐藏**（这解释了「HTML 里是 playground class，但文档说 no play button」的表面矛盾）。
- **`compile_fail` / `should_panic`：`update_play_button` 里没有针对它们的隐藏分支 ⇒ [推断] 按钮照常显示**（官方文档未承诺隐藏，但这是从源码读出的行为，未用浏览器确认）。
- 只要页面里有 `.playground`，前端就会去 `fetch('https://play.rust-lang.org/meta/crates')` 做 crate 可用性检查（源码事实）；playground 真正执行走 `https://play.rust-lang.org/evaluate.json`（源码事实）。
  ⇒ [推断] 离线/被墙时该 fetch 会走到超时分支，中国读者「点 Run 转圈/失败」是预期现象；建议提前在书里说明，或全局关掉（见 §2.1）。

### 1.4 隐藏行与 `##` 转义（实测）

源：

````markdown
```rust
# fn main() {
    let x = 5;
# }
```
````

渲染为（实测 HTML）：

```html
<pre class="playground"><code class="language-rust edition2024"><span class="boring">fn main() {
</span>    let x = 5;
<span class="boring">}</span></code></pre>
```

`# ` 前缀的行进入 `<span class="boring">`，读者可点「眼睛」展开；**这些行仍参与编译**（所以 `# fn main() {` 这种包裹法是成立的）。

`##` 转义：源码行 `## [link](x)` 渲染成**不隐藏**的字面 `# [link](x)`（实测）。
⚠️ 但注意：**`##` 转义只解决渲染**。喂给 `mdbook test` 的仍是被转义后的 `# [link](x)`，rustdoc 会把 `#`+`[link]` 当属性，测试直接报错（实测）：

```
error: expected item after attributes
  --> ch7-wrapper.md:26:1
26 | # [link](x)
```

⇒ **规则：想展示「以 `#` 开头的内容」，别放在 `rust` 块里；用 `text` 语言块，或给该块加 `ignore`。**

其他语言的隐藏前缀可配：`[output.html.code.hidelines]`（例如 `python = "~"`），块内可用 ` ```python,hidelines=!!! ` 局部覆盖。
来源：<https://rust-lang.github.io/mdBook/format/mdbook.html#hiding-code-lines>

### 1.5 `compile_fail` 与「教学故意报错」

`compile_fail` 是**正向断言**：块必须**编译失败**，测试才算通过。所以教学里展示「这样写会报错」的正确姿势就是：

````markdown
```rust,compile_fail
let x: i32 = "not a number";   // E0308
```
````

实测该块让 `mdbook test` 通过。配套写法建议：
- 想展示**具体错误信息**：错误文本本身放 ```` ```text ```` 块（不会被编译）。
- 只想让读者自己看报错、不想被断言：用 `ignore`。
- 想让读者能运行：`ignore,mdbook-runnable`。
- 想锁死「某个错误码」：`compile_fail` 在 mdBook/rustdoc 这层**不支持** `E0308` 之类细化断言；若确需，只能靠 `trybuild` 之类的独立测试，mdBook 无内置。

### 1.6 退出码（实测）

| 命令 | 结果 |
|---|---|
| 全部通过 | `exit=0`，逐章打印 `INFO Testing chapter '…'`，无别的输出 |
| 有测试失败 | `exit=101`，打印该章 rustdoc 的完整 stdout（`test … FAILED` + 编译错误） |
| `[output.html]` 里有非法字段 | **不报错、exit=0**（见 §1.7） |

⇒ `mdbook test` 是**可靠的非零退出门禁**，适合直接卡在 CI 上。

### 1.7 `mdbook test` 不校验 `[output.html]`（实测，Main 追加问题 2 的相关结论）

在 `book.toml` 里故意写非法字段 `[output.html.search] use-unicode-segmentation = true` 时：

- `mdbook build` → 打印 `ERROR Failed to deserialize 'output.html' …`（但仍 exit 0，见 §3.2）
- `mdbook test` → **完全没有这条 ERROR**（`grep -c deserialize` = 0），exit 0

原因：`test` 不跑 html 渲染器，因此从不反序列化 `[output.html]`。
⇒ **CI 不能只跑 `mdbook test`**：HTML 配置类错误只有 `mdbook build` 才会暴露（而 build 又是 exit 0……见 §3.2，结论是必须靠 grep 日志或额外检查）。

### 1.8 外部 crate：`-L` 的真实能力边界（实测，重要）

官方推荐 `mdbook test my-book -L target/debug/deps/`（<https://rust-lang.github.io/mdBook/cli/test.html#--library-path>）。实测结论**比文档暗示的窄得多**：

搭一个最小 lib crate `companion`（`cargo build` 产出 `companion/target/debug/deps/libcompanion-*.rlib`），章节里三种写法：

| 场景 | 不加 `-L` | 加 `-L companion/target/debug/deps` |
|---|---|---|
| `extern crate companion;`（2015 风格） | ❌ `E0463 can't find crate` | ✅ **ok** |
| `use companion::add;`（2018+ 路径导入，全局 edition=2024） | ❌ `E0432 unresolved import` | ❌ **仍然 `E0432`** |
| `# extern crate companion;`（隐藏行垫片）+ `use companion::add;` | ❌ | ✅ **ok** |

原因：`-L` 只是把目录加进**库搜索路径**，rustdoc 生成 doctest 时**不会**为它补 `--extern`；`extern crate` 能命中搜索路径，2018+ 的 `use 路径` 需要 extern prelude 里的 `--extern` 才能解析。
另外实测确认**无法绕过**：

- `RUSTDOCFLAGS="--extern companion=…"` → **被忽略**（`RUSTDOCFLAGS` 是 cargo 的东西，mdbook 直接 exec rustdoc）→ 仍 `E0463`。
- `mdbook test … -- --extern …` → clap 直接报 `unexpected argument`，**没有 `--` 透传**。

⇒ **可执行建议**：书里要引用配套 crate 时，在该代码块顶部写一行隐藏的 `# extern crate my_crate;`，再正常 `use`；CI 里 `mdbook test -L path/to/target/debug/deps`。或者干脆**不引用外部 crate**（见 §5）。

### 1.9 调试技巧：`[output.markdown]` 看 mdBook 到底喂了什么给 rustdoc

官方：「The Markdown renderer will run preprocessors and then output the resulting Markdown. This is mostly useful for debugging preprocessors, especially in conjunction with `mdbook test` to see the Markdown that `mdbook` is passing to `rustdoc`.」
来源：<https://rust-lang.github.io/mdBook/format/configuration/renderers.html#markdown-renderer>

实测：`book.toml` 加 `[output.markdown]` 后 `mdbook build`，产出变成 `book/html/` + `book/markdown/`（**多 backend 时自动分子目录**），`book/markdown/ch3-playground.md` 里能直接看到 `{{#playground example.rs}}` 被展开成普通 ```` ```rust ```` 块（原样、无 `#` 前缀），而 `\{{#…}}` 的转义反斜杠被吃掉。
⚠️ 这条也是「渲染 HTML 目录会从 `book/` 变成 `book/html/`」的来源，配 Pages 时要留神。

---

## 2. 渲染能力

### 2.1 Rust Playground（按钮 / `{{#playground}}`）

官方：Rust 代码块**自动**获得 play 按钮，代码发送到 <https://play.rust-lang.org/> 执行。
来源：<https://rust-lang.github.io/mdBook/format/mdbook.html#rust-playground>

- **没有 `main` 会被自动包裹**：实测 0.5.4 在 HTML 里直接插入隐藏包裹行：
  ```html
  <pre class="playground"><code class="language-rust edition2024"><span class="boring">#![allow(unused)]
  </span><span class="boring">fn main() {
  </span>let x = 5;
  println!("{x}");
  <span class="boring">}</span></code></pre>
  ```
  注意包裹**只对 `.playground` 块**发生：`ignore` / `noplayground` / 已有 `fn main` 的块**不加**包裹（实测对比）。
- 关闭单个块：`noplayground`。全局关：`[output.html.playground] runnable = false`。
- `[output.html.playground]` 全套：`editable`(false) / `copyable`(true) / `copy-js`(true) / `line-numbers`(false) / `runnable`(true)。
- `{{#playground file.rs}}`：把**外部 .rs 文件**插成可运行块（路径相对当前源文件）。附加属性可跟在文件名后，如 `{{#playground example.rs editable}}` → 生成 ```` ```rust,editable ````。实测有效。
- **⚠️ 网络依赖**：按钮/自动补全都打公网（`play.rust-lang.org/evaluate.json`、`/meta/crates`）。中国读者体验会差；建议书里显式说明「Run 需要联网访问 play.rust-lang.org」，或直接 `runnable = false` 并把可运行示例改成随书仓库。

### 2.2 `{{#…}}` 指令的转义坑（实测踩过，值得写进规范）

mdBook 的 `{{#include}}` / `{{#playground}}` / `{{#rustdoc_include}}` / `{{#title}}` 由 **links preprocessor** 处理，**在行内代码里也会展开**：

- 源码 `` `{{#playground example.rs}}`： ``（我本想展示指令本身）→ 被展开成代码块，导致栅栏错乱，`mdbook test` 报了 21 个 `unknown start of token: \`` 错误。
- 正确写法：`\{{#playground example.rs}}`（反斜杠转义）。实测转义后正常，且 `[output.markdown]` 里能看到反斜杠被吃掉、只剩字面指令。
- 同一机制也适用于 `{{#include}}`。

**其他可用指令**（官方 <https://rust-lang.github.io/mdBook/format/mdbook.html>）：
- `{{#include file.rs}}`：**原样**插入（Markdown 语义），范围/锚点：`:2`、`::10`、`:2:`、`:2:10`、`file.rs:anchorname`（`ANCHOR:` / `ANCHOR_END:` 正则匹配，任何注释行都能放锚点）。
- `{{#rustdoc_include file.rs:2}}`：插入**整个文件**、但把范围外的行加 `#` 前缀隐藏 → 读者看到片段，`mdbook test` 看到完整程序。**这是「部分展示 + 完整编译」的正解。**
- `{{#title My Title}}`：改页面 `<title>`。

### 2.3 数学：实测是 MathJax 2.7.1 + CDN，`$`/`$$` 不处理

官方（<https://rust-lang.github.io/mdBook/format/mathjax.html>）：需在 `[output.html]` 写 `mathjax-support = true`；行内 `\\( … \\)`，块级 `\\[ … \\]`；文档明确说「**不能**用 `$$ … $$`」，「`\\` 换行需要写 4 个反斜杠」。

**本机实测（v0.5.4 自己的产物）**：
- 未开 `mathjax-support` 时，`$E=mc^2$` 与 `$$…$$` **原样留在 HTML 文本里**，无任何 katex/mathjax 资源（`ls book | grep -i katex` 为空）。
- 开 `mathjax-support = true` 后，HTML head 里注入：
  ```html
  <!-- MathJax -->
  <script async src="https://cdnjs.cloudflare.com/ajax/libs/mathjax/2.7.1/MathJax.js?config=TeX-AMS-MML_HTMLorMML"></script>
  ```
  **没有 KaTeX**；`book/` 里**不落任何本地数学资源文件**——完全依赖 CDN `cdnjs.cloudflare.com`。
- 因此 `$…$` / `$$…$$` 能否渲染取决于 MathJax 2 的运行时配置，而 mdBook 文档**明确说 `$$` 不支持**。
  **[未验证]** 用真实浏览器逐段确认四种定界符（`$`、`$$`、`\(`、`\[`）的最终渲染结果——本票只做了静态产物检查，没起浏览器。**给写作的保守结论：只使用文档承诺的 `\\(…\\)` / `\\[…\\]`。**
- 另一个实测细节（不是 bug）：CommonMark 的反斜杠转义会吃掉 `\,`（`,` 是 ASCII 标点），所以 `e^{-x^2}\,dx` 渲染出来变成 `e^{-x^2},dx`。**数学里避免用 `\,` 之类的标点转义**，或改用 `\;`/`\quad`。

**取舍建议**：如果书里数学不多，与其引入 CDN 依赖 + 2.7.1 老版本 MathJax，不如用图片/纯文本公式说明；真要上，务必在发布前用浏览器实测中国大陆网络下 CDN 是否可达。

### 2.4 Mermaid / 图表

- **mdBook 0.5.4 无内置 Mermaid**。实测 ```` ```mermaid ```` 只被渲染成普通代码块：
  ```html
  <pre><code class="language-mermaid">graph TD …</code></pre>
  ```
- 官方文档把第三方 backend/plugin 列表放在 wiki：<https://github.com/rust-lang/mdBook/wiki/Third-party-plugins>
- 事实核查（2026-10-18，GitHub/crates.io API）：
  | 项目 | 状态 |
  |---|---|
  | `badboy/mdbook-mermaid` | 未归档，最后 push 2026-08-17，451★；crates.io **v0.17.1**，2026-08-14 发布，累计 614k 下载 **（活跃）** |
  - 安装：`cargo install mdbook-mermaid`（或 `cargo binstall`、或 Releases 下二进制）。
  - 配置（README）：`mdbook-mermaid install path/to/book` 会自动写
    ```toml
    [preprocessor.mermaid]
    command = "mdbook-mermaid"

    [output.html]
    additional-js = ["mermaid.min.js", "mermaid-init.js"]
    ```
    即把 `mermaid.min.js` 等**落到书源目录并本地引用**（不像 MathJax 那样吃 CDN，这点对中国读者更友好）。
  - **[未验证]** 本机没有装 `mdbook-mermaid` 跑通（安装需编译，票里允许不装）。上面全部来自其 README + crates.io 元数据。
- **SVG / 位图内嵌**：直接写 `<img src="….svg">` 或 Markdown 图片语法即可，mdBook 不做净化、不校验文件存在（存在性交给 linkcheck）。实测 `<img src="rust-logo.svg" …>` 原样输出。
- mdBook **0.5.4 新增 zoomable images**（CHANGELOG: “Added zoomable images feature.” #3127）。

### 2.5 自定义主题

- 配置：`[output.html] theme = "my-theme"`（相对书根），mdBook **选择性覆盖**默认主题文件（没覆盖的用内置默认）。
- 最快上手：`mdbook init --theme`（另有 `--force`、`--title`、`--ignore none|git`）。实测生成的 `theme/` 文件清单：
  `book.js`、`index.hbs`、`highlight.js`、`highlight.css`、`favicon.png`、`favicon.svg`、
  `css/{chrome,general,print,variables}.css`、
  `fonts/{fonts.css, *.woff2, OPEN-SANS-LICENSE.txt, SOURCE-CODE-PRO-LICENSE.txt}`
- 精细修改不想整份覆盖时用 `additional-css` / `additional-js`（在默认样式之后加载）。
- **0.5 迁移警示**：CHANGELOG 的 0.5 Migration Guide 明确说「If you have overridden any of the theme files, you will likely need to update them to match the current version.」并移除了 `{{#previous}}`/`{{#next}}`/`{{theme_option}}` 等 handlebars helper、`output.html.copy-fonts`、`curly-quotes`、google-analytics 配置。
  来源：<https://github.com/rust-lang/mdBook/blob/master/CHANGELOG.md>
  ⇒ **本项目的取舍：尽量少改主题**，用 `additional-css` 加小补丁，能显著降低 0.5→0.6 迁移成本。

### 2.6 `book.toml` 里与本书直接相关的配置项（全部来自官方文档）

来源：<https://rust-lang.github.io/mdBook/format/configuration/general.html>、<https://rust-lang.github.io/mdBook/format/configuration/renderers.html>

`[book]`：`title` / `authors` / `description` / `src` / `language` / `text-direction`
`[rust]`：`edition`（**默认 2015**）
`[build]`：`build-dir`（默认 `book`）/ `create-missing`（默认 true）/ `use-default-preprocessors` / `extra-watch-dirs`
`[output.html]`：`theme` / `default-theme` / `preferred-dark-theme` / `smart-punctuation`(默认 true) / `definition-lists`(true) / `admonitions`(true) / `mathjax-support`(false) / `additional-css` / `additional-js` / `no-section-label` / `git-repository-url` / `git-repository-icon` / `edit-url-template` / `site-url` / `cname` / `input-404`(默认 `404.md`) / `hash-files`(true) / `sidebar-header-nav`(true)
子表：`[output.html.print]`、`[output.html.fold]`、`[output.html.playground]`、`[output.html.code]`(hidelines)、`[output.html.search]`、`[output.html.redirect]`

`[output.html.search]` 的**合法字段全集**（源码 `crates/mdbook-core/src/config.rs` 的 `pub struct Search` + 实测报错信息）：
`enable`、`limit-results`、`teaser-word-count`、`use-boolean-and`、`boost-title`、`boost-hierarchy`、`boost-paragraph`、`expand`、`heading-split-level`、`copy-js`、`chapter`
（另有子表 `[output.html.search.chapter]`，按路径开关索引。）

### 2.7 ⚠️ `[rust] edition` 默认 2015 —— 必须显式设置（实测）

`[rust] edition` 默认 `"2015"`。实测：`book.toml` 不加该项时，一个 `async fn f() {}` 的块直接失败：

```
4 | async fn f() {}
  | ^^^^^ to use `async fn`, switch to Rust 2018 or later
  = help: pass `--edition 2024` to `rustc`
error: aborting due to 1 previous error
```

加上 `[rust] edition = "2024"` 后同一块通过（实测）。
⇒ **07 号票（仓库骨架）的 `book.toml` 必须写 `[rust] edition`**，且与 06 号票定的 edition 保持一致；否则整本书的示例都在 2015 语义下被校验（`dyn`、`?`、`use` 路径、数组 `into_iter` 行为等全部可能对不上）。
单块可用 `edition2015` / `edition2018` / `edition2021` / `edition2024` 属性覆盖。

### 2.8 0.5 新内置 Markdown 特性（值得用作书里的小组件）

CHANGELOG 的 0.5 Migration Guide「Major additions」：
- **admonitions**（默认开，`output.html.admonitions` 可关）——实测渲染：
  ```html
  <blockquote class="blockquote-tag blockquote-tag-note">
    <p class="blockquote-tag-title"><svg …/>Note</p>
    <p>这是一个提示框。</p>
  </blockquote>
  ```
  写法：`> [!NOTE]` / `> [!WARNING]` …。**非常适合本项目的「基础补充盒 / 注意 / 警告」**，比自造 HTML 干净。
- **definition lists**（默认开，`output.html.definition-lists`）
- **sidebar 标题导航**（`output.html.sidebar-header-nav`）
- 0.5.4 的 **zoomable images**
- 来源：<https://rust-lang.github.io/mdBook/format/markdown.html#admonitions>

其他内置能力（官方 mdbook.md 页）：HTML class `left`/`right`（浮动图片）、`hidden`；Font Awesome 图标（`<i class="fa-solid fa-print">`，内置 v6 免费集，结果转 inline SVG）。

---

## 3. 质量管线

### 3.1 子命令（实测 `mdbook --help`，v0.5.4）

`init` / `build` / `test` / `clean` / `completions` / `watch` / `serve` / `help`

- `build`：`-d/--dest-dir`、`-o/--open`
- `serve`：`-d`、`-n/--hostname`(localhost)、`-p/--port`(3000)、`-o`、`--watcher <poll|native>`（**默认 `poll`**）
- `watch`：监听并重建；`[build] extra-watch-dirs` 可加监视目录（书依赖 `src/` 之外的示例文件时用得上）

### 3.2 ⚠️ `mdbook build` 的两个静默失败（实测 + 源码，必读）

**(a) 缺失 `{{#include}}` → ERROR 但 exit 0**
```
ERROR Error updating "{{#include does-not-exist.rs}}", Could not read file for link {{#include does-not-exist.rs}} (…\src\does-not-exist.rs)
 INFO HTML book written to …
build exit=0
```
且该章节照常产出 HTML（指令原样留在页面上），`book/` 全量重建成功。
附带效应：**这种章节 `mdbook test` 会挂**（字面 `{{#include …}}` 被当 Rust 编译，`error: expected one of '!' or '['`），所以 test 反而是这里的门禁。

**(b) `[output.html]` 出现未知字段 → ERROR、整表被丢弃、exit 0**（Main 追加问题 2）
实测（清空 `book/` 后重建，全部重建成功、exit=0）：
```
ERROR Failed to deserialize `output.html`
	Caused by: unknown field `use-unicode-segmentation`, expected one of `enable`, `limit-results`, … `chapter`
in `search`
 INFO HTML book written to …
build exit=0
```
源码级解释（`crates/mdbook-core/src/config.rs`，master）：
```rust
#[serde(default, deny_unknown_fields)]   // ← 未知字段 = 反序列化失败
pub fn html_config(&self) -> Option<HtmlConfig> {   // ← 注意返回 Option，不是 Result
    match self.get("output.html") {
        Ok(Some(config)) => Some(config),
        Ok(None) => None,
        Err(e) => { log_backtrace(&e); None }        // ← 错误只记日志，然后吞掉
    }
}
```
调用点（`crates/mdbook-html/src/html_handlebars/hbs_renderer.rs`）：
```rust
let html_config = ctx.config.html_config().unwrap_or_default();   // ← 整体退回 HtmlConfig::default()
```
⇒ **后果比「报个错」严重**：`[output.html]` 里只要**任何一个**字段拼错，**整个 `[output.html]` 表被静默替换为默认值**（`site-url`、`additional-css`、`playground.runnable = false`、`mathjax-support` … 全部失效），构建仍报成功、exit 0。
官方层面：CHANGELOG 的 0.5 Migration Guide「Config changes」首条即「**Unknown fields in config are now an error.**」(#2787 #2801)——但实现上「error」只是日志级别，不是进程退出码。

**CI 建议（推论，供 07 号票落地）**：`mdbook build` 的**退出码不能**作为配置/链接健康的门禁；必须
1. `mdbook build 2>&1 | tee build.log`，再 `grep -qE '^ERROR' build.log && exit 1`；或
2. 引入 linkcheck（§3.4，backend 失败会以错误形式暴露——**[未验证]**其确切退出码语义）；或
3. 写一个极小的 CI 自检：解析 `book.toml` 的字段白名单。
另可加 `[build] create-missing = false`，让 SUMMARY 里指向不存在的文件**报错退出**（默认 true 会自动建空文件，属于另一种静默）。**[未验证]** `create-missing=false` 的确切退出码未在本机实测。

### 3.3 `mdbook serve`（实测，起了 43.6s 后按票要求收掉）

```
INFO Serving on: http://localhost:3111
INFO Watching for changes...
```
`curl -o /dev/null -w '%{http_code}' http://localhost:3111/` → **200**。默认 watcher 是 `poll`（Windows 上更稳，但 CPU/延迟略高，可用 `--watcher native`）。
工作流建议：日常写作 `mdbook serve`，提交前跑 `mdbook test` + `mdbook build`。

### 3.4 链接检查：`mdbook-linkcheck` 的现状与替代（事实核查 2026-10-18）

| 项目 | 状态（GitHub / crates.io API 实测） |
|---|---|
| `rust-lang/mdBook-linkcheck` | **404 Not Found**（仓库已不在该路径） |
| `Michael-F-Bryan/mdbook-linkcheck`（原始） | 未归档，最后 push **2024-08-15**；crates.io **v0.7.7，最后发版 2022-10-03**，499,987 下载 ⇒ **实质停滞** |
| `marxin/mdbook-linkcheck2`（fork） | 未归档，最后 push **2026-08-16**；crates.io **v0.13.0**，2026-08-16 更新，69,764 下载 ⇒ **活跃** |

官方立场（一手）：mdBook 0.5.4 CHANGELOG 里有一条 “**Switched recommendation to linkcheck2 fork.** (#3132)”，且官方 CI 文档正文现在写 “You may also want to consider running other kinds of tests, like **mdbook-linkcheck2**”（<https://rust-lang.github.io/mdBook/continuous-integration.html>）。
⇒ **结论：新项目直接用 `mdbook-linkcheck2`，不要用老的 `mdbook-linkcheck`。**

安装与配置（来自 linkcheck2 README，<https://github.com/marxin/mdbook-linkcheck2>）：
```sh
cargo install mdbook-linkcheck2
```
```toml
[output.html]

[output.linkcheck2]     # 兼容旧名 [output.linkcheck]
```
常用项：`follow-web-links`(false，开外网检查性能开销大)、`traverse-parent-directories`(false，安全)、`require-md-extension`(false)、`exclude`(正则数组)、`user-agent`、`cache-timeout`(12h)、`warning-policy`(`warn`|`error`|`ignore`)、`[output.linkcheck2.http-headers]`（可按站点注入 header，支持 `$ENV` 插值）。
⚠️ **必须注意**：一旦 `[output]` 表多于一个，HTML 输出就从 `book/` 挪到 `book/html/`（README 明确提示 + 本机用 `[output.markdown]` 实测到同一行为）——部署脚本/Pages 路径要同步改。
**[未验证]** 本机未安装 linkcheck2（编译耗时，票里允许跳过）；「链接坏掉时 `mdbook build` 是否 exit≠0」未实测，**不要当作已验证结论**。

### 3.5 GitHub Actions：构建 + 部署 Pages（一手：mdBook wiki，2026-10-18 抓取）

来源：<https://github.com/rust-lang/mdBook/wiki/Automated-Deployment>、<https://github.com/rust-lang/mdBook/wiki/Automated-Deployment:-GitHub-Actions>

**(a) 跑测试**（wiki 原文，官方推荐先装预编译二进制而非 `cargo install`）：
```yaml
name: CI
on: [push, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@master
    - name: Install Rust
      run: |
        rustup set profile minimal
        rustup toolchain install stable
        rustup default stable
    - name: Install latest mdbook
      run: |
        tag=$(curl 'https://api.github.com/repos/rust-lang/mdbook/releases/latest' | jq -r '.tag_name')
        url="https://github.com/rust-lang/mdbook/releases/download/${tag}/mdbook-${tag}-x86_64-unknown-linux-gnu.tar.gz"
        mkdir bin
        curl -sSL $url | tar -xz --directory=bin
        echo "$(pwd)/bin" >> $GITHUB_PATH
    - name: Run tests
      run: mdbook test
```
⚠️ 上面是「装 latest」，与 `book.toml`/CI 的**版本可复现性**冲突。官方 CI 文档给的可复现写法（<https://rust-lang.github.io/mdBook/continuous-integration.html>）：
`cargo install mdbook --no-default-features --features search --vers "^0.5.4" --locked`（`--no-default-features` 去掉 serve 用的 HTTP server，显著省编译时间）。
**建议：本项目固定 `=0.5.4`（或下载 `v0.5.4` 的 tar.gz），避免 latest 漂移。**

**(b) 部署 Pages（Actions 版）**，wiki 原文要点：Settings → Pages 选 **GitHub Actions**；workflow：
```yaml
jobs:
  deploy:
    runs-on: ubuntu-latest
    permissions:
      contents: write
      pages: write
      id-token: write
    steps:
      - uses: actions/checkout@v4
        with: { fetch-depth: 0 }
      - name: Install latest mdbook
        run: |
          tag=$(curl 'https://api.github.com/repos/rust-lang/mdbook/releases/latest' | jq -r '.tag_name')
          url="https://github.com/rust-lang/mdbook/releases/download/${tag}/mdbook-${tag}-x86_64-unknown-linux-gnu.tar.gz"
          mkdir mdbook
          curl -sSL $url | tar -xz --directory=./mdbook
          echo `pwd`/mdbook >> $GITHUB_PATH
      - name: Build Book
        run: mdbook build
      - name: Setup Pages
        uses: actions/configure-pages@v4
      - name: Upload artifact
        uses: actions/upload-pages-artifact@v3
        with: { path: 'book' }
      - name: Deploy to GitHub Pages
        id: deployment
        uses: actions/deploy-pages@v4
```
wiki 另给了「不用第三方 action、直接 `git worktree` 推 `gh-pages`」的裸脚本，以及 `XAMPPRocky/deploy-mdbook`、`JamesIves/github-pages-deploy-action`、Vercel 方案。
注意：若同时启用了 linkcheck2/markdown renderer，`path:` 要改成 `book/html`。

**本地优先的工作流（本项目采用建议）**：本地只做 `mdbook serve` / `test` / `build`；CI 里跑 `mdbook test` + `mdbook build` + linkcheck2；部署到 Pages。远端公开按 map 决定推后。

### 3.6 404 与 `site-url`

mdBook 自动生成根目录 `404.html`（GitHub Pages 会自动使用）。**若书不部署在域名根路径，必须设 `[output.html] site-url`**（如 `/rust-guide/`），否则 404 页与相对资源会错；自定义 404 内容写 `src/404.md`，文件名可用 `output.html.input-404` 改。
来源：<https://rust-lang.github.io/mdBook/continuous-integration.html#404-handling>

---

## 4. 示例工程结构：多文件示例怎么组织

### 4.1 四种可选形态与实测取舍

| 形态 | 写法 | 优点 | 缺点（实测/文档） |
|---|---|---|---|
| A. 章内内联块 | 直接写在 `.md` 里 | 零结构成本；`mdbook test` 全覆盖 | 无法复用；长示例难维护；散文与代码耦合 |
| B. `{{#include ../examples/foo.rs}}` | 真实 .rs 文件进仓库 | 代码可 `cargo build`/`cargo test` 独立验证；编辑器友好 | **原样插入**，展示与编译必须一致（或用锚点裁剪）；跨目录 include **实测可行**（`{{#include ../companion/src/lib.rs}}` 渲染并通过 test） |
| C. `{{#rustdoc_include foo.rs:2}}` | 引用完整文件、只显示部分 | **读者看片段、rustdoc 编译完整程序**（唯一兼得的方式） | 只能按行号/锚点；文件需保持「完整可编译」 |
| D. 配套 crate + `mdbook test -L` | 例：`examples/` 或 `companion/` crate | 代码是真正的库，可 `cargo test` | **受限**：`-L` 只对 `extern crate` 生效，2018+ `use` 必须用隐藏行 `# extern crate …;` 垫片；无 `--extern` 透传（§1.8） |

补充实测事实：
- **只有 SUMMARY 里的章节会被 `mdbook test` 覆盖**；`{{#include}}` 引用的 `.rs` 文件自身**不会**被单独测试（它们只是文本），所以 B/D 形态要么被 include 进章节、要么靠 `cargo test` 兜。
- 多 backend（如同时开 html+markdown+linkcheck）会改变输出布局（`book/html`），影响部署路径。
- `[build] create-missing = false` 可让 SUMMARY 指向缺失文件时报错；默认 `true` 会**静默建空文件**。

### 4.2 建议给本项目（供 07 号票）

推荐混合 B/C/D：

```
D:/Rust/
├── book.toml                 # [rust] edition 必填；[output.html] site-url 等
├── src/                      # 章节目录（mdbook）
│   ├── SUMMARY.md
│   ├── ch-xx.md
│   └── snippets/<章>/        # 该章专用的可编译 .rs 片段（用 {{#rustdoc_include}} 引入）
└── examples/                 # 实战项目的配套 crate（workspace 成员）
    ├── Cargo.toml
    └── part-1-.../src/lib.rs
```

- **短示例** → 章内内联（A），保持阅读流畅。
- **需要「读者看 3 行、实际是完整程序」** → 放 `src/snippets/` 的独立 `.rs` + `{{#rustdoc_include}}`（C）。
- **实战项目** → `examples/` 下的独立 crate（D），**由 `cargo test` 直接负责正确性**；书里只 `{{#include}}` 关键片段（必要时加 `ignore` 或 `rustdoc_include`）。这样避开 `-L`/`--extern` 的所有限制。
- 每章末尾或专门一节写清「本章示例对应仓库路径」，让读者能 `cargo run -p part-1`。

**一个可执行的检查清单（专给本项目）**：
1. `book.toml` 必须有 `[rust] edition = "2024"`（与 06 号票定的一致）。
2. 正文里**不留裸栅栏**；非 Rust 片段一律标语言。
3. 想展示 mdBook 指令本身时写 `\{{#…}}`。
4. 想展示「以 `#` 开头的内容」时不要放在 `rust` 块（用 `text` 或 `ignore`）。
5. 教学报错用 `compile_fail`；需要读者运行但不想被断言用 `ignore,mdbook-runnable`。
6. 需要行号的场景慎用：0.5 的 `hash-files` 默认 true，资源文件名带哈希（章节 HTML 文件名不变）。
7. CI：`mdbook test` 卡门禁；`mdbook build` 的日志要 grep `ERROR`；坏链交给 linkcheck2。

---

## 5. Main 追加问题的直接回答

**Q1：0.5.x 是否移除了 `use-unicode-segmentation`？中文搜索分词现状？**

答案：**这个字段在 0.4 和 0.5 都不存在**（不是「0.5 移除」）。证据：
- `[output.html.search]` 的合法字段全集（`crates/mdbook-core/src/config.rs` 的 `pub struct Search`，master）只有：`enable`、`limit-results`、`teaser-word-count`、`use-boolean-and`、`boost-title`、`boost-hierarchy`、`boost-paragraph`、`expand`、`heading-split-level`、`copy-js`、`chapter`。本机报错信息列出的白名单与之完全一致。
- 回溯检查 **v0.4.52** 的 `src/config.rs`：搜 `unicode` **零命中**；v0.4.52 的 `src/renderer/html_handlebars/search.rs` 与 master 的 tokenizer **完全相同**；v0.4.52 的官方文档里 `[output.html.search]` 字段表也**没有**该项。
- 再往前查 v0.4.0 / v0.3.7 的 `src/config.rs`：搜 `unicode` 同样零命中。
  （逐一核对过 HTTP 状态与体积，排除「拉取失败导致假阴性」：v0.3.7 28961 B、v0.4.0 34426 B、v0.4.52 45849 B、master `crates/mdbook-core/src/config.rs` 44795 B，均 HTTP 200，`grep -ci unicode` 全为 0。）
- CHANGELOG 全文搜 `katex|mathjax|mermaid|unicode|segmentation|tokeniz`：只有“IDs now use Unicode lowercase”与一条无关 fix，**没有**相关增删记录。
- master 的官方文档 `guide/src/format/configuration/renderers.md`：搜 `unicode` **0 命中**，`[output.html.search]` 示例字段表同样不含该项。
⇒ **判定：该字段从来不是 mdBook 的配置项**（很可能是和 elasticlunr / lunr 的配置记混了）。

中文（CJK）搜索分词现状（源码级）：
```rust
/// Tokenizes in the same way as elasticlunr-rs (for English), but also drops long tokens.
fn tokenize(text: &str) -> Vec<String> {
    text.split(|c: char| c.is_whitespace() || c == '-')
```
（`crates/mdbook-html/src/html_handlebars/search.rs`，master；v0.4.52 同）
即**只按空白与 `-` 切**，没有 CJK 分词。中文句子不含空格 ⇒ **整句（直到标点/空格）会变成一个 token**，读者搜「所有权」这类词几乎必然搜不到（除非整句完全相同）。**0.5.4 没有任何替代配置能改善**。
可缓解的办法（均为[建议/推断]，非引擎能力）：
- 章节标题（`heading-split-level`，默认 3）与短句里把术语**用空格或 `-` 隔开**，或让关键术语单独成词（标题分词对搜索加权最高：`boost-title` 默认 2）。
- 在正文里对关键术语给出**英文原词**（本项目本来就要「首现中英对照」），英文词能被正常分词检索。
- 真要中文分词，只能自己写 preprocessor/pre-render 往正文注入空格或改写 `searchindex.js`——**成本高，建议记为已知限制而非本期解决**。
- 相关已知问题可在上游搜 `rust-lang/mdBook` issues 里 “chinese / CJK search”（未在本票核实具体 issue 编号）。

**Q2：配置字段非法时打印 ERROR 却 exit 0——官方是否说明过？**

答案：**官方只说了「未知字段现在是错误」，没有承诺退出码**；实测该错误被吞掉，且连累整个 `[output.html]` 表：
- 文档/CHANGELOG：0.5 Migration Guide「Config changes」首条「**Unknown fields in config are now an error.**」（PR #2787、#2801）。<https://github.com/rust-lang/mdBook/blob/master/CHANGELOG.md>
- 实现（源码）：`Config::html_config()` 返回 `Option<HtmlConfig>`，`Err` 分支只 `log_backtrace(&e)` 然后 `None`；渲染器 `unwrap_or_default()` → **整个 `[output.html]` 退回默认值**（细节见 §3.2(b)）。
- 实测：`mdbook build` 打印 `ERROR Failed to deserialize 'output.html'`，仍完整重建 `book/`，**exit 0**；`mdbook test` 根本不检查 `[output.html]`（不跑 html 渲染器）。
⇒ 官方语义其实是「**日志级 error，不是致命错误**」；文档措辞容易让人误以为会 fail。**结论：这是需要在本项目 CI 里自行设防的坑（grep ERROR 日志），不能依赖退出码。** 另外，`[book]`/`[rust]`/`[build]` 等其他顶层表的同类错误是否也被吞掉，**本票未逐一实测**（[未验证]）。

---

## 6. 未验证 / 存疑清单

1. **MathJax 四种定界符的浏览器实际渲染结果**（`$`、`$$`、`\(`、`\[`）——只做了静态产物检查；文档称 `$$` 不支持。且 CDN（cdnjs）在中国大陆可达性未验证。
2. **`mdbook-linkcheck2` 的实际运行与退出码**——未安装（编译耗时）。安装方式、配置项、维护状态均已核实。
3. **`mdbook-mermaid` 实际渲染效果**——未安装。维护状态/安装方式/配置已核实。
4. **`[build] create-missing = false` 的确切退出码**。
5. **其他顶层表（`[book]`/`[rust]`/`[build]`/`[preprocessor.*]`）的未知字段是否同样 exit 0**（`[output.html]` 已确认）。
6. 文档站（跟随 master）与本机 0.5.4 的字段差异未逐条 diff；本文所有「实测」项已在本机 0.5.4 上跑过。
7. 上游是否已有 CJK 搜索的 issue/PR 编号未核实。

---

## 7. 复现步骤（临时样例已删除）

```sh
mkdir -p tmp-mdbook/src && cd tmp-mdbook
printf '[book]\ntitle = "t"\nlanguage = "zh"\n\n[rust]\nedition = "2024"\n\n[output.html]\n' > book.toml
# src/SUMMARY.md 列出各章；各章内容见「实测」段落的源码块
mdbook test                      # 全绿 → exit 0；有失败 → exit 101
mdbook test -c ch3-playground.md
mdbook test -c ch6-companion.md -L companion/target/debug/deps
mdbook build                     # 输出 book/；开 [output.markdown] 后变 book/html + book/markdown
mdbook serve -p 3111             # 起后台，验证 http 200 后收掉
```

实验覆盖清单（均已在本机 0.5.4 跑过）：隐藏行 `#`/`##` 转义、`ignore`/`no_run`/`should_panic`/`compile_fail`/`editable`/`noplayground`/`mdbook-runnable`/`edition20xx`、无语言栅栏、`{{#playground}}`/`{{#include}}`/`{{#rustdoc_include}}` 与转义、跨目录 include、无 `fn main` 的自动包裹、`.playground` 与按钮隐藏逻辑、admonitions、`$`/`$$`/`\(`/`\[` 数学、` ```mermaid `、内嵌 SVG、SUMMARY 外章节不被测、缺 include、非法配置字段、`[rust] edition` 2015/2024 对比、外部 crate 的 `-L` 三种写法、`RUSTDOCFLAGS` 与 `--` 透传、`mdbook init --theme`、`[output.markdown]` 调试产物。
