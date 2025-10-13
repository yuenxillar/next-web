use axum::extract::FromRequestParts;
use axum::{http::request::Parts, response::{IntoResponse, IntoResponseParts, Response, ResponseParts}};

/// 保证接口幂等性
#[derive(Debug, Clone)]
pub struct IdempotencyKey<T>(pub T);

impl<T, S> FromRequestParts<S> for IdempotencyKey<T>
where
    T: Header,
    S: Send + Sync,
{
    type Rejection = IdempotencyKeyRejection;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let mut values = parts.headers.get_all(T::name()).iter();
        let is_missing = values.size_hint() == (0, Some(0));
        T::decode(&mut values)
            .map(Self)
            .map_err(|err| IdempotencyKeyRejection {
                name: T::name(),
                reason: if is_missing {
                    // Report a more precise rejection for the missing header case.
                    IdempotencyKeyRejectionReason::Missing
                } else {
                    IdempotencyKeyRejectionReason::Error(err)
                },
            })
    }
}

#[derive(Debug)]
pub struct IdempotencyKeyRejection {

}

#[derive(Debug)]
#[non_exhaustive]
pub enum IdempotencyKeyRejectionReason {
    /// The header was missing from the HTTP request
    Missing,
    /// An error occurred when parsing the header from the HTTP request
    Error(headers::Error),
}


axum_core::__impl_deref!(IdempotencyKey);


