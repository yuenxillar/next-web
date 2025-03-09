use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

/**
*   struct:    ApiResponse
*   desc:      适用与API接口返回值统一处理
*   author:    Listening
*   email:     yuenxillar@163.com
*   date:      2024/10/02
*/

#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub status: u16,
    pub message: String,
    pub data: Option<T>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn set_status(mut self, status_code: StatusCode) -> ApiResponse<T> {
        self.status = status_code.as_u16();
        return self;
    }

    pub fn set_message(mut self, message: impl Into<String>) -> ApiResponse<T> {
        self.message = message.into();
        return self;
    }

    pub fn set_data(mut self, data: T) -> ApiResponse<T> {
        self.data = Some(data);
        return self;
    }

    pub fn set_msg(&mut self, msg: impl Into<String>) {
        self.message = msg.into();
    }

    pub fn new(status: StatusCode, message: String, data: T) -> Self {
        ApiResponse {
            status: status.as_u16(),
            message,
            data: Some(data),
        }
    }
}

impl<T: Serialize> ApiResponse<T> {
    pub fn ok(data: T) -> ApiResponse<T> {
        return ApiResponse {
            status: 200,
            message: "Ok".to_string(),
            data: Some(data),
        };
    }

    pub fn fail(data: T) -> ApiResponse<T> {
        return ApiResponse {
            status: 500,
            message: "Fail".to_string(),
            data: Some(data),
        };
    }
}

impl<T> IntoResponse for ApiResponse<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        Json(self).into_response()
    }
}
