use std::ops::{Deref, DerefMut};

use crate::ApplicationState;
use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
};
use next_web_context::ApplicationContextExt;
use next_web_core::util::SingletonUtil;

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
        let state = req
            .extensions
            .get::<ApplicationState>()
            .ok_or((StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error"))?;

        let name = SingletonUtil::name::<T>();

        let instance = find(state, name)
            .await
            .ok_or((StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error"))?;

        Ok(Self(instance))
    }
}

pub async fn find<T>(state: &ApplicationState, name: String) -> Option<T>
where
    T: Send + Sync,
    T: Clone + 'static,
{
    let reader = state.read().await;

    if let Some(instance) = reader
        .get_singleton_option_with_name::<T>(name.to_owned())
        .map(Clone::clone)
    {
        return Some(instance);
    }

    if let Some(instance) = reader
        .get_singleton_option_with_name::<T>("")
        .map(Clone::clone)
    {
        return Some(instance);
    }

    drop(reader);

    let mut writer = state.write().await;

    writer.resolve_option_with_name::<T>(name)
}
