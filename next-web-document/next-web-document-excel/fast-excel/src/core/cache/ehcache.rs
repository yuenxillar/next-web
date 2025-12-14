#[derive(Clone)]
pub struct EhCache {
    activeIndex: usize,
    dataList: Vec<String>,

    cache_alias: Option<String>,
    cache_miss: u32,
}

impl EhCache {
    pub const BATCH_COUNT: u32 = 100;
    pub const DEBUG_CACHE_MISS_SIZE: u32 = 1000;
    pub const DEBUG_WRITE_SIZE: u32 = 1000;
}
