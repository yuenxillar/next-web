use std::error::Error;

use axum_core::response::{IntoResponse, Response};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApiError {
    // 服务器错误
    ServerError {
        status: u16,
        message: String,
    },
    // 客户端请求错误
    BadRequestError {
        field: Option<String>,
        message: String,
    },
    // 资源未找到
    NotFoundError {
        resource: String,
    },
    // 业务错误
    BusinessError {
        status: u16,
        message: String,
    },

    // 未授权访问
    UnauthorizedError {
        message: String,
    },
    // 禁止访问
    ForbiddenError {
        message: String,
    },
    // 操作超时
    TimeoutError {
        operation: String,
    },
    // 数据验证失败
    ValidationError {
        details: Vec<String>,
    },
    // 外部服务错误
    ExternalServiceError {
        service: String,
        message: String,
    },

    Created(String),
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiError::ServerError { status, message } => {
                write!(f, "Server Error [Code {}]: {}", status, message)
            }
            ApiError::BadRequestError { field, message } => {
                if let Some(field) = field {
                    write!(f, "Bad Request Error [Field '{}']: {}", field, message)
                } else {
                    write!(f, "Bad Request Error: {}", message)
                }
            }
            ApiError::NotFoundError { resource } => write!(f, "Resource '{}' not found", resource),
            ApiError::BusinessError { status, message } => {
                write!(f, "Business Error [Code {}]: {}", status, message)
            }
            ApiError::UnauthorizedError { message } => write!(f, "Unauthorized: {}", message),
            ApiError::ForbiddenError { message } => write!(f, "Forbidden: {}", message),
            ApiError::TimeoutError { operation } => {
                write!(f, "Operation '{}' timed out", operation)
            }

            ApiError::ValidationError { details } => {
                write!(f, "Validation failed: {}", details.join(", "))
            }
            ApiError::ExternalServiceError { service, message } => {
                write!(f, "External Service Error [{}]: {}", service, message)
            }

            ApiError::Created(message) => {
                write!(f, "Created Error: {}", message)
            }
        }
    }
}

impl std::error::Error for ApiError {}

impl ApiError {
    pub fn get_msg(&self) -> String {
        match self {
            ApiError::ServerError { message, .. } => message.into(),
            ApiError::BadRequestError { message, .. } => message.into(),
            ApiError::NotFoundError { resource } => resource.into(),
            ApiError::BusinessError { message, .. } => message.into(),
            ApiError::UnauthorizedError { message } => message.into(),
            ApiError::ForbiddenError { message } => message.into(),
            ApiError::TimeoutError { operation } => operation.into(),
            ApiError::ValidationError { details } => details.join(","),
            ApiError::ExternalServiceError { message, .. } => message.into(),
            ApiError::Created(message) => message.into(),
        }
    }

    pub fn with_context<T>(self, f: T) -> Self
    where
        T: FnOnce() -> String,
    {
        match self {
            ApiError::ServerError { status, message } => ApiError::ServerError {
                status,
                message: format!("{}; [with_context] {}", message, f()),
            },
            ApiError::BadRequestError { field, message } => ApiError::BadRequestError {
                field,
                message: format!("{}; [with_context] {}", message, f()),
            },
            ApiError::NotFoundError { resource } => ApiError::NotFoundError {
                resource: format!("{}; [with_context] {}", resource, f()),
            },
            ApiError::BusinessError { status, message } => ApiError::BusinessError {
                status,
                message: format!("{}; [with_context] {}", message, f()),
            },
            ApiError::UnauthorizedError { message } => ApiError::UnauthorizedError {
                message: format!("{}; [with_context] {}", message, f()),
            },
            ApiError::ForbiddenError { message } => ApiError::ForbiddenError {
                message: format!("{}; [with_context] {}", message, f()),
            },
            ApiError::TimeoutError { operation } => ApiError::TimeoutError {
                operation: format!("{}; [with_context] {}", operation, f()),
            },
            ApiError::ValidationError { mut details } => {
                details.push(format!("[with_context] {}", f()));
                ApiError::ValidationError { details }
            }
            ApiError::ExternalServiceError { service, message } => ApiError::ExternalServiceError {
                service,
                message: format!("{}; [with_context] {}", message, f()),
            },
            ApiError::Created(message) => {
                ApiError::Created(format!("{}; [with_context] {}", message, f()))
            }
        }
    }
}

impl From<Box<dyn Error>> for ApiError {
    fn from(err: Box<dyn Error>) -> Self {
        ApiError::Created(err.to_string())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (http_code, status, error_message) = match self {
            ApiError::ServerError { status, message } => (500, status, message),
            ApiError::BadRequestError { field, message } => {
                if let Some(field) = field {
                    (
                        400,
                        400,
                        format!("Bad Request Error [Field '{}']: {}", field, message),
                    )
                } else {
                    (400, 400, format!("Bad Request Error: {}", message))
                }
            }
            ApiError::NotFoundError { resource } => {
                (404, 404, format!("Resource '{}' not found", resource))
            }
            ApiError::BusinessError { status, message } => (
                400,
                status,
                format!("Business Error [Code {}]: {}", status, message),
            ),
            ApiError::UnauthorizedError { message } => {
                (401, 401, format!("Unauthorized: {}", message))
            }
            ApiError::ForbiddenError { message } => (403, 403, format!("Forbidden: {}", message)),
            ApiError::TimeoutError { operation } => {
                (408, 408, format!("Operation '{}' timed out", operation))
            }
            ApiError::ValidationError { details } => (
                422,
                422,
                format!("Validation failed: {}", details.join(", ")),
            ),
            ApiError::ExternalServiceError { service, message } => (
                503,
                503,
                format!("External Service Error [{}]: {}", service, message),
            ),
            ApiError::Created(message) => (500, 500, message),
        };

        Response::builder()
            .status(http_code)
            .header("Content-Type", "application/json")
            .body(
                format!("{{\"status\": {status},\"message\": {error_message},\"data\": null}}")
                    .into(),
            )
            .unwrap()
    }
}
