//! The configuration properties of the OpenAPI documentation of an application.
//!
//! The properties are bound from the `next.open_api` prefix of the environment
//! of the application, and the instance is registered as a singleton of the
//! application context under the name `openApiProperties`, so a component can
//! depend on them like on any other singleton. The default implementation of
//! [`Application::open_api`](crate::Application::open_api) reads them to
//! describe the API its documentation is served for.
//!
//! ```yaml
//! next:
//!   open_api:
//!     title: Pet API
//!     version: 1.0.0
//!     description: The API of the pet store.
//!     terms_of_service: https://example.com/terms
//!     license:
//!       name: MIT or Apache-2.0
//!       url: https://example.com/license
//!     contact:
//!       name: Pet Store Team
//!       email: team@example.com
//! ```
//!
//! A field is read from the property of the same name, written in snake case,
//! in kebab case, or as the name of an environment variable, so the title above
//! is also read from `next.open_api.title`, `next.open-api.title` and
//! `NEXT_OPEN_API_TITLE`. The values of the placeholders of a property are
//! resolved against the environment as well, so `title: ${API_TITLE}` works. See
//! [`Binder`](next_web_core::env::Binder) for the rules of the binding.
//!
//! Every value the environment does not configure falls back to the default of
//! its field, and the properties also implement [`Deserialize`] and
//! [`Serialize`], so a document that is written by hand can be read into the
//! struct and the effective configuration can be written back out.

use next_web_macros::configuration_properties;
use next_web_open_api::openapi::{
    Contact, ContactBuilder, Info, InfoBuilder, License, LicenseBuilder,
};
use serde::{Deserialize, Serialize};

/// The prefix the properties of the OpenAPI documentation are bound from.
pub const OPEN_API_PROPERTIES_PREFIX: &str = "next.open_api";

/// The title of the documentation of an application that configures none.
pub const DEFAULT_TITLE: &str = "API Documentation";

/// The version of the documentation of an application that configures none.
pub const DEFAULT_VERSION: &str = "0.1.0";

/// The name of the license of an application that configures none.
pub const DEFAULT_LICENSE_NAME: &str = "MIT or Apache-2.0";

/// The description of an application that configures none.
///
/// The description is read from the `CARGO_PKG_DESCRIPTION` environment
/// variable, which Cargo sets for the process it runs a binary in, and falls
/// back to `Empty` when it is unset.
pub const DEFAULT_DESCRIPTION: &str = "Empty";

/// The name of the contact of an application that configures none.
///
/// The name is read from the `CARGO_PKG_AUTHORS` environment variable, which
/// Cargo sets for the process it runs a binary in, and falls back to the author
/// of the framework when it is unset.
pub const DEFAULT_CONTACT_NAME: &str = "Listening";

/// The properties of the OpenAPI documentation of an application.
///
/// The properties are bound from the `next.open_api` prefix of the environment
/// while the application starts, and the instance is registered as a singleton
/// named `openApiProperties`. The [`Application::open_api`] implementation of
/// the framework reads them to build the [`Info`] of the documentation, so an
/// application that declares none of them serves the defaults.
///
/// # Examples
///
/// Reading the properties of a document:
///
/// ```
/// use next_web::open_api::OpenApiProperties;
///
/// let properties: OpenApiProperties = serde_yaml::from_str(
///     r#"
///     title: Pet API
///     version: 1.0.0
///     license:
///       name: MIT
///     "#,
/// )
/// .unwrap();
///
/// assert_eq!(properties.title(), "Pet API");
/// assert_eq!(properties.version(), "1.0.0");
/// assert_eq!(properties.license().map(|license| license.name()), Some("MIT"));
/// ```
///
/// [`Application::open_api`]: crate::Application::open_api
#[configuration_properties(prefix = "next.open_api")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct OpenApiProperties {
    /// The title of the API the documentation describes.
    title: String,

    /// The version of the API the documentation describes.
    ///
    /// The version is the version of the documentation, which is usually the
    /// version of the API and not the version of the application.
    version: String,

    /// The description of the API, which supports markdown.
    description: Option<String>,

    /// The url of the terms of service of the API.
    terms_of_service: Option<String>,

    /// The license of the API.
    license: Option<LicenseProperties>,

    /// The contact of the API.
    contact: Option<ContactProperties>,
}

impl OpenApiProperties {
    /// Returns the title of the API.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Returns the version of the API.
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Returns the description of the API.
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// Returns the url of the terms of service of the API.
    pub fn terms_of_service(&self) -> Option<&str> {
        self.terms_of_service.as_deref()
    }

    /// Returns the license of the API.
    pub fn license(&self) -> Option<&LicenseProperties> {
        self.license.as_ref()
    }

    /// Returns the contact of the API.
    pub fn contact(&self) -> Option<&ContactProperties> {
        self.contact.as_ref()
    }

    /// Sets the title of the API.
    pub fn set_title(&mut self, title: impl Into<String>) {
        self.title = title.into();
    }

    /// Sets the version of the API.
    pub fn set_version(&mut self, version: impl Into<String>) {
        self.version = version.into();
    }

    /// Sets the description of the API.
    pub fn set_description(&mut self, description: impl Into<String>) {
        self.description = Some(description.into());
    }

    /// Sets the url of the terms of service of the API.
    pub fn set_terms_of_service(&mut self, terms_of_service: impl Into<String>) {
        self.terms_of_service = Some(terms_of_service.into());
    }

    /// Sets the license of the API.
    pub fn set_license(&mut self, license: impl Into<Option<LicenseProperties>>) {
        self.license = license.into();
    }

    /// Sets the contact of the API.
    pub fn set_contact(&mut self, contact: impl Into<Option<ContactProperties>>) {
        self.contact = contact.into();
    }

    /// Builds the [`Info`] of the documentation the properties describe.
    ///
    /// The license and the contact are only added to the [`Info`] when the
    /// properties declare them, so an application that disables one of them
    /// serves a document without it.
    pub fn to_info(&self) -> Info {
        let mut info = InfoBuilder::new()
            .title(self.title.clone())
            .version(self.version.clone())
            .description(self.description.clone())
            .terms_of_service(self.terms_of_service.clone());

        if let Some(license) = &self.license {
            info = info.license(Some(license.to_license()));
        }

        if let Some(contact) = &self.contact {
            info = info.contact(Some(contact.to_contact()));
        }

        info.build()
    }
}

impl Default for OpenApiProperties {
    fn default() -> Self {
        Self {
            title: DEFAULT_TITLE.to_owned(),
            version: DEFAULT_VERSION.to_owned(),
            description: Some(
                std::env::var("CARGO_PKG_DESCRIPTION")
                    .unwrap_or_else(|_| DEFAULT_DESCRIPTION.to_owned()),
            ),
            terms_of_service: None,
            license: Some(LicenseProperties::default()),
            contact: Some(ContactProperties::default()),
        }
    }
}

/// The license of the OpenAPI documentation of an application.
///
/// The properties are part of [`OpenApiProperties`] and are bound from the
/// `next.open_api.license` prefix of the environment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct LicenseProperties {
    /// The name of the license, for example `MIT or Apache-2.0`.
    name: String,

    /// The url pointing to the license.
    ///
    /// The url is mutually exclusive with the [`identifier`](Self::identifier).
    url: Option<String>,

    /// The [SPDX][spdx] expression of the license, for example `Apache-2.0`.
    ///
    /// The identifier is mutually exclusive with the [`url`](Self::url).
    ///
    /// [spdx]: https://spdx.org/licenses/
    identifier: Option<String>,
}

impl LicenseProperties {
    /// Returns the name of the license.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the url pointing to the license.
    pub fn url(&self) -> Option<&str> {
        self.url.as_deref()
    }

    /// Returns the SPDX expression of the license.
    pub fn identifier(&self) -> Option<&str> {
        self.identifier.as_deref()
    }

    /// Sets the name of the license.
    pub fn set_name(&mut self, name: impl Into<String>) {
        self.name = name.into();
    }

    /// Sets the url pointing to the license.
    pub fn set_url(&mut self, url: impl Into<String>) {
        self.url = Some(url.into());
    }

    /// Sets the SPDX expression of the license.
    pub fn set_identifier(&mut self, identifier: impl Into<String>) {
        self.identifier = Some(identifier.into());
    }

    /// Builds the [`License`] the properties describe.
    pub fn to_license(&self) -> License {
        LicenseBuilder::new()
            .name(self.name.clone())
            .url(self.url.clone())
            .identifier(self.identifier.clone())
            .build()
    }
}

impl Default for LicenseProperties {
    fn default() -> Self {
        Self {
            name: DEFAULT_LICENSE_NAME.to_owned(),
            url: None,
            identifier: None,
        }
    }
}

/// The contact of the OpenAPI documentation of an application.
///
/// The properties are part of [`OpenApiProperties`] and are bound from the
/// `next.open_api.contact` prefix of the environment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ContactProperties {
    /// The identifying name of the contact person or organization of the API.
    name: Option<String>,

    /// The url pointing to the contact information of the API.
    url: Option<String>,

    /// The email of the contact person or organization of the API.
    email: Option<String>,
}

impl ContactProperties {
    /// Returns the name of the contact.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Returns the url pointing to the contact information.
    pub fn url(&self) -> Option<&str> {
        self.url.as_deref()
    }

    /// Returns the email of the contact.
    pub fn email(&self) -> Option<&str> {
        self.email.as_deref()
    }

    /// Sets the name of the contact.
    pub fn set_name(&mut self, name: impl Into<String>) {
        self.name = Some(name.into());
    }

    /// Sets the url pointing to the contact information.
    pub fn set_url(&mut self, url: impl Into<String>) {
        self.url = Some(url.into());
    }

    /// Sets the email of the contact.
    pub fn set_email(&mut self, email: impl Into<String>) {
        self.email = Some(email.into());
    }

    /// Builds the [`Contact`] the properties describe.
    pub fn to_contact(&self) -> Contact {
        ContactBuilder::new()
            .name(self.name.clone())
            .url(self.url.clone())
            .email(self.email.clone())
            .build()
    }
}

impl Default for ContactProperties {
    fn default() -> Self {
        Self {
            name: Some(
                std::env::var("CARGO_PKG_AUTHORS")
                    .unwrap_or_else(|_| DEFAULT_CONTACT_NAME.to_owned()),
            ),
            url: None,
            email: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The properties of an application that configures none serve the
    /// documentation the framework served before the properties existed. The
    /// description and the name of the contact describe the package Cargo runs
    /// the binary for, so only the values that do not depend on the process are
    /// asserted here.
    #[test]
    fn defaults_describe_the_documentation_of_the_application() {
        let info = OpenApiProperties::default().to_info();

        assert_eq!(info.title, DEFAULT_TITLE);
        assert_eq!(info.version, DEFAULT_VERSION);
        assert!(info.description.is_some());
        assert_eq!(
            info.license.as_ref().map(|license| license.name.as_str()),
            Some(DEFAULT_LICENSE_NAME)
        );
        assert!(info.contact.is_some());
    }

    /// The properties a document configures are the ones the documentation is
    /// served with, and the values the document omits fall back to the defaults.
    #[test]
    fn a_document_describes_the_documentation() {
        let properties: OpenApiProperties = serde_yaml::from_str(
            r#"
            title: Pet API
            version: 1.0.0
            description: The API of the pet store.
            license:
              name: MIT
              identifier: MIT
            contact:
              name: Pet Store Team
              email: team@example.com
            "#,
        )
        .expect("the document describes the properties");

        let info = properties.to_info();

        assert_eq!(info.title, "Pet API");
        assert_eq!(info.version, "1.0.0");
        assert_eq!(
            info.description.as_deref(),
            Some("The API of the pet store.")
        );

        let license = info.license.expect("the license is described");
        assert_eq!(license.name, "MIT");
        assert_eq!(license.identifier.as_deref(), Some("MIT"));

        let contact = info.contact.expect("the contact is described");
        assert_eq!(contact.name.as_deref(), Some("Pet Store Team"));
        assert_eq!(contact.email.as_deref(), Some("team@example.com"));
    }

    /// The properties are bound from the `next.open_api` prefix of the
    /// environment, so the property sources of the environment describe them
    /// the same way a document does.
    #[test]
    fn the_properties_are_bound_from_the_environment() {
        use next_web_core::{
            context::properties::ConfigurationProperties,
            env::{BaseEnvironment, ConfigurableEnvironment, MapPropertySource},
            util::indexmap::IndexMap,
        };

        let mut environment = BaseEnvironment::new();
        environment
            .property_sources()
            .add_last(Box::new(MapPropertySource::new(
                "test".to_owned(),
                [
                    ("next.open_api.title", "Pet API"),
                    ("next.open_api.version", "2.0.0"),
                    ("next.open_api.license.name", "Apache-2.0"),
                    ("next.open_api.contact.email", "team@example.com"),
                ]
                .into_iter()
                .map(|(key, value)| (key.to_owned(), value.to_owned()))
                .collect::<IndexMap<_, _>>(),
            )));

        let properties = OpenApiProperties::bind(&environment).expect("the properties bind");
        let info = properties.to_info();

        assert_eq!(info.title, "Pet API");
        assert_eq!(info.version, "2.0.0");
        assert_eq!(
            info.license.as_ref().map(|license| license.name.as_str()),
            Some("Apache-2.0")
        );
        assert_eq!(
            info.contact
                .as_ref()
                .and_then(|contact| contact.email.as_deref()),
            Some("team@example.com")
        );
    }

    /// The properties are registered under the name the framework looks them
    /// up by, so the implementation of `Application::open_api` resolves the
    /// instance the environment configured.
    #[test]
    fn the_properties_are_registered_under_the_name_of_the_type() {
        use next_web_core::context::properties::ConfigurationProperties;
        use next_web_singletons::util::SingletonUtil;

        let properties = OpenApiProperties::default();

        assert_eq!(properties.prefix(), OPEN_API_PROPERTIES_PREFIX);
        assert_eq!(
            properties.name(),
            SingletonUtil::default_singleton_name::<OpenApiProperties>()
        );
    }
}
