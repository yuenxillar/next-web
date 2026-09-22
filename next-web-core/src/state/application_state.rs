//! Shared access to the application context of a running application.

use std::{borrow::Cow, sync::Arc};

use next_web_context::{ApplicationContext, ApplicationContextExt};
use tokio::sync::RwLock;

/// The application context of a running application, shared with the handlers.
///
/// The state holds the context behind a [`dyn ApplicationContext`], so that it
/// does not depend on the context implementation of the application.
///
/// [`dyn ApplicationContext`]: next_web_context::ApplicationContext
#[derive(Clone)]
pub struct ApplicationState {
    pub(crate) context: Arc<RwLock<Box<dyn ApplicationContext>>>,
}

impl ApplicationState {
    /// Creates the state that shares the given application context.
    ///
    /// # Arguments
    ///
    /// * `application_context` - The context of the application.
    pub fn from_context(application_context: Box<dyn ApplicationContext>) -> Self {
        let context: Arc<RwLock<Box<dyn ApplicationContext>>> =
            Arc::new(RwLock::new(application_context));

        Self { context }
    }

    /// Returns the shared application context.
    pub fn context(&self) -> &Arc<RwLock<Box<dyn ApplicationContext>>> {
        &self.context
    }

    /// Returns the shared application context, mutably.
    pub fn mut_context(&mut self) -> &mut Arc<RwLock<Box<dyn ApplicationContext>>> {
        &mut self.context
    }

    /// Resolves a singleton, creating it when it does not exist yet.
    ///
    /// The instance is looked up under the given name first, then under the
    /// empty name, and is created by the context when it is not available.
    ///
    /// # Panics
    ///
    /// Panics when the context cannot provide an instance for the type and name.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The type of the instance.
    ///
    /// # Arguments
    ///
    /// * `name` - The name the instance is registered under.
    pub async fn get_single_with_name<T>(&self, name: impl Into<Cow<'static, str>>) -> T
    where
        T: Send + Sync + Clone + 'static,
    {
        let name = name.into();

        self.find_single_with_name::<T>(name.clone())
            .await
            .unwrap_or_else(|| panic!("No instance found for {name}"))
    }

    /// Resolves a singleton, creating it when it does not exist yet.
    ///
    /// Returns `None` when the context cannot provide an instance for the type
    /// and the given name.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The type of the instance.
    ///
    /// # Arguments
    ///
    /// * `name` - The name the instance is registered under.
    pub async fn find_single_with_name<T>(&self, name: impl Into<Cow<'static, str>>) -> Option<T>
    where
        T: Send + Sync + Clone + 'static,
    {
        let name = name.into();

        {
            let reader = self.context.read().await;

            if let Some(instance) = reader.get_single_option_with_name::<T>(name.clone()) {
                return Some(instance.clone());
            }

            if let Some(instance) = reader.get_single_option_with_name::<T>("") {
                return Some(instance.clone());
            }
        }

        let mut writer = self.context.write().await;

        writer.resolve_option_with_name::<T>(name)
    }
}

