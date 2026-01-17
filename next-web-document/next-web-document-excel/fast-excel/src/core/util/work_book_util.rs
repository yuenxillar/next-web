use next_web_core::error::BoxError;

use crate::core::{
    poi::ss::usermodel::{row::Row, sheet::Sheet},
    write::metadata::holder::write_workbook_holder::WriteWorkbookHolder,
};

pub struct WorkBookUtil;

impl WorkBookUtil {
    pub fn create_work_book(
        write_workbook_holder: &mut WriteWorkbookHolder,
    ) -> Result<(), BoxError> {
        Ok(())
    }

    pub fn create_row(sheet: &mut dyn Sheet, row_index: u32) -> Box<dyn Row> {
        sheet.create_row(row_index)
    }
}
