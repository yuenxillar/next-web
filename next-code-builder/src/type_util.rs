pub struct TypeUtil;

impl TypeUtil {
    pub fn match_field_type(field: &str) -> String {
        let binding = field.to_uppercase();
        let str = binding.as_str();
        if str.contains("VARCHAR") {
            return "String".to_string();
        } else if str.contains("BIGINT") {
            return "u64".to_string();
        } else if str.contains("INT") {
            return "u32".to_string();
        } else if str.contains("DATETIME") || str.contains("TIMESTAMP") {
            return "chrono::NaiveDateTime".to_string();
        } else if str.contains("DATE") {
            return "chrono::NaiveDate".to_string();
        } else if str.contains("DECIMAL") {
            return "rust_decimal::Decimal".to_string();
        } else if str.contains("DOUBLE") {
            return "f64".to_string();
        } else if str.contains("FLOAT") {
            return "f32".to_string();
        } else if str.contains("TEXT") {
            return "String".to_string();
        } else if str.contains("BLOB") {
            return "Vec<u8>".to_string();
        } else if str.contains("BOOLEAN") {
            return "bool".to_string();
        } else if str.contains("JSON") {
            return "serde_json::Value".to_string();
        } else if str.contains("ENUM") {
            return "String".to_string();
        } else if str.contains("SET") {
            return "String".to_string();
        } else if str.contains("BIT") {
            return "u8".to_string();
        } else if str.contains("TINYINT") {
            return "u8".to_string();
        }

        
        if str.contains("SERIAL") {
            return "i32".to_string(); // PostgreSQL 的自增类型
        } else if str.contains("UUID") {
            return "uuid::Uuid".to_string(); // 使用 uuid 库
        } else if str.contains("MONEY") {
            return "rust_decimal::Decimal".to_string(); // 使用 rust_decimal 库
        }
        return "String".to_string();
    }
}
