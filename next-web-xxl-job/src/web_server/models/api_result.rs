use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};

pub const SUCCESS_CODE: i32 = 200;
pub const FAIL_CODE: i32 = 500;

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct XxlApiResult<T> {
    pub content: Option<T>,
    pub code: i32,
    pub msg: Option<String>,
}

impl<T> XxlApiResult<T>
where
    T: Default,
{
    pub fn success(content: Option<T>) -> XxlApiResult<T> {
        Self {
            content,
            code: SUCCESS_CODE,
            msg: None,
        }
    }

    pub fn fail(msg: Option<String>) -> XxlApiResult<T> {
        Self {
            content: None,
            code: FAIL_CODE,
            msg,
        }
    }

    pub fn is_success(&self) -> bool {
        self.code == SUCCESS_CODE
    }
}

impl<T> IntoResponse for XxlApiResult<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        Response::builder()
            .status(200)
            .body(serde_json::to_string(&self).unwrap_or_default().into())
            .unwrap()
    }
}
