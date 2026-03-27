use std::sync::atomic::{AtomicU32, Ordering};

/// Global counter for generating unique IDs
/// Thread-safe atomic counter that can be safely used in multi-threaded environments
///
/// 全局计数器，用于生成唯一ID
/// 使用 AtomicU32 确保线程安全，可以在多线程环境下安全使用
static COUNTER: AtomicU32 = AtomicU32::new(0);

/// Generate a formatted unique ID string
///
/// # How it works / 工作原理
/// 1. Atomically get current counter value and increment by 1
///    原子性地获取当前计数器的值并加1
/// 2. Take modulo 1000 to ensure ID cycles between 0-999
///    对 1000 取模，确保ID在 0-999 之间循环
/// 3. Format as 3-digit number with zero padding (e.g., "001", "042", "999")
///    格式化为3位数字，不足3位时补零（如 "001", "042", "999"）
///
/// # Return value / 返回值
/// Returns a formatted 3-digit string, ranging from "000" to "999"
/// 返回一个格式化的3位数字符串，范围从 "000" 到 "999"
///
/// # Thread safety / 线程安全
/// Uses atomic operations, safe to call in multi-threaded environments
/// 使用原子操作，可以在多线程环境中安全调用
///
/// # Examples / 示例
/// ```
/// let id1 = unique_id(); // First call returns "000" / 第一次调用返回 "000"
/// let id2 = unique_id(); // Second call returns "001" / 第二次调用返回 "001"
/// // ...
/// let id1000 = unique_id(); // 1000th call returns "000" (wraps around) / 第1000次调用返回 "000"（循环）
/// ```
///
/// # Notes / 注意
/// - Uses `Relaxed` memory ordering because we only need atomicity, not synchronization
///   使用 `Relaxed` 内存顺序，因为这里只需要原子性，不需要同步
/// - IDs wrap around after reaching 999, suitable for scenarios requiring only short-term uniqueness
///   ID在达到 999 后会回绕到 000，适用于只需要短期唯一性的场景
pub fn unique_id() -> String {
    // fetch_add atomically increments the counter and returns the previous value
    // Ordering::Relaxed provides the lightest atomic guarantee, suitable for counters
    //
    // fetch_add 原子性地增加计数器的值，并返回增加前的值
    // Ordering::Relaxed 提供最轻量级的原子保证，适合计数器场景
    let val = COUNTER.fetch_add(1, Ordering::Relaxed) % 1_000;

    // Format as 3-digit number with zero padding on the left
    // {:03} means width of 3, padded with zeros
    //
    // 格式化为3位数字，不足3位时左边补零
    // {:03} 表示宽度为3，用0填充
    format!("{:03}", val)
}
