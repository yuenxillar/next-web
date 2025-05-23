pub mod core;
mod support;

#[cfg(test)]
pub mod tests {
    use std::time::Instant;

    use umya_spreadsheet::{writer::xlsx::write, Worksheet};

    use crate::core::{
        read::excel_reader::{ExcelReader, open_workbook, open_workbook_from_data},
        write::excel_writer::ExcelWriter,
    };

    #[derive(serde::Serialize, serde::Deserialize, Debug)]
    pub struct User {
        #[serde(rename = "你开了吗")]
        pub id: i64,
        #[serde(rename = "我没开")]
        pub number: i64,
        #[serde(rename = "好家伙")]
        pub datetime: i64,
    }

    #[test]
    fn it_works() {
        // let mut file = std::fs::File::open("hello.xlsx").unwrap();

        // let mut buf = Vec::new();
        // file.read_to_end(&mut buf).unwrap();

        // // open_workbook_auto_from_rs(data)
        // let mut excel = open_workbook("hello.xlsx").unwrap();
        // let result = excel.read_rows::<User>("客户要求", 1..4);
        // if let Ok(data) = result {
        //     for row in data {
        //         println!("{:?}", row);
        //     }
        // }

        let mut book = umya_spreadsheet::new_file();
        let instant = Instant::now();
        let mut worksheet = Worksheet::default();
        worksheet.set_name("TestSheet1");
        let sheet = book.add_sheet(worksheet).unwrap();
        println!("Time elapsed in open_workbook is: {:?}", instant.elapsed());
        for i in 1..1000{
            for i1 in 1..300 {
                sheet.get_cell_value((i, i1)).set_value("Hello World");
            }
        }

        println!("Time elapsed in open_workbook is: {:?}", instant.elapsed());
        write(&book, "test666.xlsx").unwrap();
        println!("Time elapsed in open_workbook is: {:?}", instant.elapsed());
    }

    #[test]
    fn it_works1() {
        let mut writer = ExcelWriter::new();
        let mut buf = Vec::new();
        for i in 1..10000 {
            buf.push(User {
                id: i,
                number: i + 100,
                datetime: i + 1000,
            });
        }
        // writer.write("Sheet1000", &buf);
        writer.save("test.xlsx").unwrap();
    }
}
