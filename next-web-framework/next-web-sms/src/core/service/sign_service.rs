use std::collections::BTreeMap;

use next_web_core::{async_trait, error::BoxError};

use serde::de::DeserializeOwned;
use serde_json::Value;

#[async_trait]
pub trait SignService: Send + Sync {
    /// Creates a new signature record.
    ///
    /// # Arguments
    /// * `sign_name`: The name of the signature.
    /// * `sign_type`: The type of the signature (as `i32`).
    /// * `sign_purpose`: The purpose of the signature (as `i32`).
    /// * `qualification_id`: ID of the associated qualification.
    /// * `remark`: Optional remark or note about the signature.
    /// * `expand_params`: Optional extended parameters as key-value pairs.
    ///
    /// # Returns
    /// * `Ok(R)` if the signature was created successfully.
    /// * `Err(BoxError)` if an error occurred.
    async fn create_sign<'a, R>(
        &self,
        sign_name: &'a str,
        sign_type: i32,
        sign_purpose: i32,
        qualification_id: u64,
        remark: Option<&'a str>,
        expand_params: Option<BTreeMap<&'a str, Value>>,
    ) -> Result<R, BoxError>
    where
        R: DeserializeOwned;

    /// Deletes a signature by its ID.
    ///
    /// # Arguments
    /// * `sign_id`: A reference to the ID of the signature to delete.
    ///
    /// # Returns
    /// * `Ok(R)` if the deletion was successful.
    /// * `Err(BoxError)` if an error occurred.
    async fn delete_sign<R>(&self, sign_id: &str) -> Result<R, BoxError>
    where
        R: DeserializeOwned;

    /// Updates an existing signature record.
    ///
    /// # Arguments
    /// * `sign_name`: The name of the signature.
    /// * `sign_type`: The type of the signature (as `i32`).
    /// * `sign_purpose`: The purpose of the signature (as `i32`).
    /// * `qualification_id`: ID of the associated qualification.
    /// * `remark`: Optional remark or note about the signature.
    /// * `expand_params`: Optional extended parameters as key-value pairs.
    ///
    /// # Returns
    /// * `Ok(R)` if the update was successful.
    /// * `Err(BoxError)` if an error occurred.
    async fn update_sign<'a, R>(
        &self,
        sign_name: &'a str,
        sign_type: i32,
        sign_purpose: i32,
        qualification_id: u64,
        remark: Option<&'a str>,
        expand_params: Option<BTreeMap<&'a str, Value>>,
    ) -> Result<R, BoxError>
    where
        R: DeserializeOwned;

    /// Queries a signature by its ID.
    ///
    /// # Arguments
    /// * `sign_id`: A reference to the ID of the signature to query.
    /// * `expand_params`: Optional extended parameters as key-value pairs.
    ///
    /// # Returns
    /// * `Ok(R)` containing the queried signature data if found.
    /// * `Err(BoxError)` if an error occurred during the query.
    async fn query_sign<R>(
        &self,
        sign_id: &str,
        expand_params: Option<BTreeMap<&str, Value>>,
    ) -> Result<R, BoxError>
    where
        R: DeserializeOwned;
}
