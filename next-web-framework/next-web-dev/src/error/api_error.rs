use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use crate::application::api::api_response::ApiResponse;

// This is the implementation of the ApiException enum.
#[derive(Debug, Clone)]
pub enum ApiError {
    ServerError,
    BadRequestError,
    NotFoundError,

    BusinessError(String),
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[ApiError] msg: {}", self.get_msg())
    }
}

impl std::error::Error for ApiError {}

impl ApiError {
    pub fn get_msg(&self) -> &str {
        match self {
            ApiError::BusinessError(msg) => msg,
            _ => "",
        }
    }

    pub fn with_context<T>(self, f: T) -> Self
    where
        T: FnOnce() -> String,
    {
        match self {
            ApiError::BusinessError(msg) => {
                ApiError::BusinessError(format!("with context: {}\n{}", f(), msg))
            }
            _ => self,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            ApiError::ServerError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                String::from("Internal server error"),
            ),
            ApiError::BadRequestError => (StatusCode::BAD_REQUEST, String::from("Bad request")),
            ApiError::NotFoundError => (StatusCode::NOT_FOUND, String::from("Resource not found")),

            ApiError::BusinessError(msg) => {
                (StatusCode::OK, format!("Business error, cause: {}", msg))
            }
        };
        (
            status,
            ApiResponse::fail(())
                .set_status(status)
                .set_message(error_message),
        )
            .into_response()
    }
}
