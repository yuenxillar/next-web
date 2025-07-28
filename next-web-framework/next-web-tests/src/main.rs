#![allow(missing_docs)]
use std::sync::Arc;

use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::Router;
use next_web_core::async_trait;
use next_web_core::error::BoxError;
use next_web_core::interface::data_decoder::DataDecoder;
use next_web_core::interface::job::application_job::ApplicationJob;
use next_web_core::interface::job::context::job_execution_context::JobExecutionContext;
use next_web_core::interface::job::schedule_type::ScheduleType;
use next_web_core::interface::service::Service;
use next_web_core::{context::properties::ApplicationProperties, ApplicationContext};

use next_web_dev::application::Application;
use next_web_dev::event::default_application_event_publisher::DefaultApplicationEventPublisher;
use next_web_dev::interceptor::request_data_interceptor::Data;
use next_web_dev::stream::local_file_stream::LocalFileStream;
use next_web_dev::stream::response_stream::ResponseStream;
use next_web_dev::Singleton;
use serde::{Deserialize, Serialize};

// mod test_mqtt;
// mod test_redis;
// mod test_websocket;

/// Test application
#[derive(Default, Clone)]
pub struct TestApplication;

#[async_trait]
impl Application for TestApplication {
    /// initialize the middleware.
    async fn init_middleware(&mut self, _properties: &ApplicationProperties) {}

    // get the application router. (open api  and private api)
    async fn application_router(&mut self, ctx: &mut ApplicationContext) -> axum::Router {
        ctx.get_single_with_name::<String>("test");
        Router::new()
            .route("/test/789", post(test_789))
            .route("/download", get(download))
    }
}

async fn download() -> impl IntoResponse {
    return ResponseStream::new(LocalFileStream("".into()));
}

#[Singleton(binds=[Self::into_job])]
#[derive(Clone)]
pub struct TestJob;

impl TestJob {
    fn into_job(self) -> Arc<dyn ApplicationJob> {
        Arc::new(self)
    }
}

#[async_trait]
impl ApplicationJob for TestJob {
    fn schedule(&self) -> ScheduleType {
        ScheduleType::Repeated(1500)
    }

    async fn execute(&self, _context: JobExecutionContext) -> Result<(), BoxError> {
        println!("我正在执行任务!");
        Ok(())
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct TestData {
    pub name: String,
    pub age: i32,
}

async fn test_789(Data(data): Data<TestData>) -> String {
    serde_json::to_string(&data).unwrap()
}

#[Singleton(binds=[Self::into_decoder])]
#[derive(Clone)]
pub struct TestDecoder;

impl TestDecoder {
    fn into_decoder(self) -> Arc<dyn DataDecoder> {
        Arc::new(self)
    }
}

impl DataDecoder for TestDecoder {
    fn decode(&self, data: &[u8]) -> Result<String, &'static str> {
        let d = data
            .iter()
            .filter(|&&s| s != b'\\')
            .copied()
            .collect::<Vec<_>>();
        Ok(String::from_utf8_lossy(&d).to_string())
    }
}

#[Singleton]
#[derive(Clone)]
pub struct TestModule {
    pub publisher: DefaultApplicationEventPublisher,
}

impl Service for TestModule {}


#[tokio::main]
async fn main() {
    TestApplication::run().await;
}