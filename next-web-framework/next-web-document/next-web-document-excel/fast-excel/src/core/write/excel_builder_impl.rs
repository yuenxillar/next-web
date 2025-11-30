use crate::core::{
    context::{write_context::WriteContext, write_context_impl::WriteContextImpl},
    write::{
        excel_builder::ExcelBuilder,
        executor::{
            excel_write_add_executor::ExcelWriteAddExecutor,
            excel_write_fill_executor::ExcelWriteFillExecutor,
        },
        metadata::{
            fill::fill_config::FillConfig, write_sheet::WriteSheet, write_table::WriteTable,
            write_workbook::WriteWorkbook,
        },
    },
};

pub struct ExcelBuilderImpl<C = WriteContextImpl> {
    context: C,
    excel_write_fill_executor: Option<ExcelWriteFillExecutor>,
    excel_write_add_executor: Option<ExcelWriteAddExecutor>,
}

impl ExcelBuilderImpl {
    pub fn new(write_workbook: WriteWorkbook) -> Self {
        ExcelBuilderImpl {
            context: WriteContextImpl::new(write_workbook),
            excel_write_fill_executor: None,
            excel_write_add_executor: None,
        }
    }
}

impl<C> ExcelBuilder for ExcelBuilderImpl<C>
where
    C: WriteContext,
{
    fn add_content<T>(&mut self, data: T, write_sheet: WriteSheet, write_table: Option<WriteTable>)
    where
        T: AsRef<[u8]>,
    {
    }

    fn add_content_with_data<T>(
        &mut self,
        data: T,
        write_sheet: WriteSheet,
        write_table: Option<WriteTable>,
    ) where
        T: Iterator<Item = String>,
    {
    }

    fn fill<T>(&mut self, data: T, fill_config: Option<FillConfig>, write_sheet: WriteSheet)
    where
        T: AsRef<[u8]>,
    {
    }

    fn write_context<T>(&self) -> T
    where
        T: WriteContext,
    {
        todo!()
    }

    fn finish(&mut self, on_error: bool) {
        todo!()
    }
}
