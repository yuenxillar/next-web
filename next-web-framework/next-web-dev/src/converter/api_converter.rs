use crate::{application::api::api_response::ApiResponse, error::api_error::ApiError};

pub trait Converter<T: serde::Serialize> {
    type Output: serde::Serialize;

    fn into_api_result(self) -> Result<ApiResponse<Self::Output>, ApiError>;
}

impl<T: serde::Serialize> Converter<T> for bool {
    type Output = bool;

    fn into_api_result(self) -> Result<ApiResponse<Self::Output>, ApiError> {
        match self {
            true => Ok(ApiResponse::ok(true)),
            false => Err(ApiError::BusinessError(String::new())),
        }
    }
}

impl<T: serde::Serialize> Converter<T> for Option<T> {
    type Output = Option<T>;

    fn into_api_result(self) -> Result<ApiResponse<Self::Output>, ApiError> {
        match self {
            Some(var) => Ok(ApiResponse::ok(Some(var))),
            None => Err(ApiError::BusinessError(String::new())),
        }
    }
}

impl<T: serde::Serialize + Sync + Send> Converter<T> for rbatis::Page<T> {
    type Output = rbatis::Page<T>;

    fn into_api_result(self) -> Result<ApiResponse<Self::Output>, ApiError> {
        Ok(ApiResponse::ok(self))
    }
}
