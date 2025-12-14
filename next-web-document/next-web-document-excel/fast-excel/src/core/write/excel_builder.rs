use crate::core::{
    context::write_context::WriteContext,
    write::metadata::{
        fill::fill_config::FillConfig, write_sheet::WriteSheet, write_table::WriteTable,
    },
};

pub trait ExcelBuilder {
    fn add_content<T>(&mut self, data: T, write_sheet: WriteSheet, write_table: Option<WriteTable>)
    where
        T: AsRef<[u8]>;

    fn add_content_with_data<T>(
        &mut self,
        data: T,
        write_sheet: WriteSheet,
        write_table: Option<WriteTable>,
    ) where
        T: Iterator<Item = String>;

    fn fill<T>(&mut self, data: T, fill_config: Option<FillConfig>, write_sheet: WriteSheet)
    where
        T: AsRef<[u8]>;

    fn write_context<T>(&self) -> T
    where
        T: WriteContext;

    fn finish(&mut self, on_error: bool);
}
