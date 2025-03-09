use std::path::PathBuf;

use builder_rule::BuilderRule;
use code_builder::CodeBuilder;
use database_meta::{DatabaseMeta, DatabaseType};

mod builder_rule;
mod code_builder;
mod column_info;
mod database_client;
mod database_meta;
mod tera_context;
mod type_util;
mod write_text;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let project = "next-user-server";

    let database_meta = DatabaseMeta {
        url: String::from("postgres://admin:123456@localhost:5432/next_web"),
        database_name: String::from("next_web"),
        table_name: String::from("media_process"),
        database_type: DatabaseType::default(),
    };

    let binding = std::env::current_dir().unwrap();
    let path = binding.parent().unwrap().to_str().unwrap();
    println!("path: {:?}", path);
    CodeBuilder::builder()
        .set_project(project.to_string())
        .set_project_dir(format!("{}/{project}/src", path))
        .set_builder_rule(BuilderRule::DDD)
        .set_database_meta(database_meta)
        .set_author(String::from("Listening <Test@163.com>"))
        .set_template_dir(
            PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
                .join("templates")
                .join("**")
                .join("*")
                .to_str()
                .unwrap()
                .to_string(),
        )
        .build()
        .code()
        .await
        .ok();
    Ok(())
}
