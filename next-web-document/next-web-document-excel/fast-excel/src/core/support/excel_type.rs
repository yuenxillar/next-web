use std::{io::Read, path::Path};

use crate::core::read::metadata::read_workbook::ReadWorkbook;
use next_web_core::error::BoxError;

pub const XLS_HEADER: [u8; 8] = [208, 207, 17, 224, 161, 177, 26, 225];
pub const XLSX_HEADER: [u8; 4] = [80, 75, 3, 4];
pub const CSV_HEADER: [u8; 4] = [229, 167, 147, 229];

pub const XLS_EXTENSION: &str = "xls";
pub const XLSX_EXTENSION: &str = "xlsx";
pub const CSV_EXTENSION: &str = "csv";

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ExcelType {
    /// XLS
    Xls,

    /// XLSX
    Xlsx,

    /// CSV
    Csv,
}

impl ExcelType {
    pub fn from_read_workbook(read_workbook: &ReadWorkbook) -> Result<Self, BoxError> {
        match read_workbook.get_excel_type() {
            Some(excel_type) => Ok(excel_type.clone()),
            None => {
                let path = match read_workbook.get_path() {
                    Some(path) => Path::new(path),
                    None => return Err("File mest be a no none".into()),
                };

                if !path.exists() {
                    return Err(format!("File {:?} not exists.", path.to_str()).into());
                }

                // Use the name to determine the type
                let file_name = path
                    .file_name()
                    .map(|s| s.to_str().unwrap_or_default())
                    .unwrap_or_default();
                if file_name.ends_with(XLSX_EXTENSION) {
                    return Ok(ExcelType::Xlsx);
                } else if file_name.ends_with(XLS_EXTENSION) {
                    return Ok(ExcelType::Xls);
                } else if file_name.ends_with(CSV_EXTENSION) {
                    return Ok(ExcelType::Csv);
                }

                let mut file = match std::fs::File::open(path) {
                    Ok(file) => file,
                    Err(err) => return Err(format!("Failed to open file: {}", err).into()),
                };

                let mut buf = [0; 8];
                file.read_exact(&mut buf)?;
                return Ok(Self::matches(&buf));
            }
        }
    }

    fn matches(data: &[u8]) -> Self {
        if Self::find_magic(&XLSX_HEADER, data) {
            Self::Xls
        } else if Self::find_magic(&XLS_HEADER, data) {
            Self::Xlsx
        } else {
            Self::Csv
        }
    }

    pub fn find_magic(expected: &[u8], actual: &[u8]) -> bool {
        for (index, expected_byte) in expected.iter().enumerate() {
            if &actual[index] != expected_byte && expected_byte != &b'?' {
                return false;
            }
        }
        true
    }
}
