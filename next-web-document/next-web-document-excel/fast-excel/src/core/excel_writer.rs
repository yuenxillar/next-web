use next_web_core::error::BoxError;

use crate::core::{
    Closeable,
    context::write_context::WriteContext,
    write::{
        excel_builder::ExcelBuilder,
        excel_builder_impl::ExcelBuilderImpl,
        metadata::{
            fill::fill_config::FillConfig, write_sheet::WriteSheet, write_table::WriteTable,
            write_workbook::WriteWorkbook,
        },
    },
};

pub struct ExcelWriter<E = ExcelBuilderImpl> {
    excel_builder: E,
}

impl<E> ExcelWriter<E>
where
    E: ExcelBuilder,
{
    pub fn write_data<T>(&mut self, data: T, write_sheet: WriteSheet) -> &mut Self
    where
        T: AsRef<[u8]>,
    {
        self.excel_builder.add_content(data, write_sheet, None);
        self
    }

    pub fn write_data_with_table<T>(
        &mut self,
        data: T,
        write_sheet: WriteSheet,
        write_table: WriteTable,
    ) -> &mut Self
    where
        T: AsRef<[u8]>,
    {
        self.excel_builder
            .add_content(data, write_sheet, Some(write_table));
        self
    }

    pub fn fill<T>(&mut self, data: T, write_sheet: WriteSheet) -> &mut Self
    where
        T: AsRef<[u8]>,
    {
        self.excel_builder.fill(data, None, write_sheet);
        self
    }

    pub fn fill_with_config<T>(
        &mut self,
        data: T,
        fill_config: FillConfig,
        write_sheet: WriteSheet,
    ) -> &mut Self
    where
        T: AsRef<[u8]>,
    {
        self.excel_builder
            .fill(data, Some(fill_config), write_sheet);

        self
    }

    pub fn finish(&mut self) {
        self.excel_builder.finish(false);
    }

    pub fn write_context<T: WriteContext>(&self) -> T {
        self.excel_builder.write_context()
    }
}

impl ExcelWriter {
    pub fn new(write_workbook: WriteWorkbook) -> Self {
        let excel_builder = ExcelBuilderImpl::new(write_workbook);
        Self { excel_builder }
    }
}

impl<T: ExcelBuilder> Closeable for ExcelWriter<T> {
    fn close(&mut self) -> Result<(), BoxError> {
        self.finish();
        Ok(())
    }
}
