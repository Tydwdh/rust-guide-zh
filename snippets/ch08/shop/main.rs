mod catalog;

use catalog::items::Product;

fn main() {
    let mut shop = catalog::Catalog::new();
    shop.add(Product::new("键盘", 199));
    shop.add(Product::new("鼠标", 89));

    println!("{} 件商品，合计 {} 元", shop.len(), shop.total());
    for p in shop.products() {
        println!("- {}：{} 元", p.name, p.price);
    }
}
