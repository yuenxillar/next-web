use async_trait::async_trait;

use crate::context::{application_context::ApplicationContext, properties::ApplicationProperties};

///
/// AutoRegister trait
///
/// This trait is used to register a module to the application context.
///
/// The `register` method is called by the framework to register the module.
/// The `singleton_name` method is used to get the name of the module.
///
/// The `register` method takes two parameters:
/// - `ctx`: The application context.
/// - `properties`: The application properties.
///
/// The `ApplicationContext` and `ApplicationProperties` are used to access the application context and properties.
///
/// The `register` method returns a `Result` with an error type of `Box<dyn std::error::Error>`.
///
/// The `AutoRegister` trait is implemented for all modules that want to be registered automatically.
///
#[async_trait]
pub trait AutoRegister: Sync + Send {
    /// Get the name of the registration instance.
    ///
    /// This method is used to obtain the name of the registration instance.
    ///
    fn registered_name(&self) -> &'static str;

    ///
    /// Register the singleton to the application context.
    ///
    /// This method is called by the framework to register the singleton.
    ///
    async fn register(
        &self,
        ctx: &mut ApplicationContext,
        properties: &ApplicationProperties,
    ) -> Result<(), Box<dyn std::error::Error>>;
}

#[doc(hidden)]
pub use inventory::submit;

use crate::{DynProvider, Module};

#[doc(hidden)]
pub struct ProviderRegister {
    pub register: fn() -> DynProvider,
}

inventory::collect!(ProviderRegister);

/// Returns an iterator over all auto-registered providers.
///
/// [`AutoRegisterModule`] uses this function to collect all auto-registered [`DynProvider`]s.
/// If you don't want to use `AutoRegisterModule`, you can use this function to customize your own module.
///
/// # Example
///
/// ```rust
/// use rudi::{auto_registered_providers, modules, Context, DynProvider, Module, SingleOwner};
///
/// struct MyAutoRegisterModule;
///
/// impl Module for MyAutoRegisterModule {
///     fn eager_create() -> bool {
///         true
///     }
///
///     fn providers() -> Vec<DynProvider> {
///         auto_registered_providers().collect()
///     }
/// }
///
/// #[SingleOwner]
/// struct A;
///
/// # fn main() {
/// let cx = Context::create(modules![MyAutoRegisterModule]);
/// assert!(cx.get_single_option::<A>().is_some());
/// # }
/// ```
pub fn auto_registered_providers() -> impl Iterator<Item = DynProvider> {
    inventory::iter::<ProviderRegister>
        .into_iter()
        .map(|register| (register.register)())
}

/// A module that auto-registers all providers.
///
/// This module is enabled by the `auto-register` feature.
/// Because auto-registration relies on [`inventory`] crate, auto-registration
/// is not available on platforms where `inventory` is not supported.
///
/// # Example
///
/// ```rust
/// use rudi::{modules, AutoRegisterModule, Context, Singleton, Transient};
///
/// #[Singleton]
/// #[derive(Clone)]
/// struct A;
///
/// #[Transient]
/// struct B(A);
///
/// # fn main() {
/// let mut cx = Context::create(modules![AutoRegisterModule]);
/// assert!(cx.resolve_option::<B>().is_some());
/// # }
/// ```
pub struct AutoRegisterModule;

impl Module for AutoRegisterModule {
    fn providers() -> Vec<DynProvider> {
        auto_registered_providers().collect()
    }
}

/// Register a `Provider` that will be collected by [`auto_registered_providers`].
///
/// If you have:
///   - Enabled the `auto-register` feature (which is enabled by default).
///   - Define [`Provider`](crate::Provider) using the [`#[Singleton]`](crate::Singleton), [`#[Transient]`](crate::Transient) or [`#[SingleOwner]`](crate::SingleOwner) macro.
///   - [`#[Singleton]`](crate::Singleton), [`#[Transient]`](crate::Transient) or [`#[SingleOwner]`](crate::SingleOwner) does not use the `auto_register = false` attribute.
///
/// Then you don't need to use this macro to register `Provider`.
///
/// But if you use function define a [`Provider`](crate::Provider) and you want to use auto-registration,
/// then you need to use this macro.
///
/// # Example
///
/// ```rust
/// use rudi::{register_provider, singleton, Context, Provider};
///
/// fn foo() -> Provider<&'static str> {
///     singleton(|_| "Hello").into()
/// }
///
/// register_provider!(foo());
///
/// fn main() {
///     let mut cx = Context::auto_register();
///     assert!(cx.resolve_option::<&'static str>().is_some());
/// }
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

/// Generate a function to enable auto-registration.
///
/// In Rust, it is possible to use [`inventory`] to accomplish something like
/// auto-registration, but there is still a problem, and it exists in Rudi as well.
///
/// Suppose you have two crates, one called `crate_1` and one called `crate_2`,
/// and you define some auto-registration types in `crate_2`.
///
/// If it is just a dependency on `crate_2` in `crate_1`'s `Cargo.toml`, then using
/// [`auto_registered_providers`] in `crate_1` will not collect the types defined in `crate_2`,
/// you have to use a function (or type, or constant) in `crate_1` that is defined in `crate_2`
/// in order to enable auto-registration.
///
/// So, there is this macro, which generates a function called `enable`, with no parameters
/// and no return, just to be called by other crates to enable auto-registration.
///
/// At the same time, you can also call the enable functions of other crates that the current
/// crate depends on in this macro, so that when the enable function of the current crate is
/// called, the enable functions of other crates will be called together.
///
/// # Example
///
/// ```rust ignore
/// // lib1/src/lib.rs
/// use rudi::{enable, Transient};
///
/// enable! {}
///
/// #[Transient(name = "lib1")]
/// fn Lib1() -> i32 {
///     5
/// }
///
/// // lib2/src/lib.rs
/// use rudi::{enable, Transient};
///
/// enable! {
///     lib1::enable();
/// }
///
/// #[Transient(name = "lib2")]
/// fn Lib2() -> i32 {
///     5
/// }
///
/// // bin/src/main.rs
/// use rudi::Context;
///
/// fn main() {
///     lib2::enable();
///
///     let mut cx = Context::auto_register();
///     assert_eq!(cx.resolve_by_type::<i32>().into_iter().sum::<i32>(), 10);
/// }
/// ```
#[macro_export]
macro_rules! enable {
    ($($body:tt)*) => {
        /// Enable auto-registration.
        pub fn enable() {
            $($body)*
        }
    };
}
