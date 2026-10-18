pub mod items;

use self::items::Product;

pub struct Catalog {
    products: Vec<Product>,
}

impl Catalog {
    pub fn new() -> Self {
        Self { products: Vec::new() }
    }

    pub fn add(&mut self, product: Product) {
        self.products.push(product);
    }

    pub fn len(&self) -> usize {
        self.products.len()
    }

    pub fn products(&self) -> &[Product] {
        &self.products
    }

    pub fn total(&self) -> u32 {
        self.products.iter().map(|p| p.price).sum()
    }
}
