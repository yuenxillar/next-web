#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct {{table.pascal_name}} {
{{#each table.columns}}    pub {{rust_name}}: {{rust_type}},
{{/each}}}
