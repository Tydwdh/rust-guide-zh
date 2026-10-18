//! 消费 `ch39-attr-demo` 的探针：看属性宏的叠加顺序与组合形态。
//!
//! 编译这份代码时，cargo 会打印各个宏的 `eprintln!`，那就是顺序证据。

use ch39_attr_demo::{Sniff, make_item, tag};

// ① 同一个属性宏叠两层：谁先展开、后者的 item 里还剩什么？
#[tag(OUTER_SAW)]
#[tag(INNER_SAW)]
const STACKED: u8 = 1;

// ② 属性宏与 derive 混排：属性宏在 derive 之前
#[tag(RECORD_TAG)]
#[derive(Sniff)]
struct Record {
    id: u32,
}

// ③ 反过来：derive 在属性宏之前
#[derive(Sniff)]
#[tag(LATE_TAG)]
struct Late {
    id: u32,
}

// ④ 函数式宏的输出里带一条属性宏调用
make_item!();

// ⑤ 声明宏展开出属性宏调用
macro_rules! gen_item {
    () => {
        #[tag(FROM_RULES)]
        const RULES_ITEM: u8 = 4;
    };
}
gen_item!();

fn main() {
    println!("STACKED            = {STACKED}");
    println!("OUTER_SAW          = {OUTER_SAW}");
    println!("INNER_SAW          = {INNER_SAW}");
    println!("RECORD_TAG         = {RECORD_TAG}");
    println!("LATE_TAG           = {LATE_TAG}");
    println!("SNIFFED_RECORD     = {SNIFFED_RECORD}");
    println!("SNIFFED_LATE       = {SNIFFED_LATE}");
    println!("MADE_BY_FN_MACRO   = {MADE_BY_FN_MACRO}");
    println!("FROM_FN_MACRO      = {FROM_FN_MACRO}");
    println!("RULES_ITEM         = {RULES_ITEM}");
    println!("FROM_RULES         = {FROM_RULES}");
    println!("Record.id          = {}", Record { id: 7 }.id);
    println!("Late.id            = {}", Late { id: 8 }.id);
}

#[cfg(test)]
mod tests {
    // 这些断言能编译过，就说明各条属性宏调用（包括宏生成的）都被展开了。
    #[test]
    fn markers_exist() {
        assert_eq!(super::OUTER_SAW, "OUTER_SAW");
        assert_eq!(super::INNER_SAW, "INNER_SAW");
        assert_eq!(super::SNIFFED_RECORD, 3);
        assert_eq!(super::FROM_FN_MACRO, "FROM_FN_MACRO");
        assert_eq!(super::FROM_RULES, "FROM_RULES");
    }
}
