//! OpenAPI and Swagger documentation support for an application.
//!
//! The module re-exports the [`next_web_open_api`] crate, which integrates
//! [utoipa](https://crates.io/crates/utoipa) with the framework, and declares
//! the [`OpenApiProperties`] an application describes its documentation with.
//! The properties are bound from the `next.open_api` prefix of the environment
//! and the [`Application::open_api`](crate::Application::open_api) default
//! implementation reads them to build the documentation.

pub use next_web_open_api::*;

mod properties;

pub use properties::{
    ContactProperties, DEFAULT_CONTACT_NAME, DEFAULT_DESCRIPTION, DEFAULT_LICENSE_NAME,
    DEFAULT_TITLE, DEFAULT_VERSION, LicenseProperties, OPEN_API_PROPERTIES_PREFIX,
    OpenApiProperties,
};
