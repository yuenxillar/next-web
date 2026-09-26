use from_attr::FromAttr;
use syn::{Expr, ExprPath, LitStr};

/// The attributes of `#[auto_configuration]`.
///
/// # Attributes
///
/// * `name` - The name the configuration is registered under in the
///   application context. It defaults to the name of the generated type, which
///   is derived from the annotated type and therefore unique.
/// * `order` - The order the configuration is applied in, as an integer. It
///   defaults to `100`, the default of [`Ordered`](next_web_core::Ordered), so
///   a configuration runs after the ones of a lower order and before the ones
///   of a higher order.
/// * `conditional` - The conditions the configuration is applied under. Each
///   names a function that takes `&dyn ApplicationContext` and returns a
///   `bool`, and the configuration is skipped when one of them returns
///   `false`, which is the counterpart of the `#[provider(conditional = ...)]`
///   attribute of the methods.
#[derive(FromAttr)]
#[attribute(idents = [_none])]
pub(crate) struct AutoConfigurationAttr {
    pub name: Option<LitStr>,
    pub order: Option<Expr>,
    pub conditional: Vec<ExprPath>,
}

/// The attributes of a provider of an auto-configuration.
///
/// # Attributes
///
/// * `name` - The name the instance is registered under. It defaults to the
///   name of the method, which is lower cased at its first character.
/// * `conditional` - The conditions the provider is created under. Each names a
///   function that takes `&dyn ApplicationContext` and returns a `bool`, and
///   the instance is not created when one of them returns `false`.
/// * `order` - The order the provider is created in, as an integer. It defaults
///   to `100`, so a provider runs after the ones of a lower order and before
///   the ones of a higher order.
#[derive(FromAttr)]
#[attribute(idents = [provider])]
pub(crate) struct ProviderAttr {
    pub name: Option<LitStr>,
    pub conditional: Vec<ExprPath>,
    pub order: Option<Expr>,
}

/// The attributes of `#[conditional_on_property]`.
///
/// # Attributes
///
/// * `name` - The name of the property the provider is conditioned on.
/// * `having_value` - The value the property has to hold for the provider to be
///   created. The value of the property is compared as the text the environment
///   resolves for it.
#[derive(FromAttr)]
#[attribute(idents = [conditional_on_property])]
pub(crate) struct ConditionalOnPropertyAttr {
    pub name: LitStr,
    pub having_value: LitStr,
}

/// The attributes of `#[autowired]` of a parameter.
///
/// # Attributes
///
/// * `name` - The name the parameter is resolved under. It defaults to the name
///   of the parameter.
/// * `default` - Whether the parameter falls back to `Default::default()` when
///   no instance is registered under its name.
#[derive(FromAttr)]
#[attribute(idents = [autowired])]
pub(crate) struct AutowiredAttr {
    pub name: Option<LitStr>,
    pub default: Option<bool>,
}

impl std::fmt::Debug for ProviderAttr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProviderAttr")
            .field("name", &self.name)
            .field("conditional", &self.conditional)
            .field("order", &self.order)
            .finish()
    }
}
