pub struct Product {
    pub name: String,
    pub price: u32,
}

impl Product {
    pub fn new(name: &str, price: u32) -> Self {
        Self { name: name.to_owned(), price }
    }
}
