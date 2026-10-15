use std::sync::atomic::{AtomicU32, Ordering};

/// Global counter for generating unique IDs.
///
/// A thread-safe atomic counter that can be safely used in
/// multi-threaded environments.
static COUNTER: AtomicU32 = AtomicU32::new(0);

/// Generates a formatted unique ID string.
///
/// # How it works
/// 1. Atomically fetch the current counter value and increment it by 1.
/// 2. Take the value modulo 1000 so the ID cycles between 0 and 999.
/// 3. Format it as a 3-digit number with zero padding
///    (e.g., `"001"`, `"042"`, `"999"`).
///
/// # Return value
/// Returns a formatted 3-digit string, ranging from `"000"` to `"999"`.
///
/// # Thread safety
/// Uses atomic operations, so it is safe to call from multiple threads.
///
/// # Examples
/// ```ignore
/// let id1 = unique_id();    // First call returns "000"
/// let id2 = unique_id();    // Second call returns "001"
/// // ...
/// let id1000 = unique_id(); // 1000th call returns "000" (wraps around)
/// ```
///
/// # Notes
/// - Uses `Relaxed` memory ordering because only atomicity is required,
///   not synchronization.
/// - IDs wrap around after reaching 999, so this is suitable only for
///   scenarios that need short-term uniqueness.
pub fn unique_id() -> String {
    // `fetch_add` atomically increments the counter and returns the
    // previous value. `Ordering::Relaxed` provides the lightest atomic
    // guarantee, which is sufficient for a simple counter.
    let val = COUNTER.fetch_add(1, Ordering::Relaxed) % 1_000;

    // Format as a 3-digit number with zero padding on the left.
    // `{:03}` means a width of 3, padded with zeros.
    format!("{:03}", val)
}