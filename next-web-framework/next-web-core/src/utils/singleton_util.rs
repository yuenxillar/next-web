use crate::core::singleton::Singleton;

pub fn find_group_singleton(singleton: impl Singleton) -> Option<i32>
{
    let pkg_name = std::env::var("CARGO_PKG_NAME").unwrap();
    
    None
}
