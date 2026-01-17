use crate::core::write::property::excel_write_head_property::ExcelWriteHeadProperty;

pub trait WriteHolder {
    fn relative_head_row_index(&self) -> u32;

    fn excel_write_head_property(&self) -> &ExcelWriteHeadProperty;
}
