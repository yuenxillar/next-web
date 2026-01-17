use std::{any::Any, collections::HashMap};

pub enum RowData {
    List(Vec<i32>),
    Map(HashMap<u32, String>),
    Object(Box<dyn Any>),
}

impl RowData {
    pub fn is_collection(&self) -> bool {
        match self {
            RowData::List(_) => true,
            RowData::Map(_) => true,
            RowData::Object(_) => false,
        }
    }

    pub fn is_object(&self) -> bool {
        !self.is_collection()
    }

    pub fn is_empty(&self) -> bool {
        match self {
            RowData::List(list) => list.is_empty(),
            RowData::Map(map) => map.is_empty(),
            RowData::Object(_) => false,
        }
    }

    pub fn size(&self) -> usize {
        match self {
            RowData::List(list) => list.len(),
            RowData::Map(map) => map.len(),
            RowData::Object(_) => 0,
        }
    }
}
