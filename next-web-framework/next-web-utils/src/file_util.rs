use std::{
    error::Error,
    fs::File,
    io::{BufReader, Read},
    path::Path,
};

/**
*struct:    FileUtil
*desc:      文件操作工具类
*author:    Listening
*email:     yuenxillar@163.com
*date:      2024/10/02
*/

pub struct FileUtil;

impl FileUtil {
    pub fn read_file(path: &str) -> Result<Vec<u8>, Box<dyn Error>> {
        let file = File::open(Path::new(path))?;
        let mut reader = BufReader::new(file);
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf).unwrap();
        Ok(buf)
    }
}
