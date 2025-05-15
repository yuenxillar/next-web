#![allow(missing_docs)]

use std::error::Error;
use std::sync::Arc;

use axum::extract::ws::CloseFrame;
use axum::extract::State;
use axum::routing::get;
use axum::Router;
use next_web_core::async_trait;
use next_web_core::{context::properties::ApplicationProperties, ApplicationContext};
use next_web_data_database::service::database_service::DatabaseService;
use next_web_data_mongodb::{doc, Document};
use next_web_data_mongodb::service::mongodb_service::MongodbService;
use next_web_data_redis::core::event::expired_keys_event::RedisExpiredKeysEvent;
use next_web_data_redis::service::redis_service::RedisService;
use next_web_dev::{
    application::Application,
    router::{open_router::OpenRouter, private_router::PrivateRouter},
    Singleton,
};
use next_web_mqtt::core::topic::base_topic::BaseTopic;
use next_web_websocket::core::handler::WebSocketHandler;
use next_web_websocket::core::session::WebSocketSession;

/// Test application
#[derive(Default, Clone)]
pub struct TestApplication;

#[async_trait]
impl Application for TestApplication {
    /// initialize the middleware.
    async fn init_middleware(&mut self, _properties: &ApplicationProperties) {}

    // get the application router. (open api  and private api)
    async fn application_router(
        &mut self,
        ctx: &mut ApplicationContext,
    ) -> (OpenRouter, PrivateRouter) {
        let service = ctx
            .get_single_with_name::<DatabaseService>("databaseService")
            .to_owned();
        let interface = Router::new()
            .route("/test_api", get(test_api))
            .with_state(service);
        (OpenRouter::default(), PrivateRouter(interface))
    }
}

async fn test_api(State(service): State<DatabaseService>) -> impl axum::response::IntoResponse {
    let version: String = service
        .query_decode("SELECT VERSION();", vec![])
        .await
        .unwrap();
    version
}

#[Singleton( binds = [Self::into_base_topic])]
#[derive(Clone)]
pub struct TestOneBaseTopic;

impl TestOneBaseTopic {
    fn into_base_topic(self) -> Box<dyn BaseTopic> {
        Box::new(self)
    }
}

#[Singleton( binds = [Self::into_base_topic])]
#[derive(Clone)]
pub struct TestTwoBaseTopic;

impl TestTwoBaseTopic {
    fn into_base_topic(self) -> Box<dyn BaseTopic> {
        Box::new(self)
    }
}

#[async_trait]
impl BaseTopic for TestOneBaseTopic {
    fn topic(&self) -> &'static str {
        "test/+/event"
    }

    async fn consume(&self, topic: &str, message: &[u8]) {
        println!(
            "Received message0, Topic: {}, Data Content: {:?}",
            topic,
            String::from_utf8_lossy(message)
        );
    }
}

#[async_trait]
impl BaseTopic for TestTwoBaseTopic {
    fn topic(&self) -> &'static str {
        "test/#"
    }

    async fn consume(&self, topic: &str, message: &[u8]) {
        println!(
            "Received message1, Topic: {}, Data Content: {:?}",
            topic,
            String::from_utf8_lossy(message)
        );
    }
}

/// Test
#[Singleton(binds = [Self::into_websocket_handler])]
#[derive(Clone)]
pub struct TestWebSocket {
    #[autowired(name = "databaseService")]
    pub database_service: DatabaseService,
    #[autowired(name = "redisService")]
    pub redis_service: RedisService,
    #[autowired(name = "mongodbService")]
    pub mongdob_service: MongodbService,
}

impl TestWebSocket {
    fn into_websocket_handler(self) -> Arc<dyn WebSocketHandler> {
        Arc::new(self)
    }
}

use next_web_data_redis::AsyncCommands;
use next_web_websocket::core::handler::Result;
use next_web_websocket::Message;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
struct User {
    pub id: u64,
    pub username: String,
    pub password: String,
    pub salt: String,
    pub nickname: Option<String>,
    pub email: String,
    pub name: String,
}

#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
struct TestUser {
    pub id: u64,
}

#[async_trait]
impl WebSocketHandler for TestWebSocket {
    fn paths(&self) -> Vec<&'static str> {
        vec!["/test1/websocket", "/test1/websocket2"]
    }

    // When the socket connection enters, this method will be entered first
    async fn on_open(&self, session: &WebSocketSession) -> Result<()> {
        println!(
            "Client remote address: {:?}, Session id: {:?}, Client header: {:?}, Client paths: {:?}",
            session.remote_address(),
            session.id(),
            session.header(),
            session.path());
        Ok(())
    }

    /// When the client sends a message, it will enter the following method
    async fn on_message(&self, session: &WebSocketSession, message: Message) -> Result<()> {
        if let Message::Text(msg) = message {
            if msg.contains("test") {
                let result: Vec<User> = self
                    .database_service
                    .query_decode("SELECT * FROM `user`", vec![])
                    .await?;
                println!("users: {:?}", result);
                let _ = session
                    .send_message(Message::Text(serde_json::to_string(&result)?.into()))
                    .await;
            } else if msg.contains("redis") {
                let mut con = self.redis_service.get_connection().await.unwrap();
                let _: () = con.set(msg.to_string(), 42).await.unwrap();
            } else if msg.contains("mongodb") {
                if let Some(db) = self.mongdob_service.default_database() {
                    let collection =
                        db.collection::<TestUser>("user_media_log");
                    let docs = vec![
                        doc! { "title": "1984", "author": "George Orwell" },
                        doc! { "title": "Animal Farm", "author": "George Orwell" },
                        doc! { "title": "The Great Gatsby", "author": "F. Scott Fitzgerald" },
                    ];
                    // Document::new().get_object_id(key)
                    // let var = collection.update_one(filter).await;
                    // let var = collection.insert_many(&vec![TestUser { id: 6666 }]).await;
                    // let var = collection.delete_many(query).await;
                    // let var = collection.count_documents(filter).await;
                }
            }
        }
        Ok(())
    }

    /// When an error occurs during the connection process or message transmission, the following methods will be executed
    async fn on_error(
        &self,
        _session: &WebSocketSession,
        error: Box<dyn Error + Send + Sync>,
    ) -> Result<()> {
        println!("On error: {:#}", error);
        Ok(())
    }

    /// After handling the error, close the connection and proceed to the following method
    async fn on_close(&self, session: &WebSocketSession, _close: Option<CloseFrame>) -> Result<()> {
        println!("User left: {:?}", session.id());
        Ok(())
    }
}

#[Singleton(binds = [Self::into_expired_key_listener])]
#[derive(Clone)]
pub struct TestExpiredKeyListener;

impl TestExpiredKeyListener {
    fn into_expired_key_listener(self) -> Box<dyn RedisExpiredKeysEvent> {
        Box::new(self)
    }
}

#[async_trait]
impl RedisExpiredKeysEvent for TestExpiredKeyListener {
    async fn on_message(&mut self, message: &[u8], pattern: &[u8]) {
        println!(
            "Expired key: {}, pattern: {}",
            String::from_utf8_lossy(message),
            String::from_utf8_lossy(pattern)
        );
    }
}

#[tokio::main]
async fn main() {
    TestApplication::run().await;
}
