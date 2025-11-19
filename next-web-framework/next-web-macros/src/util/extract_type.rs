use syn::{GenericArgument, PathArguments, ReturnType, Type};

pub fn extract_result_error_type(return_type: &ReturnType) -> Option<Type> {
    if let ReturnType::Type(_, ty) = return_type {
        if let Type::Path(type_path) = &**ty {
            if let Some(segment) = type_path.path.segments.last() {
                if segment.ident == "Result" {
                    if let PathArguments::AngleBracketed(args) = &segment.arguments {
                        if args.args.len() >= 2 {
                            if let GenericArgument::Type(error_ty) = &args.args[1] {
                                return Some(error_ty.clone());
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

pub fn extract_option_inner_type(ty: &syn::Type) -> Option<syn::Type> {
    // 检查是否为 Option 类型
    // Check if it is of Option type
    if let syn::Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            if segment.ident == "Option" {
                // 提取泛型参数
                // Extract generic parameters
                if let PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(GenericArgument::Type(inner_type)) = args.args.first() {
                        return Some(inner_type.clone());
                    }
                }
            }
        }
    }

    None
}
