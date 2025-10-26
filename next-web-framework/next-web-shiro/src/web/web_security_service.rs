use std::sync::Arc;

use crate::core::mgt::security_manager::SecurityManager;

#[derive(Clone)]
pub struct WebSecurityService {
    security_manager:  Arc<dyn SecurityManager>,
    http_filter:       Vec<Arc<dyn HttpFilter>>,
}

