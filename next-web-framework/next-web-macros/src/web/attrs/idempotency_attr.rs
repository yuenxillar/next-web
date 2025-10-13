use from_attr::FromAttr;
use syn::{LitInt, LitStr};

#[derive(FromAttr)]
#[attribute(idents = [find])]
pub(crate) struct IdempotencyAttr {
    pub key: Option<LitStr>,
    pub cache_key_prefix: Option<LitStr>,
    pub ttl: Option<LitInt>,
}

// impl Default for IdempotencyAttr {
//     fn default() -> Self {
//         Self {
//             method_: Default::default(),
//             path: default_path(),
//             headers: default_list(),
//             consume: Default::default(),
//             produce: Default::default(),
//         }
//     }
// }