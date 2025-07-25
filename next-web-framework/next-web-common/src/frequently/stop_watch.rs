use std::time::{Duration, Instant};

/// 简单的秒表实现，可以启动、停止和重置
///
/// A simple stopwatch implementation that can be started, stopped, and reset.
pub struct Stopwatch {
    /// 开始时间
    ///
    /// The time when the stopwatch was last started. If `None`, the stopwatch is not running.
    start_time: Option<Instant>,

    /// 秒表自创建或上次重置以来运行的总持续时间
    ///
    /// The total duration the stopwatch has been running since its creation or last reset.
    elapsed: Duration,
}

impl Stopwatch {
    /// 创建一个新的秒表
    ///
    /// Initializes a new stopwatch with zero elapsed time and no active start time.
    pub fn new() -> Self {
        Stopwatch {
            start_time: None,
            elapsed: Duration::default(),
        }
    }

    /// 启动秒表
    ///
    /// Records the current instant as the start time of the stopwatch. If the stopwatch is already
    /// running, this call does nothing.
    pub fn start(&mut self) {
        if self.start_time.is_none() {
            self.start_time = Some(Instant::now());
        }
    }

    /// 停止秒表
    ///
    /// Stops the stopwatch by recording the elapsed time since the last start and resetting the start
    /// time. If the stopwatch is not running, this call does nothing.
    pub fn stop(&mut self) {
        if let Some(start) = self.start_time.take() {
            self.elapsed += start.elapsed();
        }
    }

    /// 重置秒表
    ///
    /// Resets the stopwatch to its initial state with zero elapsed time and no active start time.
    pub fn reset(&mut self) {
        self.start_time = None;
        self.elapsed = Duration::default();
    }

    /// 获取当前累计时间
    ///
    /// Returns the total duration the stopwatch has been running. This includes any time between starts
    /// and stops, plus any additional time if the stopwatch is currently running.
    pub fn elapsed(&self) -> Duration {
        let mut total = self.elapsed;
        if let Some(start) = self.start_time {
            total += start.elapsed();
        }
        total
    }

    /// 获取当前累计时间（以秒为单位）
    ///
    /// Returns the total duration in seconds the stopwatch has been running.
    pub fn elapsed_seconds(&self) -> f64 {
        self.elapsed().as_secs_f64()
    }

    /// 获取当前累计时间（以毫秒为单位）
    ///
    /// Returns the total duration in milliseconds the stopwatch has been running.
    pub fn elapsed_millis(&self) -> f64 {
        self.elapsed().as_millis() as f64
    }
}