use crate::core::write::{
    handler::context::workbook_write_handler_context::WorkbookWriteHandlerContext,
    metadata::write_workbook::WriteWorkbook,
};

#[cfg(feature = "async")]
use tokio::io::AsyncRead as Read;

pub struct WriteWorkbookHolder {}

impl WriteWorkbookHolder {
    pub fn new(write_workbook: WriteWorkbook) -> Self {
        WriteWorkbookHolder {}
    }

    pub fn set_workbook_write_handler_context(&mut self, context: WorkbookWriteHandlerContext) {
        // self.context = context;
    }

    pub fn get_temp_template_input_stream(&self) -> Option<&dyn Read> {
        None
    }
}
