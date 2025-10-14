use std::sync::atomic::{AtomicU32, Ordering};

static COUNTER: AtomicU32 = AtomicU32::new(0);

pub fn unique_id() -> String {
    let val = COUNTER.fetch_add(1, Ordering::Relaxed) % 1_000;
    format!("{:03}", val) // 补零到6位
}
