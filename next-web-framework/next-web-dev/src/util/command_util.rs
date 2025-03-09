use std::collections::HashMap;

/**
*struct:    CommandUtil
*desc:      命令工具类 适用于 linux shell 命令
*author:    Listening
*email:     yuenxillar@163.com
*date:      2024/10/02
*/

pub struct CommandUtil;

/// 运行命令工具类
impl CommandUtil {
    //! 运行命令 例如 linux shell 命令
    pub fn run_command(program: &str, args: Vec<&str>) -> Result<String, std::io::Error> {
        // run the ls command

        let output = std::process::Command::new(program).args(args).output()?;

        if !output.status.success() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                String::from_utf8_lossy(output.stdout.as_slice()),
            ));
        }

        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }

    pub fn handle_args(args: Vec<String>) -> HashMap<String, String> {
        let mut data: HashMap<String, String> = HashMap::new();
        // 解析命令行参数
        // 格式：--key=value 或者 -key2=value2
        for arg in args {
            if !arg.starts_with("--") || !arg.starts_with("-") {
                continue;
            }
            if !arg.contains("=") {
                continue;
            }

            let mut split_arg = arg.split('=');
            let key = split_arg.next().unwrap().to_string();
            let value = split_arg.next().unwrap().to_string();
            data.insert(key, value);
        }
        return data;
    }
}