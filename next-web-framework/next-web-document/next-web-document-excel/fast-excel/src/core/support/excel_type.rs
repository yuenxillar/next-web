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
    // pub fn extension(&self) -> &str {
    //     match self {
    //         ExcelType::Xls => "xls",
    //         ExcelType::Xlsx => "xlsx",
    //     }
    // }

    // pub fn header_bytes(&self) -> Vec<u8> {
    //     match self {
    //         ExcelType::Xls => XLS_HEADER.to_vec(),
    //         ExcelType::Xlsx => XLSX_HEADER.to_vec(),
    //     }
    // }

    pub fn from_meta_data(meta_data: &ReadMetaData) -> Result<Self, ExcelTypeError> {
        match meta_data.extension.as_str() {
            "xlsx" => {
                if meta_data.header == XLSX_HEADER.to_vec() {
                    Ok(ExcelType::Xlsx)
                } else {
                    Err(ExcelTypeError::HeaderBytesMismatch)
                }
            }
            "xls" => {
                if meta_data.header == XLS_HEADER.to_vec() {
                    Ok(ExcelType::Xls)
                } else {
                    Err(ExcelTypeError::HeaderBytesMismatch)
                }
            }
            _ => Err(ExcelTypeError::UnsupportedType),
        }
    }

    pub fn index(extension: &str) -> usize {
        match extension {
            "xls" => XLS_HEADER.len(),
            "xlsx" => XLSX_HEADER.len(),
            _ => 0,
        }
    }
}

#[derive(Debug, Error)]
pub enum ExcelTypeError {
    #[error("This file is an unsupported type")]
    UnsupportedType,
    #[error("The header bytes of this file do not match")]
    HeaderBytesMismatch,
}
