use std::{
    fs::File,
    io::{BufReader, Cursor, Read, Seek},
    ops::Range,
    path::Path,
};

use calamine::{
    Error, HeaderRow, RangeDeserializerBuilder, Reader, Sheets, open_workbook_auto,
    open_workbook_from_rs,
};

use crate::core::excel_type::ExcelType;

use super::meta_data::ReadMetaData;

pub struct ExcelReader<RS> {
    sheet_names: Vec<String>,
    read_meta_data: Option<ReadMetaData>,
    workbook: Option<Sheets<RS>>,
}

impl<RS> ExcelReader<RS>
where
    RS: Read + Seek,
{
    pub fn meta_data(&self) -> Option<&ReadMetaData> {
        self.read_meta_data.as_ref()
    }

    pub fn read_rows<T>(
        &mut self,
        sheet: impl AsRef<str>,
        range: Range<u32>,
    ) -> Result<Vec<T>, Box<dyn std::error::Error>>
    where
        T: serde::de::DeserializeOwned,
    {
        if let Some(workbook) = &mut self.workbook {
            let rg = workbook
                .with_header_row(HeaderRow::FirstNonEmptyRow)
                .worksheet_range(sheet.as_ref())?;
            let rg_data = RangeDeserializerBuilder::new().from_range(&rg)?;

            let mut data_array = Vec::new();
            // 只获取指定范围的数据
            for row in rg_data
                .into_iter()
                .skip(if range.start > 0 {
                    (range.start - 1) as usize
                } else {
                    0
                })
                .take(range.end as usize)
            {
                if let Ok(data) = row {
                    data_array.push(data);
                }
            }
            return Ok(data_array);
        }else {
            return Err(Error::Msg("Workbook is none").into());
        }
    }

    pub fn sheet_names(&self) -> Vec<String> {
        self.sheet_names.clone()
    }
}

pub fn open_workbook<P>(path: P) -> Result<ExcelReader<BufReader<File>>, calamine::Error>
where
    P: AsRef<Path>,
{
    let workbook = open_workbook_auto(path)?;
    let mut excel_reader = ExcelReader {
        sheet_names: Vec::new(),
        read_meta_data: None,
        workbook: None,
    };
    excel_reader.sheet_names = workbook.sheet_names();
    excel_reader.workbook = Some(workbook);

    Ok(excel_reader)
}

pub fn open_workbook_from_data<'a, A>(
    name: A,
    data: &'a [u8],
) -> Result<ExcelReader<Cursor<&'a [u8]>>, Error>
where
    A: AsRef<Path>,
{
    let name = name.as_ref();
    if !name.is_file() {
        return Err(Error::Msg("Cannot detect file format"));
    }

    let ext = name.extension();
    if ext.is_none() {
        return Err(Error::Msg("Extension name does not exist"));
    }

    let extension = ext.unwrap().to_str().unwrap_or_default();

    let work_data = data.as_ref();
    let read_meta_data = ReadMetaData {
        name: name.to_string_lossy().to_string(),
        extension: extension.into(),
        header: {
            let mut header = Vec::new();
            let index = ExcelType::index(extension);
            for i in 0..index {
                header.push(work_data[i]);
            }
            header
        },
    };

    let excel_type = ExcelType::from_meta_data(&read_meta_data);
    if let Err(_) = excel_type {
        return Err(Error::Msg("Unsupported type or header bytes  do not match"));
    }
    
    let mut excel_reader = ExcelReader {
        sheet_names: Vec::new(),
        read_meta_data: None,
        workbook: None,
    };
    

    let excel_type = excel_type.unwrap();
    excel_reader.read_meta_data = Some(read_meta_data);

    let cursor = Cursor::new(work_data);
    let workbook: Sheets<Cursor<&[u8]>> = match excel_type {
        ExcelType::Xls => Sheets::Xls(open_workbook_from_rs(cursor).unwrap()),
        ExcelType::Xlsx => Sheets::Xlsx(open_workbook_from_rs(cursor).unwrap()),
    };

    excel_reader.workbook = Some(workbook);

    Ok(excel_reader)
}
