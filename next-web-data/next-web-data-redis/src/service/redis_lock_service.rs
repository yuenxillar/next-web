use std::ops::Deref;

use next_web_core::core::service::Service;
use rslock::LockManager;

#[derive(Clone)]
pub struct RedisLockService {
    lock: LockManager
}

impl RedisLockService {
    
    pub fn new(urls: Vec<String>) -> Self {
        Self {
            lock: LockManager::new(urls)
        }
    }
}

impl Service for RedisLockService {}


impl Deref  for  RedisLockService {
    type Target = LockManager;

    fn deref(&self) -> &Self::Target {
        &self.lock
    }
}