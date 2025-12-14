use crate::core::{
    context::write_context::WriteContext,
    write::metadata::holder::write_workbook_holder::WriteWorkbookHolder,
};

pub struct WorkbookWriteHandlerContext {
    // write_context: WriteContext,
    write_workbook_holder: WriteWorkbookHolder,
}
