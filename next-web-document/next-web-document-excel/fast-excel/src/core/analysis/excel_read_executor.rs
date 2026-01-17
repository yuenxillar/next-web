use next_web_core::error::BoxError;

use crate::core::read::metadata::read_sheet::ReadSheet;

pub trait ExcelReadExecutor<T> {
    fn sheet_list(&self) -> Vec<&ReadSheet<T>>;
    fn execute(&self) -> Result<(), BoxError>;
}
