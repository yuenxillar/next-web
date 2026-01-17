use crate::core::{
    context::write_context::WriteContext,
    error::excel_error::ExcelError,
    write::handler::context::{
        row_write_handler_context::RowWriteHandlerContext,
        workbook_write_handler_context::WorkbookWriteHandlerContext,
    },
};

pub struct WriteHandlerUtils;

impl WriteHandlerUtils {
    pub fn create_workbook_write_handler_context<T: WriteContext>(
        write_context: &mut T,
    ) -> WorkbookWriteHandlerContext {
        // let context = WorkbookWriteHandlerContext::new(todo!());

        // write_context
        //     .write_workbook_holder()
        //     .set_workbook_write_handler_context(context);
        // context
        todo!()
    }

    pub fn before_workbook_create() {}

    pub fn after_workbook_create(context: WorkbookWriteHandlerContext, run_own: bool) {}

    pub fn create_row_write_handler_context(
        write_context: &dyn WriteContext,
        row_index: u32,
        relative_row_index: u32,
        is_header: bool,
    ) -> RowWriteHandlerContext {
        todo!()
    }

    pub fn before_row_create(context: &mut RowWriteHandlerContext) -> Result<(), ExcelError> {
        todo!()
    }

    pub fn after_row_create(context: &mut RowWriteHandlerContext) {}

    pub fn after_row_dispose(context: &mut RowWriteHandlerContext) {}
}
