use crate::core::write::metadata::{
    fill::fill_config::FillConfig, write_sheet::WriteSheet, write_table::WriteTable,
};

pub trait ExcelBuilder {
    fn add_content<T>(&mut self, data: T, write_sheet: WriteSheet, write_table: WriteTable)
    where
        T: Iterator<Item = String>;

    fn fill(&mut self, data: String, fill_config: FillConfig, write_sheet: WriteSheet);
}
