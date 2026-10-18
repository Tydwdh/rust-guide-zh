//! 第 39 章的探针 crate：过程宏的组合形态与叠加顺序。
//!
//! 不引任何外部依赖（没有 syn/quote）：输入是 `TokenStream`，需要的那点结构自己认。
//! 每个宏都做两件事：
//!   1. 把「我看到了什么」打到编译日志（`eprintln!` 走编译器进程的 stderr，cargo 会显示）；
//!   2. 在输出里塞一个标记常量，运行期也能验证它跑过。

use proc_macro::{TokenStream, TokenTree};

/// 把 token 流拼成一行，纯粹为了看输入。
fn flatten(stream: TokenStream) -> String {
    stream
        .into_iter()
        .map(|tt: TokenTree| tt.to_string())
        .collect::<Vec<_>>()
        .join(" ")
}

/// 属性宏 `#[tag(NAME)]`：留一个 `const NAME: &str`，其余原样放行。
/// 标记名由调用方给，所以同一个 item 上叠两个 `#[tag(…)]` 也不会撞名。
#[proc_macro_attribute]
pub fn tag(attr: TokenStream, item: TokenStream) -> TokenStream {
    eprintln!("[tag]   attr = <{}>", flatten(attr.clone()));
    eprintln!("[tag]   item = <{}>", flatten(item.clone()));
    let name = flatten(attr).replace(' ', ""); // 探针：只取「标识符」这一段
    let mut out: TokenStream = format!("const {name}: &str = \"{name}\";").parse().unwrap();
    out.extend(item);
    out
}

/// derive 宏：给结构体留一个 `const SNIFFED_<名字>: u8`。
#[proc_macro_derive(Sniff)]
pub fn derive_sniff(input: TokenStream) -> TokenStream {
    eprintln!("[Sniff] input = <{}>", flatten(input.clone()));
    let name = struct_name(input);
    format!("const SNIFFED_{}: u8 = 3;", name.to_uppercase())
        .parse()
        .unwrap()
}

/// 粗略找出 `struct` 后面那个标识符（探针够用，不处理泛型与元组结构体）。
fn struct_name(input: TokenStream) -> String {
    let mut seen_struct = false;
    for tt in input {
        if let TokenTree::Ident(id) = tt {
            let text = id.to_string();
            if seen_struct {
                return text;
            }
            if text == "struct" {
                seen_struct = true;
            }
        }
    }
    panic!("Sniff 只支持具名结构体");
}

/// 函数式宏：它的输出本身就是一条**属性宏调用**。
#[proc_macro]
pub fn make_item(_: TokenStream) -> TokenStream {
    "#[tag(FROM_FN_MACRO)] const MADE_BY_FN_MACRO: u8 = 2;"
        .parse()
        .unwrap()
}
