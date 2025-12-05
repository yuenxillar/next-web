use crate::core::{
    context::write_context::WriteContext,
    write::handler::context::workbook_write_handler_context::WorkbookWriteHandlerContext,
};

pub struct WriteHandlerUtils;

impl WriteHandlerUtils {
    pub fn create_workbook_write_handler_context<T: WriteContext>(
        write_context: &mut T,
    ) -> WorkbookWriteHandlerContext {
        let context = WorkbookWriteHandlerContext::new(todo!());

        write_context
            .write_workbook_holder()
            .set_workbook_write_handler_context(context);
        context
    }

    pub fn before_workbook_create() {}

    pub fn after_workbook_create(context: WorkbookWriteHandlerContext, run_own: bool) {}
}
