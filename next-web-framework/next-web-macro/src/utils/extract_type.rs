use syn::{
    ReturnType, Type, PathArguments, GenericArgument
};

pub(crate) fn extract_error_type(return_type: &ReturnType) -> Option<Type> {
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