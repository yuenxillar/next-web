use axum::{
    extract::{FromRef, FromRequestParts},
    http::{request::Parts, StatusCode},
};

pub struct FindSingleton<T>(Option<T>);

impl<T> Clone for FindSingleton<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<S, T> FromRequestParts<S> for FindSingleton<T>
where
    S: Send + Sync,
    T: Send + Sync + 'static,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(_req: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let component = None;

        Ok(Self(component))
    }
}
