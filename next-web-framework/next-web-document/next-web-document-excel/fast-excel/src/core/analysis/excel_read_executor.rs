use next_web_core::error::BoxError;

use crate::core::read::metadata::read_sheet::ReadSheet;

pub trait ExcelReadExecutor {
    fn sheet_list(&self) -> Vec<&ReadSheet>;
    fn execute(&self) -> Result<(), BoxError>;
}
