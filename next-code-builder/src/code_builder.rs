use crate::{
    builder_rule::BuilderRule, database_client::DatabaseClient, database_meta::DatabaseMeta,
    tera_context::TeraContext, type_util::TypeUtil, write_text::WriteText,
};
use chrono::Local;
use std::collections::HashMap;

#[derive(Default)]
pub struct CodeBuilder {
    project: String,
    project_dir: String,
    builder_rule: BuilderRule,
    database_meta: DatabaseMeta,
    author: String,
    template_dir: String,
}

pub struct CodeBuilderTemp {
    project: Option<String>,
    project_dir: Option<String>,
    builder_rule: Option<BuilderRule>,
    database_meta: Option<DatabaseMeta>,
    author: Option<String>,
    template_dir: Option<String>,
}

impl Default for CodeBuilderTemp {
    fn default() -> Self {
        Self {
            project: None,
            project_dir: None,
            builder_rule: None,
            database_meta: None,
            author: None,
            template_dir: None,
        }
    }
}

static TEMPLATE_ENTITY: &str = "\t/// ${comment}\n\t ${field_id}: ${field_type}";

impl CodeBuilder {
    pub fn builder() -> CodeBuilderTemp {
        CodeBuilderTemp::default()
    }

    pub async fn code(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // 1. Connect to the database
        let db = DatabaseClient::connection(&self.database_meta).await?;
        db.query_version().await?;

        // 2. Get database metadata
        let table_info = db.query_table_info().await?;
        let order = db.order().await?;
        let table_comment = db.query_table_comment().await?;

        let order_var = db.fields_order(&order);

        // 3. Generate variables
        let (entity, id, fn_param, columns) =
            self.generate_entity_and_columns(&table_info, &order_var)?;

        // 4. Initialize Tera and context
        let tera = tera::Tera::new(&self.template_dir)?;
        let mut context = TeraContext::init();

        // Initialize write text
        let mut write_text = WriteText::new(self.project_dir.clone(), self.builder_rule.clone());

        // 5. Generate code
        let struct_name = Self::to_pascal_case(&self.database_meta.table_name);
        let bean_name = Self::to_camel_case(&self.database_meta.table_name);

        context.set_value("structName", &struct_name);
        context.set_value("entity", &entity);
        context.set_value("id", &id);
        // context.set_value("fn_param", &fn_param);
        context.set_value("comment", &table_comment.as_str().unwrap_or("Null"));
        context.set_value("structSmallName", &self.database_meta.table_name);
        context.set_value("beanName", &bean_name);
        context.set_value("author", &self.author);
        context.set_value("project", &self.project);
        context.set_value("dateTime", &Local::now().format("%Y/%m/%d %H:%M").to_string());
        context.set_value("tableName", &self.database_meta.table_name);
        context.set_value("columns", &columns);
        context.set_value("apiRoute", &Self::camel_to_kebab_case(&struct_name));
        // Write the generated code to files
        write_text.write(&tera, &context.inner, &struct_name, &self.builder_rule);

        Ok(())
    }

    /// Generate entity and columns from table info
    fn generate_entity_and_columns(
        &self,
        table_info: &rbs::Value,
        order_var: &[String],
    ) -> Result<(String, String, String, String), Box<dyn std::error::Error>> {
        let mut entity = String::new();
        let mut id = String::new();
        let mut fn_param = String::new();
        let mut columns = String::new();
        let mut final_str: HashMap<String, String> = HashMap::new();

        if let Some(rows) = table_info.as_array() {
            for (index, row) in rows.iter().enumerate() {
                let mut code = TEMPLATE_ENTITY.to_string();
                let mut is_null = false;
                let mut field_type = String::new();

                for (key, value) in row.as_map().unwrap().0.iter() {
                    let key_str = key.as_str().unwrap();

                    match key_str {
                        "COLUMN_COMMENT" | "column_comment" => {
                            code = code.replace("${comment}", value.as_str().unwrap());
                        }
                        "IS_NULLABLE" | "is_nullable" => {
                            if value.as_str().unwrap() == "YES" {
                                is_null = true;
                            }
                        }
                        "COLUMN_TYPE" | "data_type" => {
                            let type_str = value.as_str().unwrap();
                            field_type = TypeUtil::match_field_type(type_str);

                            if is_null {
                                field_type = format!("Option<{}>", field_type);
                            }
                        }
                        "COLUMN_NAME" | "column_name" => {
                            let field_id = value.as_string().unwrap();

                            if field_id == "id" {
                                id = field_type.clone();
                            }

                            columns.push_str(&format!("{},\n\t\t", &field_id));
                            code = code.replace("${field_id}", &field_id);
                            code = code.replace("${field_type}", &field_type);

                            let param = field_type.replace("\n", "").replace("   ", "");
                            fn_param.push_str(&param);

                            final_str.insert(field_id.clone(), code.clone());
                        }
                        _ => {}
                    }
                }

                if index == rows.len() - 1 {
                    code = code.replace(",\n", "");
                }
            }
        }

        for field in order_var {
            if let Some(code) = final_str.get(field) {
                entity.push_str(code);
            }
        }

        Ok((
            entity,
            id,
            fn_param,
            columns[0..columns.len() - 4].to_string(),
        ))
    }

    /// Convert snake_case to PascalCase
    fn to_pascal_case(name: &str) -> String {
        name.split('_')
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(c) => c.to_uppercase().chain(chars).collect(),
                }
            })
            .collect()
    }

    fn camel_to_kebab_case(input: &str) -> String {
        let mut result = String::new();
    
        for (i, c) in input.chars().enumerate() {
            if c.is_uppercase() {
                // 如果不是第一个字符，在前面添加短横线
                if i != 0 {
                    result.push('-');
                }
                // 将大写字母转换为小写
                result.push(c.to_ascii_lowercase());
            } else {
                result.push(c);
            }
        }
    
        result
    }

    /// Convert snake_case to camelCase
    fn to_camel_case(name: &str) -> String {
        let mut result = String::new();
        for (i, word) in name.split('_').enumerate() {
            if i == 0 {
                result.push_str(word);
            } else {
                let mut chars = word.chars();
                if let Some(c) = chars.next() {
                    result.push_str(&c.to_uppercase().to_string());
                    result.push_str(&chars.collect::<String>());
                }
            }
        }
        result
    }
}

impl CodeBuilderTemp {
    pub fn set_project(mut self, project: String) -> Self {
        self.project = Some(project);
        self
    }

    pub fn set_project_dir(mut self, project_dir: String) -> Self {
        self.project_dir = Some(project_dir);
        self
    }

    pub fn set_builder_rule(mut self, builder_rule: BuilderRule) -> Self {
        self.builder_rule = Some(builder_rule);
        self
    }

    pub fn set_database_meta(mut self, database_meta: DatabaseMeta) -> Self {
        self.database_meta = Some(database_meta);
        self
    }

    pub fn set_author(mut self, author: String) -> Self {
        self.author = Some(author);
        self
    }

    pub fn set_template_dir(mut self, template_dir: String) -> Self {
        self.template_dir = Some(template_dir);
        self
    }

    pub fn build(self) -> CodeBuilder {
        CodeBuilder {
            project: self.project.unwrap_or_default(),
            project_dir: self.project_dir.unwrap_or_default(),
            builder_rule: self.builder_rule.unwrap_or_default(),
            database_meta: self.database_meta.unwrap_or_default(),
            author: self.author.unwrap_or_default(),
            template_dir: self.template_dir.unwrap_or_default(),
        }
    }
}
