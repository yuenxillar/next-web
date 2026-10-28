use std::sync::Arc;

use crate::{
    executor::{admin_server::ServerAccessActor, executor_actor::ExecutorActor},
    web_server::config::client_config::ClientConfig,
};

#[derive(Clone)]
pub struct XxlJobAppState {
    pub executor_actor: ExecutorActor,
    pub server_access_actor: ServerAccessActor,
    pub client_config: Arc<ClientConfig>,
}
