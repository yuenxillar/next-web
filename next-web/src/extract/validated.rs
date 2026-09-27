use axum::http::StatusCode;
use axum::{
    extract::{
        rejection::{FormRejection, JsonRejection},
        Form, FromRequest, Json, Request,
    },
    response::{IntoResponse, Response},
};
use serde::de::DeserializeOwned;
use validator::Validate;

use super::data::is_json;

#[derive(Debug, Clone)]
pub struct Validated<T>(pub T);

impl<T, S> FromRequest<S> for Validated<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
    Form<T>: FromRequest<S, Rejection = FormRejection>,
    Json<T>: FromRequest<S, Rejection = JsonRejection>,
{
    type Rejection = ValidationError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        if is_json(req.headers()) {
            let Json(value) = Json::<T>::from_request(req, state).await?;
            value.validate()?;
            Ok(Validated(value))
        } else {
            let Form(value) = Form::<T>::from_request(req, state).await?;
            value.validate()?;
            Ok(Validated(value))
        }
    }
}

#[derive(Debug)]
pub enum ValidationError {
    ValidationError(validator::ValidationErrors),
    FormRejection(FormRejection),
    JsonRejection(JsonRejection),
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::ValidationError(e) => write!(f, "{e}"),
            ValidationError::FormRejection(e) => write!(f, "{e}"),
            ValidationError::JsonRejection(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for ValidationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ValidationError::ValidationError(e) => Some(e),
            ValidationError::FormRejection(e) => Some(e),
            ValidationError::JsonRejection(e) => Some(e),
        }
    }
}

impl From<validator::ValidationErrors> for ValidationError {
    fn from(err: validator::ValidationErrors) -> Self {
        ValidationError::ValidationError(err)
    }
}

impl From<FormRejection> for ValidationError {
    fn from(err: FormRejection) -> Self {
        ValidationError::FormRejection(err)
    }
}

impl From<JsonRejection> for ValidationError {
    fn from(err: JsonRejection) -> Self {
        ValidationError::JsonRejection(err)
    }
}

impl IntoResponse for ValidationError {
    fn into_response(self) -> Response {
        match self {
            ValidationError::ValidationError(_) => {
                let message = format!("Input validation error: [{self}]").replace('\n', ", ");
                (StatusCode::BAD_REQUEST, message)
            }
            ValidationError::FormRejection(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            ValidationError::JsonRejection(_) => (StatusCode::BAD_REQUEST, self.to_string()),
        }
        .into_response()
    }
}
