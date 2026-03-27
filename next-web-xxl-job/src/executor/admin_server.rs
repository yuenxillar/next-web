use axum::BoxError;

use crate::client::admin_client::AdminClient;
use crate::executor::models::{ServerAccessActorReq, ServerAccessActorResult};
use crate::utils::now_millis_i64;
use crate::web_server::config::client_config::ClientConfig;
use crate::web_server::models::admin_req::CallbackParam;
use crate::web_server::models::api_result::SUCCESS_CODE;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

#[derive(Clone)]
pub struct ServerAccessActor {
    admin_client: Arc<AdminClient>,
    running: Arc<AtomicBool>,
}

impl ServerAccessActor {
    pub fn new(client_config: Arc<ClientConfig>) -> Self {
        let admin_client = Arc::new(AdminClient::new(client_config).unwrap());
        Self {
            admin_client,
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    pub async fn start(&self) {
        self.running.store(true, Ordering::Relaxed);
        self.do_heartbeat().await;
        self.registry_heartbeat();
    }

    fn registry_heartbeat(&self) {
        let actor = self.clone();
        tokio::spawn(async move {
            loop {
                if !actor.running.load(Ordering::Relaxed) {
                    return;
                }

                actor.do_heartbeat().await;
                tokio::time::sleep(Duration::from_millis(29500)).await;
            }
        });
    }

    async fn do_heartbeat(&self) {
        let _ = self.admin_client.registry().await;
    }

    async fn stop(&self) {
        let _ = self.admin_client.registry_remove().await;

        self.running.store(false, Ordering::Relaxed);
    }

    async fn callback(&self, params: Vec<CallbackParam>) {
        let mut i = 2u16;
        // 失败最多尝试重试10次
        while i < 12u16 && self.admin_client.callback(&params).await.is_err() {
            tokio::time::sleep(Duration::from_secs((i * i) as u64)).await;
            i += 1;
        }
    }

    pub(crate) async fn do_send(
        &self,
        msg: ServerAccessActorReq,
    ) -> Result<ServerAccessActorResult, BoxError> {
        match msg {
            ServerAccessActorReq::Stop => {
                self.stop().await;
            }
            ServerAccessActorReq::CallBack(params) => {
                self.callback(params).await;
            }
        };
        Ok(ServerAccessActorResult::None)
    }
}

pub(crate) async fn do_callback(
    server_access_actor: &ServerAccessActor,
    param: Vec<CallbackParam>,
) {
    let _ = server_access_actor
        .do_send(ServerAccessActorReq::CallBack(param))
        .await;
}

pub async fn callback_success(server_access_actor: &ServerAccessActor, log_id: u64) {
    let callback_param = CallbackParam {
        log_id,
        log_date_tim: now_millis_i64(),
        handle_code: SUCCESS_CODE,
        handle_msg: None,
    };
    do_callback(server_access_actor, vec![callback_param]).await;
}

pub async fn callback(
    server_access_actor: &ServerAccessActor,
    log_id: u64,
    handle_code: i32,
    handle_msg: Option<String>,
) {
    let callback_param = CallbackParam {
        log_id,
        log_date_tim: now_millis_i64(),
        handle_code,
        handle_msg,
    };
    do_callback(server_access_actor, vec![callback_param]).await;
}
