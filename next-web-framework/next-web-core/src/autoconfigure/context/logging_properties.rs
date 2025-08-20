/// 日志配置属性
///
/// 包含控制日志功能行为的各项可选设置。
///
/// # Logging Properties
///
/// Contains optional settings to control the behavior of the logging functionality.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct LoggingProperties {
    /// 是否写入日志文件
    ///
    /// Write to log file or not.
    write: Option<bool>,

    /// 日志级别
    ///
    /// 日志记录的级别。
    /// 可选值: trace | debug | info | warn | error
    ///
    /// # Log Level
    ///
    /// The level of logging.
    /// Optional value: trace | debug | info | warn | error
    level: Option<String>,

    /// 日志文件存储目录
    ///
    /// # Log Directory
    ///
    /// The directory where log files are stored.
    log_dir: Option<String>,

    /// 日志文件最大大小（单位：MB）
    ///
    /// # Maximum Log Size
    ///
    /// The maximum size of a single log file in megabytes (MB).
    log_maximum_size: Option<u32>,

    /// 是否在日志文件名中添加日期
    ///
    /// # Additional Date Suffix
    ///
    /// Whether to append a date suffix to the log file name.
    additional_date: Option<bool>,
}

impl LoggingProperties {
    /// 获取日志写入状态
    ///
    /// 如果配置中未指定，则默认返回 `false`。
    ///
    /// # Returns
    /// * `true` - 写入日志
    /// * `false` - 不写入日志（默认）
    ///
    /// # Get Logging Write Status
    ///
    /// Returns `true` if logging is written, `false` otherwise.
    /// Defaults to `false` if not specified in the configuration.
    ///
    /// # Returns
    ///
    /// * `true` - Logging is written
    /// * `false` - Logging is not written (default)
    pub fn write(&self) -> bool {
        self.write.unwrap_or(false)
    }

    pub fn level(&self) -> tracing::Level {
        match self
            .level
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("info")
            .to_lowercase()
            .as_str()
        {
            "trace" => tracing::Level::TRACE,
            "debug" => tracing::Level::DEBUG,
            "info" => tracing::Level::INFO,
            "warn" => tracing::Level::WARN,
            "error" => tracing::Level::ERROR,
            _ => tracing::Level::INFO,
        }
    }

    /// 获取日志目录路径
    ///
    /// # Returns
    ///
    /// 返回 `Some(&str)` 如果设置了日志目录，否则返回 `None`。
    ///
    /// # Get Log Directory Path
    ///
    /// # Returns
    ///
    /// Returns `Some(&str)` if the log directory is set, otherwise returns `None`.
    pub fn log_dir(&self) -> Option<&str> {
        self.log_dir.as_deref()
    }

    /// 获取日志文件最大大小
    ///
    /// # Returns
    ///
    /// 返回 `Some(u32)` 如果设置了最大大小，否则返回 `None`。
    ///
    /// # Get Maximum Log File Size
    ///
    /// # Returns
    ///
    /// Returns `Some(u32)` if the maximum size is set, otherwise returns `None`.
    pub fn log_maximum_size(&self) -> Option<u32> {
        self.log_maximum_size
    }

    /// 获取是否在日志文件名中添加日期
    ///
    /// 如果配置中未指定，则默认返回 `false`。
    ///
    /// # Returns
    ///
    /// * `true` - 在文件名中添加日期
    /// * `false` - 不添加日期（默认）
    ///
    /// # Get Whether to Add Date to Log Filename
    ///
    /// Returns `true` if a date should be appended to the log filename, `false` otherwise.
    /// Defaults to `false` if not specified.
    ///
    /// # Returns
    ///
    /// * `true` - Append date to filename
    /// * `false` - Do not append date (default)
    pub fn additional_date(&self) -> bool {
        self.additional_date.unwrap_or(false)
    }
}

impl Default for LoggingProperties {
    /// 创建具有默认值的 `LoggingProperties`
    ///
    /// 所有字段初始化为 `None`，表示使用系统默认行为或外部配置。
    ///
    /// # Returns
    ///
    /// 一个新的 `LoggingProperties` 实例，所有字段均为 `None`。
    ///
    /// # Create `LoggingProperties` with Default Values
    ///
    /// Initializes a new instance with all fields set to `None`,
    /// indicating that system defaults or external configuration should be used.
    ///
    /// # Returns
    ///
    /// A new `LoggingProperties` instance with all fields as `None`.
    fn default() -> Self {
        Self {
            write: None,
            level: Some("info".to_string()),
            log_dir: None,
            log_maximum_size: None,
            additional_date: None,
        }
    }
}
