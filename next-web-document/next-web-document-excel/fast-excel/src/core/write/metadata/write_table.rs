use std::ops::{Deref, DerefMut};

use next_web_core::traits::required::Required;

use crate::core::{
    metadata::basic_parameter::BasicParameter,
    write::metadata::write_basic_parameter::WriteBasicParameter,
};

pub struct WriteTable {
    pub table_no: Option<u32>,

    write_basic_parameter: WriteBasicParameter,
}

impl WriteTable {
    pub fn get_table_no(&self) -> Option<u32> {
        self.table_no
    }

    pub fn set_table_no(&mut self, table_no: u32) {
        self.table_no = Some(table_no);
    }
}

impl Required<BasicParameter> for WriteTable {
    fn get_object(&self) -> &BasicParameter {
        &self.write_basic_parameter.basic_parameter
    }

    fn get_mut_object(&mut self) -> &mut BasicParameter {
        &mut self.write_basic_parameter.basic_parameter
    }
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

impl Default for WriteTable {
    fn default() -> Self {
        Self {
            table_no: None,
            write_basic_parameter: Default::default(),
        }
    }
}
