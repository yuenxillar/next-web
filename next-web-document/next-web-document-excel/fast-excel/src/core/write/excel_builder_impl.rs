use crate::core::{
    context::{write_context::WriteContext, write_context_impl::WriteContextImpl},
    enums::write_type::WriteType,
    error::excel_error::ExcelError,
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
    excel_write_add_executor: Option<ExcelWriteAddExecutor<C>>,
}

impl ExcelBuilderImpl {
    pub fn new(write_workbook: WriteWorkbook) -> Result<Self, ExcelError> {
        Ok(ExcelBuilderImpl {
            context: WriteContextImpl::new(write_workbook)?,
            excel_write_fill_executor: None,
            excel_write_add_executor: None,
        })
    }
}

impl<C> ExcelBuilder for ExcelBuilderImpl<C>
where
    C: WriteContext,
    C: Clone,
{
    fn add_content<T>(
        &mut self,
        data: &[T],
        write_sheet: WriteSheet,
        write_table: Option<WriteTable>,
    ) -> Result<(), ExcelError>
    where
        T: AsRef<[u8]>,
    {
        let block: Result<(), ExcelError> = {
            self.context.current_sheet(write_sheet, WriteType::Add)?;
            if let Some(write_table) = write_table {
                self.context.current_table(write_table)?;
            }

            self.excel_write_add_executor
                .get_or_insert(ExcelWriteAddExecutor::new(self.context.clone()))
                .add(data)?;

            Ok(())
        };

        if let Err(err) = block {
            self.finish(true);
            return Err(err);
        }

        Ok(())
    }

    fn add_content_with_data<T>(
        &mut self,
        data: T,
        write_sheet: WriteSheet,
        write_table: Option<WriteTable>,
    ) -> Result<(), ExcelError>
    where
        T: Iterator<Item = String>,
    {
        Ok(())
    }

    fn fill<T>(
        &mut self,
        data: T,
        fill_config: Option<FillConfig>,
        write_sheet: WriteSheet,
    ) -> Result<(), ExcelError>
    where
        T: AsRef<[u8]>,
    {
        Ok(())
    }

    fn write_context<T>(&self) -> T
    where
        T: WriteContext,
    {
        todo!()
    }

    fn finish(&mut self, on_error: bool) {
        self.context.finish(on_error);
    }
}
