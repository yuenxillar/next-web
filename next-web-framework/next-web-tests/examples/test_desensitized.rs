use std::borrow::Cow;

use axum::{
    body::Body,
    http::{Response, StatusCode},
    response::IntoResponse,
};
use next_web_core::{
    async_trait, context::properties::ApplicationProperties, traits::desensitized::Desensitized,
    ApplicationContext,
};
use next_web_dev::application::Application;
use next_web_macro::{Desensitized, GetSet};

#[derive(Clone, Default)]
struct TestApplication;

#[async_trait]
impl Application for TestApplication {
    /// initialize the middleware.
    async fn init_middleware(&mut self, _properties: &ApplicationProperties) {}

    // get the application router. (open api  and private api)
    async fn application_router(&mut self, _ctx: &mut ApplicationContext) -> axum::Router {
        axum::Router::new().route("/desensitized", axum::routing::get(test))
    }
}

async fn test() -> impl IntoResponse {
    ApiResult {
        code: 200,
        message: "Success".to_string(),
        data: Some(TestA {
            email: "test_desensitized@163.com".into(),
            phone: Some("17699935688".into()),
            name: "张三".into(),
        }),
    }
}

#[derive(serde::Serialize)]
struct ApiResult<T> {
    code: i32,
    message: String,
    data: Option<T>,
}

impl<T> IntoResponse for ApiResult<T>
where
    T: serde::Serialize + Desensitized,
{
    fn into_response(mut self) -> axum::response::Response {
        self.data.as_mut().map(|val| val.desensitize());
        Response::builder()
            .status(StatusCode::OK)
            .body(Body::new(serde_json::to_string(&self).unwrap()))
            .unwrap()
    }
}


#[derive(Default, serde::Serialize, GetSet, Desensitized)]
struct TestA {
    #[de(email)]
    email: String,
    #[de(phone)]
    phone: Option<Box<str>>,
    #[de(name)]
    name: Cow<'static, str>,
}

#[tokio::main]
async fn main() {
    TestApplication::run().await;
}
