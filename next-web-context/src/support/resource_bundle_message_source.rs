use std::collections::HashMap;

use crate::support::BaseResourceBasedMessageSource;

#[derive(Clone, Default)]
pub struct ResourceBundleMessageSource {
    cachedResourceBundles: HashMap<String, HashMap<String, String>>,
    base: BaseResourceBasedMessageSource,
}
