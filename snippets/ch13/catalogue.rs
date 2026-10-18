use std::sync::Arc;

trait Animal {
    fn name(&self) -> String;
    fn kind(&self) -> &'static str;
}

// supertrait：实现 Pet 的类型必须同时实现 Animal。
trait Pet: Animal {
    fn owner(&self) -> String;
}

struct Dog;

impl Animal for Dog {
    fn name(&self) -> String {
        String::from("dog")
    }
    fn kind(&self) -> &'static str {
        "犬科"
    }
}

impl Pet for Dog {
    fn owner(&self) -> String {
        String::from("小明")
    }
}

fn main() {
    // dyn Pet 的 vtable 里同时带着 Pet 与 supertrait Animal 的方法。
    let pet: Box<dyn Pet> = Box::new(Dog);
    println!("{} 的主人是 {}", pet.name(), pet.owner());

    // 向上转型：dyn Pet -> dyn Animal（引用、Box、Arc、Rc 都支持）。
    let animal: Box<dyn Animal> = pet;
    println!("{} 属于 {}", animal.name(), animal.kind());

    // 借用形式：不加 as 也会隐式强转。
    let dog = Dog;
    let pet_ref: &dyn Pet = &dog;
    let animal_ref: &dyn Animal = pet_ref;
    println!("{}", animal_ref.name());

    let arc: Arc<dyn Pet> = Arc::new(Dog);
    let arc_animal: Arc<dyn Animal> = arc;
    println!("Arc 向上转型后：{}", arc_animal.name());

    // 顺带丢掉 auto trait：dyn Pet + Send -> dyn Animal。
    let boxed: Box<dyn Pet + Send> = Box::new(Dog);
    let plain: Box<dyn Animal> = boxed;
    println!("丢掉 Send 之后：{}", plain.name());
}
