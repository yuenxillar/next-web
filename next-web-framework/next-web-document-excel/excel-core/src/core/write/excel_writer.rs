use std::{
    io::{Seek, Write},
    path::Path,
};

use rust_xlsxwriter::*;
use umya_spreadsheet::Worksheet;

pub struct ExcelWriter {
    workbook: Workbook,
    worksheet: Vec<String>,
}

impl ExcelWriter {
    pub fn new() -> Self {
        Self {
            workbook: Workbook::new(),
            worksheet: Vec::new(),
        }
    }

    pub fn write<T>(&mut self, sheet: impl AsRef<str>, data: &Vec<T>)
    where
        T: serde::Serialize + XlsxSerialize,
    {
        self.worksheet
            .iter_mut()
            .find(|item| item.as_str().eq(sheet.as_ref()))
            .map(|name| {
                let worksheet = self.workbook.worksheet_from_name(&name).unwrap();
                worksheet.set_serialize_headers::<T>(0, 0).unwrap();
                for item in data.iter() {
                    worksheet.serialize(item).unwrap();
                }
            })
            .or_else(|| {
                let worksheet = self.workbook.add_worksheet();
                worksheet.set_name(sheet.as_ref()).unwrap();

                worksheet.set_serialize_headers::<T>(0, 0).unwrap();
                for item in data.iter() {
                    worksheet.serialize(item).unwrap();
                }

                self.worksheet.push(sheet.as_ref().to_owned());
                None
            });
    }

    pub fn save(&mut self, path: impl AsRef<Path>) -> Result<(), XlsxError> {
        self.workbook.save(path)?;
        Ok(())
    }

    pub fn save_to_buffer(&mut self) -> Result<Vec<u8>, XlsxError> {
        self.workbook.save_to_buffer()
    }

    pub fn save_to_writer<W>(&mut self, writer: W) -> Result<(), XlsxError>
    where
        W: Write + Seek + Send,
    {
        self.workbook.save_to_writer(writer)
    }
}

fn test() {
    
}

// let mut cursor: Cursor<Vec<u8>> = Cursor::new(Vec::new());
//         self.workbook.save_to_writer(&mut cursor).unwrap();

//         let writer = SinkWriter::new(cursor);
//         Ok(writer)
