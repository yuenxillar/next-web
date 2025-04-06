use from_attr::FromAttr;
use syn::{parse_quote, Path};

// #[di(rudi_path = path::to::rudi)]

#[derive(FromAttr)]
#[attribute(idents = [autowired])]
pub(crate) struct AutowiredAttr {
    #[attribute(default = default_path())]
    pub(crate) path: Path,
}

fn default_path() -> Path {
    parse_quote!(::next_web_core)
}

impl Default for AutowiredAttr {
    fn default() -> Self {
        Self {
            path: default_path(),
        }
    }
}
