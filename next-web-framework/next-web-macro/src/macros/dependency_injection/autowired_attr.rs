use from_attr::FromAttr;
use syn::{parse_quote, Path};

// #[di(path = path::to::rudi)]

#[derive(FromAttr)]
#[attribute(idents = [autowired])]
pub(crate) struct AutoWiredAttr {
    #[attribute(default = default_path())]
    pub(crate) path: Path,
}

fn default_path() -> Path {
    parse_quote!(::rudi)
}

impl Default for AutoWiredAttr {
    fn default() -> Self {
        Self {
            path: default_path(),
        }
    }
}
