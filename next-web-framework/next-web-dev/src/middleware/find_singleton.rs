use std::ops::{Deref, DerefMut};

use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
};
use next_web_core::{state::application_state::ApplicationState, traits::singleton::Singleton, util::singleton::SingletonUtil};

#[derive(Clone)]
pub struct FindSingleton<T>(pub T);

impl<T> Deref for FindSingleton<T>
where
    T: Singleton + Clone,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for FindSingleton<T>
where
    T: Singleton + Clone,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<S, T> FromRequestParts<S> for FindSingleton<T>
where
    S: Send + Sync,
    T: Send + Sync + 'static,
    T: Singleton + Clone
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(req: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let state = req.extensions.get_mut::<ApplicationState>();

        let instance = if let Some(state) = state {
            let singleton_name = SingletonUtil::name::<T>();
            let reader = state.context().read().await;

            match reader.get_single_option_with_name::<T>(singleton_name.clone()) {
                Some(instance_with_name) => instance_with_name.clone(),
                None => match reader.get_single_option_with_name::<T>("") {
                    Some(instance) => instance.clone(),
                    None => {
                        drop(reader);
                        match state
                            .context_mut()
                            .write()
                            .await
                            .resolve_option_with_name_async::<T>(singleton_name)
                            .await
                        {
                            Some(instance) => instance,
                            None => {
                                return Err((
                                    StatusCode::INTERNAL_SERVER_ERROR,
                                    "Internal Server Error",
                                ))
                            }
                        }
                    }
                },
            }
        } else {
            return Err((StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error"));
        };

        Ok(Self(instance))
    }
}