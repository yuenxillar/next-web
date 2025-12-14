use crate::common::group_name::GroupName;

pub trait Group
where
    Self: Send + Sync,
{
    fn group(&self) -> GroupName {
        GroupName {
            name: std::env::var("CARGO_PKG_NAME").unwrap_or(String::from("unknown")),
            type_name: std::any::type_name::<Self>(),
        }
    }
}