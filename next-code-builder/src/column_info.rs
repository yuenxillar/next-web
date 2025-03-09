#[derive(serde::Deserialize, Debug)]
pub struct ColumnInfo {
    #[serde(rename = "COLUMN_NAME")]
    pub column_name: Option<String>,
    #[serde(rename = "COLUMN_TYPE")]
    pub column_type: Option<String>,
    #[serde(rename = "IS_NULLABLE")]
    pub is_nullable: Option<String>,
    #[serde(rename = "COLUMN_DEFAULT")]
    pub column_default: Option<String>,
    #[serde(rename = "COLUMN_COMMENT")]
    pub column_comment: Option<String>,
}
