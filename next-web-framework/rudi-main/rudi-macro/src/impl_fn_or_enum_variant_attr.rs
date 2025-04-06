use from_attr::FromAttr;

#[derive(FromAttr)]
#[attribute(idents = [autowired])]
pub(crate) struct ImplFnOrEnumVariantAttr;
