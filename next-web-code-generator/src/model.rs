use serde::{Deserialize, Serialize};

/// Supported database kinds for metadata introspection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DatabaseKind {
    /// MySQL or compatible protocol.
    Mysql,
    /// PostgreSQL.
    Postgres,
}

/// Database column metadata used by the template layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnInfo {
    /// Raw column name from the database.
    pub name: String,
    /// Rust field name derived from the column name.
    pub rust_name: String,
    /// PascalCase field variant derived from the column name.
    pub pascal_name: String,
    /// Raw database data type.
    pub data_type: String,
    /// Database-specific full column type.
    pub column_type: String,
    /// Rust type used in generated code.
    pub rust_type: String,
    /// Whether the column is nullable.
    pub nullable: bool,
    /// Whether the column participates in a primary key.
    pub primary_key: bool,
    /// Column default expression.
    pub default_value: Option<String>,
    /// Column comment if available.
    pub comment: Option<String>,
    /// Database-specific extra flags.
    pub extra: Option<String>,
}

/// Database table metadata used by the template layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableInfo {
    /// Raw table name from the database.
    pub name: String,
    /// snake_case module name.
    pub module_name: String,
    /// camelCase name.
    pub camel_name: String,
    /// PascalCase type name.
    pub pascal_name: String,
    /// Table comment if available.
    pub comment: Option<String>,
    /// All columns belonging to the table.
    pub columns: Vec<ColumnInfo>,
    /// Primary key column names.
    pub primary_keys: Vec<String>,
}

/// Convert a snake_case or mixed string into snake_case.
pub fn to_snake_case(value: &str) -> String {
    let mut output = String::with_capacity(value.len() + 8);
    let mut previous_is_underscore = false;

    for (index, ch) in value.chars().enumerate() {
        if ch.is_ascii_alphanumeric() {
            if ch.is_ascii_uppercase() {
                if index > 0 && !previous_is_underscore {
                    output.push('_');
                }
                output.push(ch.to_ascii_lowercase());
                previous_is_underscore = false;
            } else {
                output.push(ch.to_ascii_lowercase());
                previous_is_underscore = false;
            }
        } else if !previous_is_underscore && !output.is_empty() {
            output.push('_');
            previous_is_underscore = true;
        }
    }

    output.trim_matches('_').to_string()
}

/// Convert a string into PascalCase.
pub fn to_pascal_case(value: &str) -> String {
    to_snake_case(value)
        .split('_')
        .filter(|segment| !segment.is_empty())
        .map(|segment| {
            let mut chars = segment.chars();
            match chars.next() {
                Some(first) => {
                    let mut part = String::new();
                    part.push(first.to_ascii_uppercase());
                    part.push_str(chars.as_str());
                    part
                }
                None => String::new(),
            }
        })
        .collect::<String>()
}

/// Convert a string into camelCase.
pub fn to_camel_case(value: &str) -> String {
    let pascal = to_pascal_case(value);
    let mut chars = pascal.chars();
    match chars.next() {
        Some(first) => {
            let mut output = String::new();
            output.push(first.to_ascii_lowercase());
            output.push_str(chars.as_str());
            output
        }
        None => String::new(),
    }
}

/// Map a database type into a Rust type used by generated models.
pub fn map_rust_type(kind: DatabaseKind, data_type: &str, nullable: bool) -> String {
    let base = match kind {
        DatabaseKind::Mysql => match data_type.to_ascii_lowercase().as_str() {
            "tinyint" => "i8",
            "smallint" => "i16",
            "mediumint" | "int" | "integer" => "i32",
            "bigint" => "i64",
            "float" => "f32",
            "double" | "decimal" | "numeric" => "f64",
            "bit" | "bool" | "boolean" => "bool",
            "date" => "chrono::NaiveDate",
            "time" => "chrono::NaiveTime",
            "datetime" | "timestamp" => "chrono::NaiveDateTime",
            "json" => "serde_json::Value",
            "binary" | "varbinary" | "blob" | "longblob" | "mediumblob" | "tinyblob" => "Vec<u8>",
            _ => "String",
        },
        DatabaseKind::Postgres => match data_type.to_ascii_lowercase().as_str() {
            "smallint" | "smallserial" => "i16",
            "integer" | "serial" => "i32",
            "bigint" | "bigserial" => "i64",
            "real" => "f32",
            "double precision" | "numeric" | "decimal" => "f64",
            "boolean" => "bool",
            "date" => "chrono::NaiveDate",
            "time without time zone" | "time with time zone" => "chrono::NaiveTime",
            "timestamp without time zone" | "timestamp with time zone" => "chrono::NaiveDateTime",
            "json" | "jsonb" => "serde_json::Value",
            "bytea" => "Vec<u8>",
            _ => "String",
        },
    };

    if nullable {
        format!("Option<{base}>")
    } else {
        base.to_string()
    }
}
