//! Auto registration of providers.
//!
//! The attribute macros of the framework submit the [`Provider`] they create
//! with [`register_provider!`], and a context collects them with
//! [`auto_registered_providers`]. This is how a library contributes its
//! singletons without the application having to list them itself.
//!
//! [`Provider`]: crate::Provider

use crate::{DynProvider, Module};

/// The [`inventory`] submission used by [`register_provider!`].
///
/// The type is an implementation detail of the auto registration; it is public
/// because the macro has to name it.
#[doc(hidden)]
pub struct ProviderRegister {
    /// Creates the provider that was submitted.
    pub register: fn() -> DynProvider,
}

inventory::collect!(ProviderRegister);

/// Returns every provider that was submitted with [`register_provider!`].
///
/// Note that a crate that only defines providers has to expose a function
/// (see [`enable!`]) that the application calls, because the submission of a
/// crate is only linked in when one of its items is used.
pub fn auto_registered_providers() -> impl Iterator<Item = DynProvider> {
    inventory::iter::<ProviderRegister>
        .into_iter()
        .map(|register| (register.register)())
}

/// A [`Module`] that contains every auto registered provider.
pub struct AutoRegisterModule;

impl Module for AutoRegisterModule {
    fn providers() -> Vec<DynProvider> {
        auto_registered_providers().collect()
    }
}

#[doc(hidden)]
pub use inventory::submit;

/// Registers a [`Provider`](crate::Provider) for auto registration.
///
/// The attribute macros of the framework use this macro themselves, so it only
/// has to be used for providers that are created by hand.
///
/// # Example
///
/// ```ignore
/// fn hello() -> Provider<&'static str> {
///     singleton(|_| "Hello").into()
/// }
///
/// register_provider!(hello());
/// ```
#[macro_export]
macro_rules! register_provider {
    ($provider:expr) => {
        const _: () = {
            fn register() -> $crate::DynProvider {
                <$crate::DynProvider as ::core::convert::From<_>>::from($provider)
            }

            $crate::submit! {
                $crate::ProviderRegister {
                    register
                }
            }
        };
    };
}

/// Generates the `enable` function that activates the auto registration of a
/// crate.
///
/// A crate that only defines auto registered providers has to expose the
/// function this macro generates, and the application (or a crate above it) has
/// to call it, otherwise the submissions of the crate are not linked in.
///
/// # Example
///
/// ```ignore
/// enable! {}
/// ```
#[macro_export]
macro_rules! enable {
    ($($body:tt)*) => {
        /// Enables the auto registration of the providers of this crate.
        pub fn enable() {
            $($body)*
        }
    };
}
