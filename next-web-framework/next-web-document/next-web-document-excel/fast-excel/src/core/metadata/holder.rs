use crate::core::enums::holder_type::HolderType;

pub trait Holder {
    fn holder_type(&self) -> HolderType;
}
