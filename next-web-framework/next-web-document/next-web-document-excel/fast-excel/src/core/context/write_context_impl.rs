use next_web_core::error::BoxError;
use tracing::debug;

use crate::core::{
    context::write_context::WriteContext,
    enums::write_type::WriteType,
    util::{work_book_util::WorkBookUtil, write_handler_utils::WriteHandlerUtils},
    write::metadata::{
        holder::{
            write_holder::WriteHolder, write_sheet_holder::WriteSheetHolder,
            write_table_holder::WriteTableHolder, write_workbook_holder::WriteWorkbookHolder,
        },
        write_sheet::WriteSheet,
        write_table::WriteTable,
        write_workbook::WriteWorkbook,
    },
};

pub struct WriteContextImpl {
    write_workbook_holder: WriteWorkbookHolder,
    write_sheet_holder: Option<WriteSheetHolder>,
    write_table_holder: Option<WriteTableHolder>,
    write_holder: u8,
    finished: bool,
}

impl WriteContextImpl {
    const NO_SHEETS: &'static str = "no sheets";

    pub fn new(write_workbook: WriteWorkbook) -> Result<Self, BoxError> {
        debug!("Begin to Initialization 'WriteContextImpl'");

        let write_workbook_holder = WriteWorkbookHolder::new(write_workbook);
        debug!("CurrentConfiguration is writeWorkbookHolder");
        let mut ctx = WriteContextImpl {
            write_workbook_holder,
            write_sheet_holder: None,
            write_table_holder: None,
            write_holder: 0,
            finished: false,
        };

        let workbook_write_handler_context =
            WriteHandlerUtils::create_workbook_write_handler_context(write_context);
        WriteHandlerUtils::before_workbook_create();

        match WorkBookUtil::create_work_book(&mut ctx.write_workbook_holder) {
            Ok(_) => {}
            Err(e) => return Err(format!("Failed to create workbook: {}", e.to_string()).into()),
        };
        WriteHandlerUtils::after_workbook_create(workbook_write_handler_context, false);
        debug!("Initialization 'WriteContextImpl' complete");

        Ok(ctx)
    }
}

impl WriteContext for WriteContextImpl {
    fn current_sheet(&self, write_sheet: WriteSheet, write_type: WriteType) {
        todo!()
    }

    fn current_table(&self, write_table: WriteTable) {
        todo!()
    }

    fn write_workbook_holder(&self) -> &WriteWorkbookHolder {
        todo!()
    }

    fn write_sheet_holder(&self) -> &WriteSheetHolder {
        todo!()
    }

    fn write_table_holder(&self) -> &WriteTableHolder {
        todo!()
    }

    fn current_write_holder<T: WriteHolder>(&self) -> &T {
        todo!()
    }

    fn finish(&mut self, on_rror: bool) {
        todo!()
    }
}
