pub struct TypeInfo;


impl TypeInfo {
    pub fn is_number(s: &str) -> bool {
        match s {
            "u8" | "u16" | "u32" | "u64" | "u128" | "usize" | "i8" | "i16" | "i32" | "i64" | "i128"
            | "isize" | "f32" | "f64" => true,
            _ => false,
        }
    }

    pub fn is_string(s: &str) -> bool {
        s == "String"
    }
}
