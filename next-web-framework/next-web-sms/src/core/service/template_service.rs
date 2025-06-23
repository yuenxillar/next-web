use std::collections::BTreeMap;

use next_web_core::{async_trait, error::BoxError};

use serde::de::DeserializeOwned;

pub type TemplateResult<T: DeserializeOwned> = std::result::Result<T, BoxError>;

#[async_trait]
pub trait TemplateService: Send + Sync {
    /// CreateSmsTemplate- 申请短信模板
    async fn create_template<'a, R>(
        &self,
        template_name: &'a str,
        template_content: &'a str,
        template_type: i32,
        expand_params: Option<BTreeMap<&'a str, String>>,
    ) -> TemplateResult<R>
    where
        R: DeserializeOwned;
    
    async fn delete_template<R>(&self, template_code: &str) -> TemplateResult<R>
    where
        R: DeserializeOwned;
    
    async fn update_template<'a, R>(
        &self,
        template_name: &'a str,
        template_content: &'a str,
        template_type: i32,
        expand_params: Option<BTreeMap<&'a str, String>>,
    ) -> TemplateResult<R>
    where
        R: DeserializeOwned;

    
    async fn query_template<R>(&self, template_type: i32, index: u16, size: u16) -> TemplateResult<R>
    where
        R: DeserializeOwned;
}
