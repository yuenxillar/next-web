use std::sync::Arc;

use tokio::sync::RwLock;

#[derive(Clone)]
pub struct SyncArray<T> {
    pub(crate) data: Arc<RwLock<Vec<T>>>,
}

impl<T> SyncArray<T> where T: Clone {}
