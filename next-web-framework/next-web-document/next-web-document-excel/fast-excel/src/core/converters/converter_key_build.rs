use std::{any::TypeId, sync::OnceLock};

use crate::core::enums::cell_data_type::CellDataType;

type TypeMap = std::collections::HashMap<TypeId, TypeId>;
pub static BOXING_MAP: OnceLock<TypeMap> = OnceLock::new();

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConverterKey {
    pub id: TypeId,
    pub cell_data_type: CellDataType,
}
