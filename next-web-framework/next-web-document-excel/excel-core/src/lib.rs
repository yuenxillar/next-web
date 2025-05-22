pub mod core;
mod support;

#[cfg(test)]
pub mod tests {
    use std::io::Read;
    use crate::core::read::excel_reader::{ExcelReader, open_workbook, open_workbook_from_data};

    #[test]
    fn it_works() {
        use rust_xlsxwriter::*;

        #[derive(serde::Serialize, serde::Deserialize, Debug)]
        pub struct User {
            id: i64,
            number: i64,
            datetime: i64,
        }
        let mut file = std::fs::File::open("hello.xlsx").unwrap();

        let mut buf = Vec::new();
        file.read_to_end(&mut buf).unwrap();

        // open_workbook_auto_from_rs(data)
        let mut excel = open_workbook("hello.xlsx").unwrap();
        let result = excel.read_rows::<User>("客户要求", 1..4);
        if let Ok(data) =  result{
            for row in data {
                println!("{:?}", row);
            }
        }
    }
}
