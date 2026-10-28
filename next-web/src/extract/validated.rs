use axum::extract::Query;
use axum::extract::rejection::QueryRejection;
use axum::http::StatusCode;
use axum::{
    extract::{
        Form, FromRequest, Json, Request,
        rejection::{FormRejection, JsonRejection},
    },
    response::{IntoResponse, Response},
};
use serde::de::DeserializeOwned;
use validator::Validate;

/// Validates a request body deserialized as a form.
#[derive(Debug, Clone, Copy, Default)]
pub struct ValidatedForm<T>(pub T);

/// Validates a request body deserialized as JSON.
#[derive(Debug, Clone, Copy, Default)]
pub struct ValidatedJson<T>(pub T);

/// Validates query parameters.
#[derive(Debug, Clone, Copy, Default)]
pub struct ValidatedQuery<T>(pub T);

impl<T, S> FromRequest<S> for ValidatedForm<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
    Form<T>: FromRequest<S, Rejection = FormRejection>,
{
    type Rejection = ValidationError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Form(value) = Form::<T>::from_request(req, state).await?;
        value.validate()?;
        Ok(ValidatedForm(value))
    }
}

impl<T, S> FromRequest<S> for ValidatedJson<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
    Json<T>: FromRequest<S, Rejection = JsonRejection>,
{
    type Rejection = ValidationError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(req, state).await?;
        value.validate()?;
        Ok(ValidatedJson(value))
    }
}

impl<T, S> FromRequest<S> for ValidatedQuery<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
    Query<T>: FromRequest<S, Rejection = QueryRejection>,
{
    type Rejection = ValidationError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Query(value) = Query::<T>::from_request(req, state).await?;
        value.validate()?;
        Ok(ValidatedQuery(value))
    }
}

/// Error returned when [`Validated`] fails to extract or validate the request.
#[derive(Debug)]
pub enum ValidationError {
    /// The value was successfully deserialized, but failed field validation.
    ValidationError(validator::ValidationErrors),

    /// The form body could not be deserialized.
    FormRejection(FormRejection),

    /// The JSON body could not be deserialized.
    JsonRejection(JsonRejection),

    /// The query string could not be deserialized.
    QueryRejection(QueryRejection),
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::ValidationError(e) => write!(f, "{e}"),
            ValidationError::FormRejection(e) => write!(f, "{e}"),
            ValidationError::JsonRejection(e) => write!(f, "{e}"),
            ValidationError::QueryRejection(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for ValidationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ValidationError::ValidationError(e) => Some(e),
            ValidationError::FormRejection(e) => Some(e),
            ValidationError::JsonRejection(e) => Some(e),
            ValidationError::QueryRejection(e) => Some(e),
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

impl From<QueryRejection> for ValidationError {
    fn from(err: QueryRejection) -> Self {
        ValidationError::QueryRejection(err)
    }
}

impl IntoResponse for ValidationError {
    fn into_response(self) -> Response {
        // Compute the message first so `self` is not borrowed after being matched.
        let message = match &self {
            ValidationError::ValidationError(_) => {
                format!("Input validation error: [{self}]").replace('\n', ", ")
            }
            ValidationError::FormRejection(_)
            | ValidationError::JsonRejection(_)
            | ValidationError::QueryRejection(_) => self.to_string(),
        };

        (StatusCode::BAD_REQUEST, message).into_response()
    }
}
