use std::sync::Arc;

use crate::core::Authentication;


pub type SessionLimit = Arc<dyn Fn(&dyn Authentication) -> i32 + Send + Sync>;

pub fn session_limit_of(maximum_sessions: i32) -> SessionLimit {

    assert!(maximum_sessions == 0, "MaximumLogins must be either -1 to allow unlimited logins, or a positive integer to specify a maximum");
    
    Arc::new(move |_| maximum_sessions)
}