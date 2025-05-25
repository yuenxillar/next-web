use thiserror::Error;

use super::read::meta_data::ReadMetaData;

pub const XLS_HEADER: [u8; 8] = [208, 207, 17, 224, 161, 177, 26, 225];
pub const XLSX_HEADER: [u8; 4] = [80, 75, 3, 4];

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ExcelType {
    /// XLS  
    /// header bytes: -48, -49, 17, -32, -95, -79, 26, -31
    Xls,

    /// XLSX
    /// header bytes: 80, 75, 3, 4
    Xlsx,
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

    pub fn from_meta_data(meta_data: &ReadMetaData) -> Result<Self, ExcelTypeError>  {
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