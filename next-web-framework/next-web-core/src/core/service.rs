use crate::core::singleton::Singleton;

pub trait Service: Singleton {}

pub trait TestBBH {
    fn path(&self) -> String {
        std::env::var("CARGO_PKG_NAME").unwrap()
    }
}
