use std::{
    fmt::{self, Debug},
    io,
    sync::Arc,
};

use next_web_context::{
    Locale, MessageSource, MessageSourceResolvable, NoSuchMessageError,
    support::{BundleLoader, MessageSourceAccessor, ResourceBundleMessageSource},
};
use next_web_core::io::{Resource, web_resource_loader};

/// The default MessageSource used by Next Security.
/// All Next Security classes requiring message localization will by default use this class. However, all such
/// classes will also implement MessageSourceAware so that the application context can inject an alternative
/// message source. Therefore this class is only used when the deployment environment has not specified an alternative message source.
#[derive(Clone, Default)]
pub struct NextSecurityMessageSource(ResourceBundleMessageSource);

impl NextSecurityMessageSource {
    /// Returns the accessor the components of the security support resolve their
    /// messages with.
    ///
    /// The source reads the message bundles of the application, so an
    /// application that defines the codes of the security support in its own
    /// bundles answers them in the language it configures:
    ///
    /// ```text
    /// messages/messages.properties
    /// AbstractUserDetailsAuthenticationProvider.badCredentials=Bad credentials
    /// ```
    ///
    /// A component that has a message source of its own injected keeps using it,
    /// which is what lets an application replace this default one.
    pub fn get_accessor() -> MessageSourceAccessor {
        MessageSourceAccessor::new(Arc::new(Self::with_application_messages()))
    }

    /// Returns the source that reads the message bundles of the application.
    fn with_application_messages() -> Self {
        let mut message_source = ResourceBundleMessageSource::default();
        message_source.set_bundle_loader(Arc::new(ApplicationBundleLoader));
        message_source.set_extensions(["properties", "json", "yaml", "yml"]);

        Self(message_source)
    }
}

/// Reads the message bundles of the application from the resources of the web
/// layer.
///
/// A component of the security support is built while the application starts,
/// after the loader of the web layer was installed, so the bundles it reads are
/// the ones the rest of the framework reads.
#[derive(Debug, Default, Clone, Copy)]
struct ApplicationBundleLoader;

impl BundleLoader for ApplicationBundleLoader {
    fn load(&self, location: &str) -> io::Result<Option<Vec<u8>>> {
        let resource_loader = web_resource_loader();

        if !resource_loader.exists(location) {
            return Ok(None);
        }

        let resource = resource_loader.get_resource(location)?;

        Ok(Some(resource.get_content()?.into_owned()))
    }
}

impl MessageSource for NextSecurityMessageSource {
    /// Resolves the message from the resource bundles of the source, and falls
    /// back to the given default message.
    fn message_or_default(
        &self,
        code: &str,
        args: &[&dyn fmt::Display],
        default_message: Option<&str>,
        locale: Option<&Locale>,
    ) -> Option<String> {
        self.0
            .message_or_default(code, args, default_message, locale)
    }

    /// Resolves the message from the resource bundles of the source.
    ///
    /// # Errors
    ///
    /// Returns [`NoSuchMessageError`] when the bundles do not contain the code
    /// and the source does not hold a default message for it.
    fn message(
        &self,
        code: &str,
        args: &[&dyn fmt::Display],
        locale: Option<&Locale>,
    ) -> Result<String, NoSuchMessageError> {
        self.0.message(code, args, locale)
    }

    /// Resolves the message the resolvable describes from the resource bundles
    /// of the source.
    ///
    /// # Errors
    ///
    /// Returns [`NoSuchMessageError`] when the bundles do not contain the codes
    /// of the resolvable and it does not carry a default message.
    fn message_from_resolvable(
        &self,
        resolvable: &dyn MessageSourceResolvable,
        locale: Option<&Locale>,
    ) -> Result<String, NoSuchMessageError> {
        self.0.message_from_resolvable(resolvable, locale)
    }
}

impl Debug for NextSecurityMessageSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NextSecurityMessageSource")
    }
}
