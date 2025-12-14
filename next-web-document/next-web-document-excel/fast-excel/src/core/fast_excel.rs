use crate::core::fast_excel_factory::FastExcelFactory;

pub struct FastExcel;

impl FastExcelFactory for FastExcel {}

#[cfg(test)]
mod fast_excel_tests {
    use super::*;

    #[test]
    fn test_write_file() {}
}
