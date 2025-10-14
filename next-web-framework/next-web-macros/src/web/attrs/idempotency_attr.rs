use from_attr::FromAttr;
use syn::{LitInt, LitStr};

#[derive(FromAttr)]
#[attribute(idents = [_none])]
pub(crate) struct IdempotencyAttr {
    pub name: Option<LitStr>,
    pub key: Option<LitStr>,
    pub cache_key_prefix: Option<LitStr>,
    pub ttl: Option<LitInt>,
}