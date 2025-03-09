use std::time::Duration;

/// 线程池配置结构体，用于定义线程池的核心参数。
/// 该结构体支持自定义核心线程数、最大线程数、等待队列长度以及线程存活时间。
#[derive(Clone)]
pub struct ThreadPoolConfig {
    // 核心线程数：线程池中始终保持运行的最小线程数量。
    core_threads: usize,
    // 最大线程数：线程池中允许的最大线程数量。
    max_threads: usize,
    // 等待队列的最大长度：当所有线程都在忙碌时，任务会被放入等待队列。
    queue_size: usize,
    // 线程存活时间：非核心线程在空闲一段时间后会被回收（可选）。
    keep_alive: Option<Duration>,
}

impl ThreadPoolConfig {
    /// 创建一个新的 `ThreadPoolConfig` 实例。
    /// 
    /// # 参数
    /// - `core_threads`: 核心线程数。
    /// - `max_threads`: 最大线程数。
    /// - `queue_size`: 等待队列的最大长度。
    /// 
    /// # 返回值
    /// 返回一个默认的 `ThreadPoolConfig` 实例，`keep_alive` 默认为 `None`。
    pub fn new(core_threads: usize, max_threads: usize, queue_size: usize) -> Self {
        ThreadPoolConfig {
            core_threads,
            max_threads,
            queue_size,
            keep_alive: None,
        }
    }

    /// 获取核心线程数。
    /// 
    /// # 返回值
    /// 返回当前配置中的核心线程数。
    pub fn get_core_threads(&self) -> usize {
        self.core_threads
    }

    /// 获取最大线程数。
    /// 
    /// # 返回值
    /// 返回当前配置中的最大线程数。
    pub fn get_max_threads(&self) -> usize {
        self.max_threads
    }

    /// 获取线程存活时间。
    /// 
    /// # 返回值
    /// 返回当前配置中的线程存活时间（如果设置）。
    pub fn get_keep_alive(&self) -> Option<Duration> {
        self.keep_alive
    }

    /// 获取等待队列的最大长度。
    /// 
    /// # 返回值
    /// 返回当前配置中的等待队列长度。
    pub fn get_queue_size(&self) -> usize {
        self.queue_size
    }

    /// 设置核心线程数。
    /// 
    /// # 参数
    /// - `core_threads`: 新的核心线程数。
    pub fn set_core_threads(&mut self, core_threads: usize) {
        self.core_threads = core_threads;
    }

    /// 设置最大线程数。
    /// 
    /// # 参数
    /// - `max_threads`: 新的最大线程数。
    pub fn set_max_threads(&mut self, max_threads: usize) {
        self.max_threads = max_threads;
    }

    /// 设置等待队列的最大长度。
    /// 
    /// # 参数
    /// - `queue_size`: 新的等待队列长度。
    pub fn set_queue_size(&mut self, queue_size: usize) {
        self.queue_size = queue_size;
    }

    /// 设置线程存活时间。
    /// 
    /// # 参数
    /// - `keep_alive`: 新的线程存活时间（可选）。
    pub fn set_keep_alive(&mut self, keep_alive: Option<Duration>) {
        self.keep_alive = keep_alive;
    }
}

impl Default for ThreadPoolConfig {
    /// 提供默认的线程池配置。
    /// 
    /// # 默认值
    /// - 核心线程数：4
    /// - 最大线程数：8
    /// - 等待队列长度：100
    /// - 线程存活时间：`None`
    fn default() -> Self {
        ThreadPoolConfig {
            core_threads: 4,
            max_threads: 8,
            queue_size: 100,
            keep_alive: None,
        }
    }
}