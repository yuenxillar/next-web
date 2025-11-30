use next_web_core::error::BoxError;

use crate::core::write::metadata::holder::write_workbook_holder::WriteWorkbookHolder;

pub struct WorkBookUtil;

impl WorkBookUtil {
    pub fn create_work_book(
        write_workbook_holder: &mut WriteWorkbookHolder,
    ) -> Result<(), BoxError> {
        Ok(())
    }
}
