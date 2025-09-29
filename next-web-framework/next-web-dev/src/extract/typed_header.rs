//! Extractor and response for typed headers.

use std::convert::Infallible;

use headers::{Header, HeaderMapExt};
use axum::extract::{FromRequestParts, OptionalFromRequestParts};
use axum::http::{HeaderName, StatusCode};
use axum::{http::request::Parts, response::{IntoResponse, IntoResponseParts, Response, ResponseParts}};

/// Extractor and response that works with typed header values from [`headers`].
///
/// # As extractor
///
/// In general, it's recommended to extract only the needed headers via `TypedHeader` rather than
/// removing all headers with the `HeaderMap` extractor.
///
/// ```rust,no_run
/// use axum::{
///     routing::get,
///     Router,
/// };
/// use headers::UserAgent;
/// use axum_extra::TypedHeader;
///
/// async fn users_teams_show(
///     TypedHeader(user_agent): TypedHeader<UserAgent>,
/// ) {
///     // ...
/// }
///
/// let app = Router::new().route("/users/{user_id}/team/{team_id}", get(users_teams_show));
/// # let _: Router = app;
/// ```
///
/// # As response
///
/// ```rust
/// use axum::{
///     response::IntoResponse,
/// };
/// use headers::ContentType;
/// use axum_extra::TypedHeader;
///
/// async fn handler() -> (TypedHeader<ContentType>, &'static str) {
///     (
///         TypedHeader(ContentType::text_utf8()),
///         "Hello, World!",
///     )
/// }
/// ```
#[derive(Debug, Clone, Copy)]
#[must_use]
pub struct TypedHeader<T>(pub T);

impl<T, S> FromRequestParts<S> for TypedHeader<T>
where
    T: Header,
    S: Send + Sync,
{
    type Rejection = TypedHeaderRejection;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let mut values = parts.headers.get_all(T::name()).iter();
        let is_missing = values.size_hint() == (0, Some(0));
        T::decode(&mut values)
            .map(Self)
            .map_err(|err| TypedHeaderRejection {
                name: T::name(),
                reason: if is_missing {
                    // Report a more precise rejection for the missing header case.
                    TypedHeaderRejectionReason::Missing
                } else {
                    TypedHeaderRejectionReason::Error(err)
                },
            })
    }
}

impl<T, S> OptionalFromRequestParts<S> for TypedHeader<T>
where
    T: Header,
    S: Send + Sync,
{
    type Rejection = TypedHeaderRejection;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> Result<Option<Self>, Self::Rejection> {
        let mut values = parts.headers.get_all(T::name()).iter();
        let is_missing = values.size_hint() == (0, Some(0));
        match T::decode(&mut values) {
            Ok(res) => Ok(Some(Self(res))),
            Err(_) if is_missing => Ok(None),
            Err(err) => Err(TypedHeaderRejection {
                name: T::name(),
                reason: TypedHeaderRejectionReason::Error(err),
            }),
        }
    }
}

axum_core::__impl_deref!(TypedHeader);

impl<T> IntoResponseParts for TypedHeader<T>
where
    T: Header,
{
    type Error = Infallible;

    fn into_response_parts(self, mut res: ResponseParts) -> Result<ResponseParts, Self::Error> {
        res.headers_mut().typed_insert(self.0);
        Ok(res)
    }
}

impl<T> IntoResponse for TypedHeader<T>
where
    T: Header,
{
    fn into_response(self) -> Response {
        let mut res = ().into_response();
        res.headers_mut().typed_insert(self.0);
        res
    }
}

/// Rejection used for [`TypedHeader`].
#[derive(Debug)]
pub struct TypedHeaderRejection {
    name: &'static HeaderName,
    reason: TypedHeaderRejectionReason,
}

impl TypedHeaderRejection {
    /// Name of the header that caused the rejection
    #[must_use]
    pub fn name(&self) -> &HeaderName {
        self.name
    }

    /// Reason why the header extraction has failed
    #[must_use]
    pub fn reason(&self) -> &TypedHeaderRejectionReason {
        &self.reason
    }

    /// Returns `true` if the typed header rejection reason is [`Missing`].
    ///
    /// [`Missing`]: TypedHeaderRejectionReason::Missing
    #[must_use]
    pub fn is_missing(&self) -> bool {
        self.reason.is_missing()
    }
}

/// Additional information regarding a [`TypedHeaderRejection`]
#[derive(Debug)]
#[non_exhaustive]
pub enum TypedHeaderRejectionReason {
    /// The header was missing from the HTTP request
    Missing,
    /// An error occurred when parsing the header from the HTTP request
    Error(headers::Error),
}

impl TypedHeaderRejectionReason {
    /// Returns `true` if the typed header rejection reason is [`Missing`].
    ///
    /// [`Missing`]: TypedHeaderRejectionReason::Missing
    #[must_use]
    pub fn is_missing(&self) -> bool {
        matches!(self, Self::Missing)
    }
}

impl IntoResponse for TypedHeaderRejection {
    fn into_response(self) -> Response {
        let status = StatusCode::BAD_REQUEST;
        let body = self.to_string();
        // axum_core::__log_rejection!(rejection_type = Self, body_text = body, status = status,);
        (status, body).into_response()
    }
}

impl std::fmt::Display for TypedHeaderRejection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.reason {
            TypedHeaderRejectionReason::Missing => {
                write!(f, "Header of type `{}` was missing", self.name)
            }
            TypedHeaderRejectionReason::Error(err) => {
                write!(f, "{err} ({})", self.name)
            }
        }
    }
}

impl std::error::Error for TypedHeaderRejection {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.reason {
            TypedHeaderRejectionReason::Error(err) => Some(err),
            TypedHeaderRejectionReason::Missing => None,
        }
    }
}