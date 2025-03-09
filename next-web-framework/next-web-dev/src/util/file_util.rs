use std::{
    error::Error,
    fmt::Debug,
    fs::File,
    io::{BufReader, Read},
    path::Path,
};

use hashbrown::HashMap;
use regex::Regex;
use serde::de::DeserializeOwned;

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

    // 判断文件是否存在
    pub fn is_file_exist(path: &str) -> bool {
        Path::new(path).exists()
    }

    #[inline]
    pub fn read_file_to_string(path: &str) -> Result<String, Box<dyn Error>> {
        let file = File::open(Path::new(path))?;
        let mut reader = BufReader::new(file);
        let mut buf = String::new();
        reader.read_to_string(&mut buf).unwrap();
        Ok(buf)
    }

    pub fn read_file_into_application<T: DeserializeOwned + Debug>(
        file_path: &str,
    ) -> (T, HashMap<String, serde_yaml::Value>) {
        use serde_yaml::Value;

        println!("read application file: {}", file_path);
        match std::fs::metadata(file_path) {
            Ok(_) => (),
            Err(_error) => panic!("The application config file is not exits!!"),
        }
        let mut file = File::open(file_path).expect("application file open is _error!!");
        let mut str = String::new();
        let _ = file.read_to_string(&mut str).unwrap();

        // replace var
        let replace_var = |content: &str| -> String {
            let re = Regex::new(r"\$\{server_ip\}").unwrap();

            // 替换 ${server_ip} 为新的 IP 地址
            let updated_content = re.replace_all(&content, "192.168.1.130");
            updated_content.to_string()
        };

        let buf = replace_var(str.as_str());

        // mapping value
        let docs = serde_yaml::from_str::<Value>(&buf).unwrap();
        let mut data_map: HashMap<String, Value> = HashMap::new();

        // Prepare a recursive function to fill in
        fn populate_map(prefix: String, value: &Value, map: &mut HashMap<String, Value>) {
            match value {
                Value::Mapping(mapping) => {
                    for (k, v) in mapping {
                        if let Some(key) = k.as_str() {
                            populate_map(
                                format!(
                                    "{}{}",
                                    if prefix.is_empty() {
                                        String::new()
                                    } else {
                                        format!("{}.", prefix)
                                    },
                                    key
                                ),
                                v,
                                map,
                            );
                        }
                    }
                }
                _ => {
                    map.insert(prefix, value.clone());
                }
            }
        }

        // Fill in the map
        populate_map(String::new(), &docs, &mut data_map);

        // into application
        let application: T = serde_yaml::from_str(buf.as_str()).unwrap();

        // return
        return (application, data_map);
    }
}
