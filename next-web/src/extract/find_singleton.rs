use std::ops::{Deref, DerefMut};

use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
};
use next_web_core::{state::application_state::ApplicationState, util::singleton::SingletonUtil};

#[derive(Clone)]
pub struct FindSingleton<T>(pub T);

impl<T> Deref for FindSingleton<T>
where
    T: Clone,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for FindSingleton<T>
where
    T: Clone,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<S, T> FromRequestParts<S> for FindSingleton<T>
where
    S: Send + Sync,
    T: Send + Sync + 'static,
    T: Clone,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(req: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let state = req.extensions.get_mut::<ApplicationState>();

        let state = state.ok_or((StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error"))?;
        let singleton_name = SingletonUtil::name::<T>();

        let instance = state
            .find_single_with_name::<T>(singleton_name)
            .await
            .ok_or((StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error"))?;

        Ok(Self(instance))
    }
}
