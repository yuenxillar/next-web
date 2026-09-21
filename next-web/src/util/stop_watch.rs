use std::fmt;
use std::time::{Duration, Instant};

/// A high-performance stopwatch supporting start, stop, reset, and lap timing.
#[derive(Debug, Clone)]
pub struct StopWatch {
    /// The instant when the stopwatch was last started, if running.
    start_time: Option<Instant>,
    /// Accumulated elapsed time from previous runs.
    elapsed: Duration,
    /// Recorded lap timestamps (cumulative elapsed at each lap).
    laps: Vec<Duration>,
    /// Whether the stopwatch is currently running.
    is_running: bool,
}

impl StopWatch {
    /// Creates a new, stopped stopwatch.
    pub fn new() -> Self {
        StopWatch {
            start_time: None,
            elapsed: Duration::default(),
            laps: Vec::new(),
            is_running: false,
        }
    }

    /// Creates a new stopwatch and starts it immediately.
    pub fn start_new() -> Self {
        let mut sw = StopWatch::new();
        sw.start();
        sw
    }

    /// Starts or resumes the stopwatch.
    pub fn start(&mut self) {
        if !self.is_running {
            self.start_time = Some(Instant::now());
            self.is_running = true;
        }
    }

    /// Stops the stopwatch, accumulating the elapsed time.
    pub fn stop(&mut self) {
        if let Some(start) = self.start_time.take() {
            self.elapsed += start.elapsed();
            self.is_running = false;
        }
    }

    /// Resets the stopwatch, clearing all elapsed time and lap records.
    pub fn reset(&mut self) {
        self.start_time = None;
        self.elapsed = Duration::default();
        self.laps.clear();
        self.is_running = false;
    }

    /// Records a lap and returns the duration since the previous lap.
    ///
    /// The first lap returns the time since the stopwatch started.
    /// Returns `Duration::ZERO` if the stopwatch is not running.
    pub fn lap(&mut self) -> Duration {
        if !self.is_running {
            return Duration::default();
        }
        let now = self.start_time.unwrap().elapsed();
        let last_total = self.laps.last().copied().unwrap_or(Duration::ZERO);
        let lap_duration = now - last_total;
        self.laps.push(now);
        lap_duration
    }

    /// Returns the current total elapsed time.
    pub fn elapsed(&self) -> Duration {
        let mut total = self.elapsed;
        if let Some(start) = self.start_time {
            total += start.elapsed();
        }
        total
    }

    /// Returns all recorded lap timestamps (cumulative elapsed at each lap).
    pub fn laps(&self) -> &[Duration] {
        &self.laps
    }

    /// Clears all lap records while keeping the main elapsed time.
    pub fn clear_laps(&mut self) {
        self.laps.clear();
    }

    /// Returns whether the stopwatch is currently running.
    pub fn is_running(&self) -> bool {
        self.is_running
    }

    /// Returns the elapsed time in whole seconds.
    pub fn elapsed_secs(&self) -> u64 {
        self.elapsed().as_secs()
    }

    /// Returns the elapsed time in whole milliseconds.
    pub fn elapsed_millis(&self) -> u128 {
        self.elapsed().as_millis()
    }

    /// Returns the elapsed time in whole microseconds.
    pub fn elapsed_micros(&self) -> u128 {
        self.elapsed().as_micros()
    }

    /// Returns the elapsed time in whole nanoseconds.
    pub fn elapsed_nanos(&self) -> u128 {
        self.elapsed().as_nanos()
    }

    /// Returns the elapsed time as seconds in floating-point precision.
    pub fn elapsed_secs_f64(&self) -> f64 {
        self.elapsed().as_secs_f64()
    }

    /// Restarts the stopwatch (reset and start immediately).
    ///
    /// Returns the elapsed time before the restart.
    pub fn restart(&mut self) -> Duration {
        let elapsed = self.elapsed();
        self.reset();
        self.start();
        elapsed
    }

    /// Returns the most recent lap timestamp, if any.
    pub fn last_lap(&self) -> Option<Duration> {
        self.laps.last().copied()
    }

    /// Returns the average lap duration, or `None` if no laps were recorded.
    pub fn average_lap(&self) -> Option<Duration> {
        if self.laps.is_empty() {
            None
        } else {
            let total: Duration = self.laps.iter().sum();
            Some(total / self.laps.len() as u32)
        }
    }

    /// Returns the fastest lap timestamp, if any.
    pub fn fastest_lap(&self) -> Option<Duration> {
        self.laps.iter().min().copied()
    }

    /// Returns the slowest lap timestamp, if any.
    pub fn slowest_lap(&self) -> Option<Duration> {
        self.laps.iter().max().copied()
    }
}

/// Formats the stopwatch as seconds with three decimal places.
impl fmt::Display for StopWatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.3}s", self.elapsed().as_secs_f64())
    }
}

impl PartialEq for StopWatch {
    fn eq(&self, other: &Self) -> bool {
        self.elapsed() == other.elapsed()
    }
}

impl PartialOrd for StopWatch {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.elapsed().partial_cmp(&other.elapsed())
    }
}

impl Default for StopWatch {
    fn default() -> Self {
        Self::new()
    }
}

/// Measures the execution time of a code block.
///
/// # Example
///
/// ```ignore
/// let (result, elapsed) = time_it!(|| {
///     std::thread::sleep(std::time::Duration::from_millis(100));
///     42
/// });
/// ```
#[macro_export]
macro_rules! time_it {
    ($block:expr) => {{
        let mut __sw = $crate::common::stop_watch::StopWatch::start_new();
        let result = $block();
        __sw.stop();
        (result, __sw.elapsed())
    }};
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn test_basic_operations() {
        let mut sw = StopWatch::new();
        sw.start();
        sleep(Duration::from_millis(50));
        assert!(sw.elapsed_millis() >= 50);
        assert!(sw.is_running());

        sw.stop();
        let elapsed = sw.elapsed_millis();
        sleep(Duration::from_millis(50));
        assert_eq!(sw.elapsed_millis(), elapsed);

        sw.reset();
        assert_eq!(sw.elapsed_millis(), 0);
        assert!(!sw.is_running());
    }

    #[test]
    fn test_lap_timing() {
        let mut sw = StopWatch::start_new();
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
    fn test_restart() {
        let mut sw = StopWatch::start_new();
        sleep(Duration::from_millis(25));

        let previous_elapsed = sw.restart();
        assert!(previous_elapsed.as_millis() >= 25);
        assert!(sw.is_running());
        assert!(sw.elapsed_millis() < 10);
    }

    #[test]
    fn test_display() {
        let mut sw = StopWatch::new();
        sw.start();
        sleep(Duration::from_millis(100));
        sw.stop();

        let display_str = format!("{}", sw);
        assert!(display_str.contains("0.1"));
    }
}
