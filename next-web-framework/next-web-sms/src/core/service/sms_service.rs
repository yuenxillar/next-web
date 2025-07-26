use std::collections::BTreeMap;

use next_web_core::{async_trait, interface::service::Service, error::BoxError};
use serde_json::Value;

#[async_trait]
pub trait SmsService: Service {
    type Response: serde::de::DeserializeOwned;

    /// Send a single SMS message to one phone number.
    ///
    /// # Arguments
    ///
    /// * `phone_numbers` - A string representing the target phone numbers (e.g., "13800138000").
    /// * `sign_name` - The SMS signature name.
    /// * `template_code` - The template ID.
    /// * `template_param` - Template parameters.
    /// * `expand_params` - Optional additional request parameters.
    ///
    /// # Returns
    ///
    /// * `Ok(Self::Response)` if successful.
    /// * `Err(BoxError)` if validation fails or API call fails.
    ///
    async fn send_sms<'a>(
        &self,
        phone_number: &'a str,
        sign_name: &'a str,
        template_code: &'a str,
        template_param: &'a str,
        expand_params: Option<BTreeMap<&'a str, Value>>,
    ) -> Result<Self::Response, BoxError>;

    /// Send SMS messages to multiple phone numbers in batch.
    ///
    /// # Arguments
    ///
    /// * `phone_numbers` - List of phone numbers.
    /// * `sign_names` - Corresponding list of signature names.
    /// * `template_code` - The same template code used for all messages.
    /// * `template_params` - Parameters for each message in JSON array format.
    /// * `expand_params` - Optional additional request parameters.
    ///
    /// # Returns
    ///
    /// * `Ok(Self::Response)` if successful.
    /// * `Err(BoxError)` if input validation or API call fails.
    async fn send_batch_sms<'a>(
        &self,
        phone_numbers: Vec<&'a str>,
        sign_names: Vec<&'a str>,
        template_code: &'a str,
        template_param: Vec<&'a str>,
        expand_params: Option<BTreeMap<&'a str, Value>>,
    ) -> Result<Self::Response, BoxError>;

    /// Validate phone number and signature name are non-empty.
    fn check_validity<'a>(&self, phone_number: &'a str, sign_name: &'a str) -> bool;

    /// Get the base URL for the SMS service.
    fn url(&self) -> &str;

    /// Build common HTTP headers required for API requests.
    fn common_req_headers(&self) -> BTreeMap<&str, String>;

}
