use std::time::{Duration, Instant};

/// 秒表
pub struct Stopwatch {
    // 开始时间
    start_time: Option<Instant>,
    // 累计时间
    elapsed: Duration,
}

impl Stopwatch {
    /// 创建一个新的秒表
    pub fn new() -> Self {
        Stopwatch {
            start_time: None,
            elapsed: Duration::default(),
        }
    }

    /// 启动秒表
    pub fn start(&mut self) {
        self.start_time = Some(Instant::now());
    }

    /// 停止秒表
    pub fn stop(&mut self) {
        if let Some(start) = self.start_time {
            self.elapsed += start.elapsed();
            self.start_time = None;
        }
    }

    /// 重置秒表
    pub fn reset(&mut self) {
        self.start_time = None;
        self.elapsed = Duration::default();
    }

    /// 获取当前累计时间
    pub fn elapsed(&self) -> Duration {
        let mut total = self.elapsed;
        if let Some(start) = self.start_time {
            total += start.elapsed();
        }
        total
    }

    /// 获取当前累计时间（以秒为单位）
    pub fn elapsed_seconds(&self) -> f64 {
        self.elapsed().as_secs_f64()
    }

    /// 获取当前累计时间（以毫秒为单位）
    pub fn elapsed_millis(&self) -> f64 {
        self.elapsed().as_millis() as f64
    }
}
