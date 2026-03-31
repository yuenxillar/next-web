use next_web_core::{DynClone, clone_trait_object, error::BoxError};

/// Trait for configuring an `Configurer`.
///
/// All `Configurer`s first have their `init` method invoked.
/// After all `init` methods have been invoked, each `configure` method is invoked.
///
/// # Type Parameters
/// * `O` - The object being built by the `Builder` B
/// * `B` - The `Builder` that builds objects of type O. This is
///         also the `Builder` that is being configured.
pub trait Configurer<O, B>
where
    B: super::builder::Builder<O>,
    Self: DynClone,
{
    /// Initializes the `Builder`.
    ///
    /// Here only shared state should be created and modified, but not properties
    /// on the `Builder` used for building the object. This ensures that
    /// the `configure` method uses the correct shared objects when building.
    ///
    /// # Arguments
    /// * `builder` - The builder to initialize
    ///
    /// # Returns
    /// * `Result<(), Box<dyn Error>>` - Result of the initialization
    fn init(&mut self, builder: &B) -> Result<(), BoxError>;

    /// Configures the `Builder` by setting the necessary properties
    /// on the `Builder`.
    ///
    /// # Arguments
    /// * `builder` - The builder to configure
    ///
    /// # Returns
    /// * `Result<(), Box<dyn Error>>` - Result of the configuration
    fn configure(&mut self, builder: &B) -> Result<(), BoxError>;

    /// Checks if this configurer can be assigned to the given builder.
    ///
    /// # Arguments
    /// * `builder` - The builder to check against
    ///
    /// # Returns
    /// * `bool` - `true` if this configurer can be assigned to the builder,
    ///           `false` otherwise
    fn is_assignable(&self, builder: &B) -> bool;
}

clone_trait_object!(<O, B> Configurer<O, B> where B: super::builder::Builder<O>);
