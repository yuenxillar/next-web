use std::ops::{Deref, DerefMut};

use crate::core::write::metadata::write_basic_parameter::WriteBasicParameter;

pub struct WriteTable {
    pub table_no: u32,

    write_basic_parameter: WriteBasicParameter,
}

impl Deref for WriteTable {
    type Target = WriteBasicParameter;

    fn deref(&self) -> &Self::Target {
        &self.write_basic_parameter
    }
}

impl DerefMut for WriteTable {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.write_basic_parameter
    }
}
