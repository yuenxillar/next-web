use std::sync::Arc;

use crate::core::{
    granted_authority::GrantedAuthority, simple_granted_authority::SimpleGrantedAuthority,
};

pub struct AuthorityUtils;

impl AuthorityUtils {
    pub fn no_authorities() -> Vec<Arc<dyn GrantedAuthority>> {
        Vec::new()
    }

    pub fn create_authority_list(
        authorities: impl IntoIterator<Item = impl Into<String>>,
    ) -> Vec<Arc<dyn GrantedAuthority>> {
        authorities
            .into_iter()
            .map(|authority| {
                Arc::new(SimpleGrantedAuthority::new(authority)) as Arc<dyn GrantedAuthority>
            })
            .collect()
    }
}
