use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

use axum::{
    extract::{FromRef, FromRequestParts},
    http::{request::Parts, StatusCode},
};
use tokio::sync::RwLock;

use crate::{interface::singleton::Singleton, ApplicationContext};

#[derive(Clone)]
pub struct ApplicationState {
    pub(crate) context: Arc<RwLock<ApplicationContext>>,
}

impl ApplicationState {
    pub fn new(application_context: ApplicationContext) -> Self {
        let context: Arc<RwLock<ApplicationContext>> = Arc::new(RwLock::new(application_context));
        Self { context }
    }

    pub async fn get_single_with_name<T>(&self, name: impl Into<String>) -> T
    where
        T: Clone + 'static,
    {
        let reader = self.context.read().await;
        reader.get_single_with_name::<T>(name.into()).clone()
    }
}

#[derive(Clone)]
pub struct AcSingleton<T>(pub T)
where
    T: Clone;

impl<T> Deref for AcSingleton<T>
where
    T: Singleton + Clone,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for AcSingleton<T>
where
    T: Singleton + Clone,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<S, T> FromRequestParts<S> for AcSingleton<T>
where
    S: Send + Sync,
    T: Singleton + Clone + 'static,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(req: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let state = req.extensions.get::<ApplicationState>();

        let instance = if let Some(state) = state {
            let singleton_name = get_singleton_name::<T>();
            let reader = state.context.read().await;
            if reader.contains_single_with_name::<T>(singleton_name.to_owned()) {
                reader.get_single_with_name::<T>(singleton_name).clone()
            } else {
                state
                    .context
                    .write()
                    .await
                    .resolve_with_name::<T>(singleton_name)
            }
        } else {
            return Err((StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error"));
        };

        Ok(Self(instance))
    }
}

impl<T> FromRef<ApplicationState> for AcSingleton<T>
where
    T: Singleton + Clone + 'static,
{
    fn from_ref(state: &ApplicationState) -> Self {
        let singleton_name = get_singleton_name::<T>();

        AcSingleton(
            state
                .context
                .try_write()
                .unwrap()
                .resolve_with_name::<T>(singleton_name)
                .clone(),
        )
    }
}

fn get_singleton_name<T>() -> String {
    let raw_name = std::any::type_name::<T>();
    let name = raw_name.rsplit("::").next().unwrap_or_default();

    // Convert the first character to lowercase and concatenate with the rest of the string.
    let mut chars = name.chars();
    match chars.next() {
        Some(first_char) => {
            let mut singleton_name = String::with_capacity(name.len());
            singleton_name.extend(first_char.to_lowercase());
            singleton_name.push_str(chars.as_str());
            singleton_name
        }
        None => name.to_string(), // Fallback for an unlikely empty string case.
    }
}
