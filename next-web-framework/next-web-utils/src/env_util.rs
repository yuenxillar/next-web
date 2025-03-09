/**
*struct:    EnvUtil
*desc:      系统环境变量相关工具类
*author:    Listening
*email:     yuenxillar@163.com
*date:      2024/10/02
*/

pub struct EnvUtil;

impl EnvUtil {
    /// 获取环境变量值
    pub fn get_env_var(key: &str) -> Option<String> {
        match std::env::var(key) {
            Ok(val) => Some(val),
            Err(_) => None,
        }
    }

    /// 设置环境变量值
    pub fn set_env_var(key: &str, value: &str) {
        std::env::set_var(key, value);
    }

    /// 移除环境变量值
    pub fn remove_env_var(key: &str) {
        std::env::remove_var(key);
    }

    /// 获取环境变量值，如果不存在则返回默认值
    pub fn get_env_var_or_default(key: &str, default_value: &str) -> String {
        match std::env::var(key) {
            Ok(val) => val,
            Err(_) => default_value.to_string(),
        }
    }

    /// 获取环境变量值，如果不存在则 panic!
    pub fn get_env_var_or_panic(key: &str) -> String {
        match std::env::var(key) {
            Ok(val) => val,
            Err(_) => panic!("Environment variable {} not found", key),
        }
    }

    ///! 获取环境变量值，如果不存在则打印错误信息并退出程序
    pub fn get_env_var_or_exit(key: &str) -> String {
        match std::env::var(key) {
            Ok(val) => val,
            Err(_) => {
                println!("Environment variable {} not found", key);
                std::process::exit(1);
            }
        }
    }
}
