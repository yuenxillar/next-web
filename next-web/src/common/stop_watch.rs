use std::time::{Duration, Instant};
use std::fmt;

/// 高性能秒表实现，支持启动、停止、重置、分段计时等功能
///
/// High-performance stopwatch implementation with start, stop, reset, lap timing, and more.
#[derive(Debug, Clone)]
pub struct Stopwatch {
    /// 开始时间
    start_time: Option<Instant>,
    /// 累计运行时间
    elapsed: Duration,
    /// 分段计时记录
    laps: Vec<Duration>,
    /// 是否正在运行
    is_running: bool,
}

impl Default for Stopwatch {
    fn default() -> Self {
        Self::new()
    }
}

impl Stopwatch {
    /// 创建一个新的秒表
    pub fn new() -> Self {
        Stopwatch {
            start_time: None,
            elapsed: Duration::default(),
            laps: Vec::new(),
            is_running: false,
        }
    }

    pub fn start_new() -> Self {
        let mut stopwatch = Stopwatch::new();
        stopwatch.start();
        stopwatch
    }

    /// 启动或继续计时
    pub fn start(&mut self) {
        if !self.is_running {
            self.start_time = Some(Instant::now());
            self.is_running = true;
        }
    }

    /// 停止计时
    pub fn stop(&mut self) {
        if let Some(start) = self.start_time.take() {
            self.elapsed += start.elapsed();
            self.is_running = false;
        }
    }

    /// 重置秒表（清除所有计时和分段记录）
    pub fn reset(&mut self) {
        self.start_time = None;
        self.elapsed = Duration::default();
        self.laps.clear();
        self.is_running = false;
    }

    /// 记录一个分段时间（lap time）
    pub fn lap(&mut self) -> Duration {
        if self.is_running {
            let lap_time = self.start_time.unwrap().elapsed();
            self.laps.push(lap_time);
            lap_time
        } else {
            Duration::default()
        }
    }

    /// 获取当前累计时间
    pub fn elapsed(&self) -> Duration {
        let mut total = self.elapsed;
        if let Some(start) = self.start_time {
            total += start.elapsed();
        }
        total
    }

    /// 获取所有分段时间
    pub fn laps(&self) -> &[Duration] {
        &self.laps
    }

    /// 清除所有分段记录（保留主计时）
    pub fn clear_laps(&mut self) {
        self.laps.clear();
    }

    /// 检查秒表是否正在运行
    pub fn is_running(&self) -> bool {
        self.is_running
    }

    /// 获取当前累计时间（秒）
    pub fn elapsed_secs(&self) -> u64 {
        self.elapsed().as_secs()
    }

    /// 获取当前累计时间（毫秒）
    pub fn elapsed_millis(&self) -> u128 {
        self.elapsed().as_millis()
    }

    /// 获取当前累计时间（微秒）
    pub fn elapsed_micros(&self) -> u128 {
        self.elapsed().as_micros()
    }

    /// 获取当前累计时间（纳秒）
    pub fn elapsed_nanos(&self) -> u128 {
        self.elapsed().as_nanos()
    }

    /// 获取当前累计时间（秒，浮点数精度）
    pub fn elapsed_secs_f64(&self) -> f64 {
        self.elapsed().as_secs_f64()
    }

    // /// 获取当前累计时间（毫秒，浮点数精度）
    // pub fn elapsed_millis_f64(&self) -> f64 {
    //     self.elapsed().as_millis_f64()
    // }

    // /// 获取当前累计时间（微秒，浮点数精度）
    // pub fn elapsed_micros_f64(&self) -> f64 {
    //     self.elapsed().as_micros_f64()
    // }

    /// 重启计时器（重置并立即开始）
    pub fn restart(&mut self) -> Duration {
        let elapsed = self.elapsed();
        self.reset();
        self.start();
        elapsed
    }

    /// 获取最后一段分段时间
    pub fn last_lap(&self) -> Option<Duration> {
        self.laps.last().copied()
    }

    /// 获取分段时间的平均值
    pub fn average_lap(&self) -> Option<Duration> {
        if self.laps.is_empty() {
            None
        } else {
            let total: Duration = self.laps.iter().sum();
            Some(total / self.laps.len() as u32)
        }
    }

    /// 获取最快分段时间
    pub fn fastest_lap(&self) -> Option<Duration> {
        self.laps.iter().min().copied()
    }

    /// 获取最慢分段时间
    pub fn slowest_lap(&self) -> Option<Duration> {
        self.laps.iter().max().copied()
    }
}

// 为 Stopwatch 实现 Display trait，方便格式化输出
impl fmt::Display for Stopwatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let elapsed = self.elapsed();
        write!(f, "{:.3}s", elapsed.as_secs_f64())
    }
}

// 为 Stopwatch 实现一些有用的 trait
impl PartialEq for Stopwatch {
    fn eq(&self, other: &Self) -> bool {
        self.elapsed() == other.elapsed()
    }
}

impl PartialOrd for Stopwatch {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.elapsed().partial_cmp(&other.elapsed())
    }
}

/// 便捷函数：测量代码块的执行时间
///
/// # 示例
/// ```
/// let result = time_it!(|| {
///     // 需要计时的代码
///     std::thread::sleep(std::time::Duration::from_millis(100));
///     42
/// });
/// ```
#[macro_export]
macro_rules! time_it {
    ($block:expr) => {{
        let mut stopwatch = $crate::common::stop_watch::Stopwatch::start_new();
        let result = $block();
        stopwatch.stop();
        (result, stopwatch.elapsed())
    }};
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn test_basic_operations() {
        let mut sw = Stopwatch::new();
        
        // 测试开始
        sw.start();
        sleep(Duration::from_millis(50));
        assert!(sw.elapsed_millis() >= 50);
        assert!(sw.is_running());
        
        // 测试停止
        sw.stop();
        let elapsed = sw.elapsed_millis();
        sleep(Duration::from_millis(50));
        assert_eq!(sw.elapsed_millis(), elapsed); // 停止后时间不应增加
        
        // 测试重置
        sw.reset();
        assert_eq!(sw.elapsed_millis(), 0);
        assert!(!sw.is_running());
    }

    #[test]
    fn test_lap_timing() {
        let mut sw = Stopwatch::start_new();
        sleep(Duration::from_millis(10));
        
        let lap1 = sw.lap();
        assert!(lap1.as_millis() >= 10);
        
        sleep(Duration::from_millis(20));
        let lap2 = sw.lap();
        assert!(lap2.as_millis() >= 20);
        
        assert_eq!(sw.laps().len(), 2);
        assert!(sw.average_lap().unwrap().as_millis() >= 15);
    }

    #[test]
    fn test_time_it_macro() {
        let (result, duration) = time_it!(|| {
            sleep(Duration::from_millis(30));
            42
        });
        
        assert_eq!(result, 42);
        assert!(duration.as_millis() >= 30);
    }

    #[test]
    fn test_restart() {
        let mut sw = Stopwatch::start_new();
        sleep(Duration::from_millis(25));
        
        let previous_elapsed = sw.restart();
        assert!(previous_elapsed.as_millis() >= 25);
        assert!(sw.is_running());
        assert!(sw.elapsed_millis() < 10); // 重启后时间应该很短
    }

    #[test]
    fn test_display() {
        let mut sw = Stopwatch::new();
        sw.start();
        sleep(Duration::from_millis(100));
        sw.stop();
        
        let display_str = format!("{}", sw);
        assert!(display_str.contains("0.1")); // 应该显示约0.1秒
    }
}