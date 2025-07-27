use syn::{FnArg, ItemFn, Pat, PatIdent, Type, TypeReference};



pub(crate) struct ParamInfo {
    pub(crate)  name: String,
    pub(crate)  is_reference: bool,
    pub(crate)  is_mut_ref: bool,
    pub(crate)  ty: Type,
}
pub(crate) fn extract_param_info(item_fn: &ItemFn) -> Vec<ParamInfo> {
    item_fn.sig.inputs.iter().filter_map(|arg| {
        match arg {
            FnArg::Typed(pat_type) => {
                // 获取参数名
                let name = match &*pat_type.pat {
                    Pat::Ident(PatIdent { ident, .. }) => {
                        let param_name = ident.to_string();
                        if param_name.starts_with("_") {
                            return None;
                        }
                        param_name
                    },
                    _ => return None,
                };
                
                // 检查是否是引用类型
                let (is_reference, is_mut_ref) = match &*pat_type.ty {
                    Type::Reference(TypeReference { mutability, .. }) => {
                        (true, mutability.is_some())
                    }
                    _ => (false, false),
                };
                
                // 获取完整类型
                let ty = (*pat_type.ty).clone();
                
                Some(ParamInfo {
                    name,
                    is_reference,
                    is_mut_ref,
                    ty,
                })
            }
            FnArg::Receiver(_) => None, // 忽略self参数
        }
    }).collect()
}