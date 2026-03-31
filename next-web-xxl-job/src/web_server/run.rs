use std::sync::Arc;

use next_web_core::{
    async_trait,
    traits::service::background_service::{BackgroundService, HealthStatus, ServiceError},
};
use rudi_dev::singleton;

use crate::web_server::{open_api::app, state::XxlJobAppState};

pub static DEFAULT_XXL_JOB_TOP_BANNER: &str = r#"
___   ___ ___   ___  __                     __    ______   .______
\  \ /  / \  \ /  / |  |                   |  |  /  __  \  |   _  \
 \  V  /   \  V  /  |  |      ______       |  | |  |  |  | |  |_)  |
  >   <     >   <   |  |     |______|.--.  |  | |  |  |  | |   _  <
 /  .  \   /  .  \  |  `----.        |  `--'  | |  `--'  | |  |_)  |
/__/ \__\ /__/ \__\ |_______|         \______/   \______/  |______/
"#;

#[derive(Clone)]
#[singleton(binds = [Self::into_background_service])]
pub struct XxlWebServer {
    #[autowired(name = "xxlJobAppState")]
    app_state: Arc<XxlJobAppState>,
}

impl XxlWebServer {
    fn into_background_service(self: Self) -> Arc<dyn BackgroundService> {
        Arc::new(self)
    }
}

#[async_trait]
impl BackgroundService for XxlWebServer {
    fn service_name(&self) -> &'static str {
        "XxlJobWebServer"
    }

    async fn run(&self) -> Result<(), ServiceError> {
        let addr = self.app_state.client_config.get_http_addr();

        let listener = tokio::net::TcpListener::bind(addr.to_owned())
            .await
            .unwrap();

        let app = app(self.app_state.clone());

        println!("{}", DEFAULT_XXL_JOB_TOP_BANNER);
        println!("XxlJob Web Application Starting up, Listening on: {}", addr);

        axum::serve(listener, app.into_make_service())
            .await
            .unwrap();

        Ok(())
    }

    async fn shutdown(&self) {}

    async fn health_check(&self) -> HealthStatus {
        HealthStatus::Healthy
    }
}
