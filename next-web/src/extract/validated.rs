use axum::http::StatusCode;
use axum::{
    extract::{
        rejection::{FormRejection, JsonRejection},
        Form, FromRequest, Json, Request,
    },
    response::{IntoResponse, Response},
};
use serde::de::DeserializeOwned;
use thiserror::Error;
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

#[derive(Debug, Error)]
pub enum ValidationError {
    #[error(transparent)]
    ValidationError(#[from] validator::ValidationErrors),

    #[error(transparent)]
    FormRejection(#[from] FormRejection),

    #[error(transparent)]
    JsonRejection(#[from] JsonRejection),
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
