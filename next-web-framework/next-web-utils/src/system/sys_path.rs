/// `SystemPathUtil` 是一个工具结构体，用于获取系统路径相关的信息。
pub struct SystemPathUtil;

impl SystemPathUtil {
    /// 获取当前工作目录的路径。
    /// 如果无法获取当前工作目录，则返回 `"./"` 作为默认值
    pub fn current_dir() -> String {
        std::env::current_dir()
            .map(|s| s.to_str().map(|s| s.to_string()))
            .map(|s| s.unwrap_or("./".into()))
            .unwrap_or("./".into())
    }

    /// 获取用户的主目录路径。
    /// 如果无法获取主目录路径，则返回 `None`
    pub fn home_dir() -> Option<String> {
        dirs::home_dir()
            .map(|s| s.to_str().map(|s| s.to_string()))
            .unwrap_or(None)
    }

    /// 获取系统的临时目录路径。
    /// 如果无法获取临时目录路径，则返回 `None`
    pub fn temp_dir() -> Option<String> {
        std::env::temp_dir().to_str().map(|s| s.to_string())
    }

    /// 获取模板目录路径（如果存在）。
    /// 如果无法获取模板目录路径，则返回 `None`
    pub fn template_dir() -> Option<String> {
        dirs::template_dir().and_then(|f| f.to_str().map(|s| s.to_string()))
    }

    /// 获取下载目录路径（如果存在）。
    /// 如果无法获取下载目录路径，则返回 `None`
    pub fn download_dir() -> Option<String> {
        dirs::download_dir()
            .map(|s| s.to_str().map(|s| s.to_string()))
            .unwrap_or(None)
    }

    /// 获取图片目录路径（如果存在）。
    /// 如果无法获取图片目录路径，则返回 `None`
    pub fn picture_dir() -> Option<String> {
        dirs::picture_dir()
            .map(|s| s.to_str().map(|s| s.to_string()))
            .unwrap_or(None)
    }

    /// 获取配置目录路径（如果存在）。
    /// 如果无法获取配置目录路径，则返回 `None`
    pub fn config_dir() -> Option<String> {
        dirs::config_dir()
            .map(|s| s.to_str().map(|s| s.to_string()))
            .unwrap_or(None)
    }

    /// 获取数据目录路径（如果存在）。
    /// 如果无法获取数据目录路径，则返回 `None`
    pub fn data_dir() -> Option<String> {
        dirs::data_dir()
            .map(|s| s.to_str().map(|s| s.to_string()))
            .unwrap_or(None)
    }

    /// 获取可执行文件所在的目录路径（如果存在）。
    /// 如果无法获取可执行文件目录路径，则返回 `None`
    pub fn executable_dir() -> Option<String> {
        dirs::executable_dir()
            .map(|s| s.to_str().map(|s| s.to_string()))
            .unwrap_or(None)
    }

    /// 获取缓存目录路径（如果存在）。
    /// 如果无法获取缓存目录路径，则返回 `None`
    pub fn cache_dir() -> Option<String> {
        dirs::cache_dir()
            .map(|s| s.to_str().map(|s| s.to_string()))
            .unwrap_or(None)
    }

    /// 获取运行时目录路径（如果存在）。
    /// 如果无法获取运行时目录路径，则返回 `None`
    pub fn runtime_dir() -> Option<String> {
        dirs::runtime_dir()
            .map(|s| s.to_str().map(|s| s.to_string()))
            .unwrap_or(None)
    }

    /// 获取当前可执行文件的目录路径（仅限 Windows 平台）。
    /// 如果无法获取可执行文件路径，则返回 `None`
    #[cfg(target_os = "windows")]
    pub fn current_exe_dir() -> Option<String> {
        std::env::current_exe()
            .map(|s| s.to_str().map(|s| s.to_string()))
            .unwrap_or(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_current_dir() {
        let current_dir = SystemPathUtil::current_dir();
        assert!(!current_dir.is_empty()); // 当前工作目录不应为空
    }

    #[test]
    fn test_home_dir() {
        if let Some(home_dir) = SystemPathUtil::home_dir() {
            assert!(!home_dir.is_empty()); // 主目录路径不应为空
        }
    }

    #[test]
    fn test_temp_dir() {
        if let Some(temp_dir) = SystemPathUtil::temp_dir() {
            assert!(!temp_dir.is_empty()); // 临时目录路径不应为空
        }
    }

    #[test]
    fn test_template_dir() {
        if let Some(template_dir) = SystemPathUtil::template_dir() {
            assert!(!template_dir.is_empty()); // 模板目录路径不应为空
        }
    }

    #[test]
    fn test_download_dir() {
        if let Some(download_dir) = SystemPathUtil::download_dir() {
            assert!(!download_dir.is_empty()); // 下载目录路径不应为空
        }
    }

    #[test]
    fn test_picture_dir() {
        if let Some(picture_dir) = SystemPathUtil::picture_dir() {
            assert!(!picture_dir.is_empty()); // 图片目录路径不应为空
        }
    }

    #[test]
    fn test_config_dir() {
        if let Some(config_dir) = SystemPathUtil::config_dir() {
            assert!(!config_dir.is_empty()); // 配置目录路径不应为空
        }
    }

    #[test]
    fn test_data_dir() {
        if let Some(data_dir) = SystemPathUtil::data_dir() {
            assert!(!data_dir.is_empty()); // 数据目录路径不应为空
        }
    }

    #[test]
    fn test_executable_dir() {
        if let Some(executable_dir) = SystemPathUtil::executable_dir() {
            assert!(!executable_dir.is_empty()); // 可执行文件目录路径不应为空
        }
    }

    #[test]
    fn test_cache_dir() {
        if let Some(cache_dir) = SystemPathUtil::cache_dir() {
            assert!(!cache_dir.is_empty()); // 缓存目录路径不应为空
        }
    }

    #[test]
    fn test_runtime_dir() {
        if let Some(runtime_dir) = SystemPathUtil::runtime_dir() {
            assert!(!runtime_dir.is_empty()); // 运行时目录路径不应为空
        }
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_current_exe_dir() {
        if let Some(current_exe_dir) = SystemPathUtil::current_exe_dir() {
            assert!(!current_exe_dir.is_empty()); // 当前可执行文件目录路径不应为空
        }
    }
}
