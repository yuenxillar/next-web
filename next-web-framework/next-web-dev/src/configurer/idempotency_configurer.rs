

pub struct IdempotencyConfigurer {
    key: Option<Box<str>>,
    cache_key_prefix: Option<Box<str>>,
    ttl: Option<u64>
}