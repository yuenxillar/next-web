use from_attr::FromAttr;
use syn::{parse_quote, Path};

// #[resource(path = path::to::next_web_context)]

#[derive(FromAttr)]
#[attribute(idents = [resource])]
pub(crate) struct ResourceAttr {
    #[attribute(default = default_path())]
    pub(crate) path: Path,
}

fn default_path() -> Path {
    parse_quote!(::next_web_context)
}

impl Default for ResourceAttr {
    fn default() -> Self {
        Self {
            path: default_path(),
        }
    }
}
