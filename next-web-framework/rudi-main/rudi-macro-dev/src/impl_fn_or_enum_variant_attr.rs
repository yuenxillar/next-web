use from_attr::FromAttr;

#[derive(FromAttr)]
#[attribute(idents = [resource])]
pub(crate) struct ImplFnOrEnumVariantAttr;
