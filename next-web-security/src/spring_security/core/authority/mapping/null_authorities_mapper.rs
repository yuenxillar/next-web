use std::sync::Arc;

use crate::core::{authority::mapping::GrantedAuthoritiesMapper, GrantedAuthority};

#[derive(Clone, Default)]
pub struct NullAuthoritiesMapper;

impl GrantedAuthoritiesMapper for NullAuthoritiesMapper {
    fn map_authorities(
        &self,
        authorities: &[Arc<dyn GrantedAuthority>],
    ) -> Vec<Arc<dyn GrantedAuthority>> {
        authorities.to_vec()
    }
}
