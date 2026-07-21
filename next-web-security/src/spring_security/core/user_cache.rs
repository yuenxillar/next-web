use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use crate::core::userdetails::UserDetails;

pub trait UserCache: Send + Sync {
    fn get_user_from_cache(&self, username: &str) -> Option<Arc<dyn UserDetails>>;

    fn put_user_in_cache(&self, username: String, user: Arc<dyn UserDetails>);

    fn remove_user_from_cache(&self, username: &str);
}

#[derive(Default)]
pub struct NullUserCache;

impl UserCache for NullUserCache {
    fn get_user_from_cache(&self, _username: &str) -> Option<Arc<dyn UserDetails>> {
        None
    }

    fn put_user_in_cache(&self, _username: String, _user: Arc<dyn UserDetails>) {}

    fn remove_user_from_cache(&self, _username: &str) {}
}

#[derive(Default)]
pub struct InMemoryUserCache {
    users: RwLock<HashMap<String, Arc<dyn UserDetails>>>,
}

impl UserCache for InMemoryUserCache {
    fn get_user_from_cache(&self, username: &str) -> Option<Arc<dyn UserDetails>> {
        self.users.read().ok()?.get(username).cloned()
    }

    fn put_user_in_cache(&self, username: String, user: Arc<dyn UserDetails>) {
        if let Ok(mut users) = self.users.write() {
            users.insert(username, user);
        }
    }

    fn remove_user_from_cache(&self, username: &str) {
        if let Ok(mut users) = self.users.write() {
            users.remove(username);
        }
    }
}
