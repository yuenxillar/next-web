use from_attr::FromAttr;
use syn::{parse_quote, Path};

// #[di(rudi_path = path::to::rudi)]

#[derive(FromAttr)]
#[attribute(idents = [autowired])]
pub(crate) struct AutowiredAttr {
    #[attribute(default = default_rudi_path())]
    pub(crate) rudi_path: Path,
}

fn default_rudi_path() -> Path {
    parse_quote!(::next_web_core)
}

impl Default for AutowiredAttr {
    fn default() -> Self {
        Self {
            rudi_path: default_rudi_path(),
        }
    }
}
