use std::collections::BTreeMap;

use next_web_core::{async_trait, error::BoxError};

use serde::de::DeserializeOwned;
use serde_json::Value;

#[async_trait]
pub trait TemplateService: Send + Sync {
    /// Create a new SMS template
    ///
    /// # Arguments
    ///
    /// * `template_name` - Name of the template.
    /// * `template_content` - Content of the template.
    /// * `template_type` - Type of the template (e.g., verification code).
    /// * `expand_params` - Additional parameters like related sign name or international type.
    ///
    /// # Returns
    ///
    /// * `Ok(R)` with response data if successful.
    /// * `Err(BoxError)` if validation or API call fails.
    async fn create_template<'a, R>(
        &self,
        template_name: &'a str,
        template_content: &'a str,
        template_type: i32,
        expand_params: Option<BTreeMap<&'a str, Value>>,
    ) -> Result<R, BoxError>
    where
        R: DeserializeOwned;

    /// Delete an existing SMS template by its code.
    ///
    /// # Arguments
    ///
    /// * `template_code` - Unique identifier of the template.
    ///
    /// # Returns
    ///
    /// * `Ok(R)` with response data if successful.
    /// * `Err(BoxError)` if template code is empty or API call fails.
    async fn delete_template<R>(&self, template_code: &str) -> Result<R, BoxError>
    where
        R: DeserializeOwned;

    /// Update an existing SMS template.
    ///
    /// # Arguments
    ///
    /// * `template_code` - Unique identifier of the template.
    /// * `template_name` - New name for the template.
    /// * `template_content` - New content for the template.
    /// * `template_type` - Type of the template.
    /// * `expand_params` - Optional additional parameters.
    ///
    /// # Returns
    ///
    /// * `Ok(R)` with response data if successful.
    /// * `Err(BoxError)` if validation or API call fails.
    async fn update_template<'a, R>(
        &self,
        template_code: &'a str,
        template_name: &'a str,
        template_content: &'a str,
        template_type: i32,
        expand_params: Option<BTreeMap<&'a str, Value>>,
    ) -> Result<R, BoxError>
    where
        R: DeserializeOwned;

    /// Query a list of SMS templates.
    ///
    /// # Arguments
    ///
    /// * `template_type` - Filter by template type.
    /// * `index` - Page index (starting from 1).
    /// * `size` - Number of items per page.
    ///
    /// # Returns
    ///
    /// * `Ok(R)` with response data if successful.
    /// * `Err(BoxError)` if page index or size is invalid or API call fails.
    async fn query_template<R>(
        &self,
        template_type: i32,
        index: u16,
        size: u16,
    ) -> Result<R, BoxError>
    where
        R: DeserializeOwned;
}
