use std::sync::Arc;

use crate::core::{simple_granted_authority::SimpleGrantedAuthority, GrantedAuthority};

pub struct AuthorityUtils;

impl AuthorityUtils {
    pub fn no_authorities() -> Vec<Arc<dyn GrantedAuthority>> {
        Vec::new()
    }

    pub fn create_authority_list<S>(
        authorities: impl IntoIterator<Item = S>,
    ) -> Vec<Arc<dyn GrantedAuthority>>
    where
        S: AsRef<str>,
    {
        authorities
            .into_iter()
            .map(|authority| {
                Arc::new(SimpleGrantedAuthority::new(authority.as_ref()))
                    as Arc<dyn GrantedAuthority>
            })
            .collect()
    }
}
