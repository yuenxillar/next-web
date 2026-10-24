use std::path::{Path, PathBuf};

use crate::config::{GeneratorConfig, ModuleConfig, TemplateConfig};
use crate::context::TemplateContext;
use crate::database::DatabaseIntrospector;
use crate::error::Result;
use crate::model::TableInfo;
use crate::template::{TemplateEngine, render_string_template};

/// A generated file planned or written by the generator.
#[derive(Debug, Clone)]
pub struct GeneratedFile {
    /// File path relative to the configured output root.
    pub relative_path: PathBuf,
    /// Rendered file content.
    pub content: String,
    /// Template logical name.
    pub template_name: String,
    /// Whether this file may overwrite an existing destination.
    pub overwrite: bool,
}

/// High-level generation facade used by the CLI and tests.
#[derive(Debug, Clone)]
pub struct Generator {
    config: GeneratorConfig,
    config_path: PathBuf,
}

impl Generator {
    /// Create a generator from a loaded configuration.
    pub fn new(config: GeneratorConfig, config_path: PathBuf) -> Self {
        Self {
            config,
            config_path,
        }
    }

    /// Load a generator from a `toml` config file path.
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let config = GeneratorConfig::load_from_path(&path)?;
        Ok(Self::new(config, path))
    }

    /// Return the underlying configuration.
    pub fn config(&self) -> &GeneratorConfig {
        &self.config
    }

    /// Read metadata for a specific datasource.
    pub async fn inspect(&self, datasource_name: Option<&str>) -> Result<Vec<TableInfo>> {
        let datasource = self.config.datasource(datasource_name)?;
        DatabaseIntrospector::introspect(datasource).await
    }

    /// Build the strongly typed template context for a specific datasource.
    pub async fn build_template_context(
        &self,
        datasource_name: Option<&str>,
    ) -> Result<TemplateContext> {
        let datasource = self.config.datasource(datasource_name)?;
        let tables = DatabaseIntrospector::introspect(datasource).await?;
        Ok(TemplateContext::new(
            &self.config.project,
            datasource,
            &tables,
        ))
    }

    /// Generate all files for the selected datasource without writing them to disk.
    pub async fn plan(&self, datasource_name: Option<&str>) -> Result<Vec<GeneratedFile>> {
        let context = self.build_template_context(datasource_name).await?;
        self.render_plan(&context)
    }

    /// Render all configured templates using the provided template context.
    pub fn render_plan(&self, context: &TemplateContext) -> Result<Vec<GeneratedFile>> {
        let base_dir = self
            .config_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        let global_context = context.to_value()?;
        let mut files = Vec::new();

        for template in &self.config.templates {
            let source = template.read_template(&base_dir)?;
            if template.per_table {
                for table in &context.tables {
                    let table_context = context.with_table(table).to_value()?;
                    let relative_path =
                        PathBuf::from(render_string_template(&template.output, &table_context)?);
                    let content = TemplateEngine::render(&source, &table_context)?;
                    files.push(GeneratedFile {
                        relative_path,
                        content,
                        template_name: template.name.clone(),
                        overwrite: template.overwrite_enabled(&self.config.project),
                    });
                }
            } else {
                let relative_path =
                    PathBuf::from(render_string_template(&template.output, &global_context)?);
                let content = TemplateEngine::render(&source, &global_context)?;
                files.push(GeneratedFile {
                    relative_path,
                    content,
                    template_name: template.name.clone(),
                    overwrite: template.overwrite_enabled(&self.config.project),
                });
            }
        }

        Ok(files)
    }

    /// Generate and write files for the selected datasource.
    pub async fn generate(&self, datasource_name: Option<&str>) -> Result<Vec<PathBuf>> {
        let files = self.plan(datasource_name).await?;
        let mut written = Vec::with_capacity(files.len());

        for file in files {
            let destination = self.config.project.output_dir.join(&file.relative_path);
            if let Some(parent) = destination.parent() {
                std::fs::create_dir_all(parent)?;
            }
            if destination.exists() && !file.overwrite {
                continue;
            }

            std::fs::write(&destination, file.content)?;
            maintain_module_tree(
                &self.config.project.output_dir,
                &destination,
                &self.config.project.modules,
            )?;
            written.push(destination);
        }

        Ok(written)
    }
}

/// Create an example configuration file and default templates under the provided path.
pub fn initialize_workspace(config_path: &Path, force: bool) -> Result<()> {
    if config_path.exists() && !force {
        return Ok(());
    }

    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(config_path, GeneratorConfig::example())?;

    let base_dir = config_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let template_dir = base_dir.join("templates");
    std::fs::create_dir_all(&template_dir)?;

    for (name, content) in default_templates() {
        std::fs::write(template_dir.join(name), content)?;
    }

    Ok(())
}

fn default_templates() -> [(&'static str, &'static str); 3] {
    [
        (
            "src_lib.rs.tpl",
            r#"pub mod models;
"#,
        ),
        (
            "models_mod.rs.tpl",
            r#"{{#each tables}}pub mod {{module_name}};
{{/each}}
"#,
        ),
        (
            "entity.rs.tpl",
            r#"#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct {{table.pascal_name}} {
{{#each table.columns}}    pub {{rust_name}}: {{rust_type}},
{{/each}}}
"#,
        ),
    ]
}

#[allow(dead_code)]
fn _validate_template(_template: &TemplateConfig) {}

fn maintain_module_tree(
    output_root: &Path,
    destination: &Path,
    modules: &ModuleConfig,
) -> Result<()> {
    if !modules.create_mod_rs && !modules.append_pub_mod {
        return Ok(());
    }

    let is_rust_file = destination
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.eq_ignore_ascii_case("rs"))
        .unwrap_or(false);
    if !is_rust_file {
        return Ok(());
    }

    let file_stem = destination.file_stem().and_then(|stem| stem.to_str());
    let Some(file_stem) = file_stem else {
        return Ok(());
    };

    if file_stem == "mod" {
        ensure_parent_mod_chain(output_root, destination.parent(), modules)?;
        return Ok(());
    }

    if let Some(parent_dir) = destination.parent() {
        ensure_mod_rs(parent_dir, modules)?;
        if modules.append_pub_mod {
            append_pub_mod(parent_dir.join("mod.rs"), file_stem)?;
        }
        ensure_parent_mod_chain(output_root, Some(parent_dir), modules)?;
    }

    Ok(())
}

fn ensure_parent_mod_chain(
    output_root: &Path,
    start_dir: Option<&Path>,
    modules: &ModuleConfig,
) -> Result<()> {
    if !modules.append_pub_mod {
        return Ok(());
    }

    let mut current = start_dir;
    while let Some(dir) = current {
        if dir == output_root {
            break;
        }

        let Some(parent) = dir.parent() else {
            break;
        };
        if parent == output_root
            || parent
                .parent()
                .map(|grand_parent| grand_parent == output_root)
                .unwrap_or(false)
        {
            break;
        }

        let Some(module_name) = dir.file_name().and_then(|name| name.to_str()) else {
            break;
        };

        ensure_mod_rs(parent, modules)?;
        append_pub_mod(parent.join("mod.rs"), module_name)?;
        current = Some(parent);
    }

    Ok(())
}

fn ensure_mod_rs(dir: &Path, modules: &ModuleConfig) -> Result<()> {
    if !modules.create_mod_rs {
        return Ok(());
    }

    let mod_rs = dir.join("mod.rs");
    if !mod_rs.exists() {
        std::fs::write(mod_rs, "")?;
    }

    Ok(())
}

fn append_pub_mod(mod_rs: PathBuf, module_name: &str) -> Result<()> {
    let declaration = format!("pub mod {};", module_name);
    let existing = if mod_rs.exists() {
        std::fs::read_to_string(&mod_rs)?
    } else {
        String::new()
    };

    if existing.lines().any(|line| line.trim() == declaration) {
        return Ok(());
    }

    let mut updated = existing;
    if !updated.is_empty() && !updated.ends_with('\n') {
        updated.push('\n');
    }
    updated.push_str(&declaration);
    updated.push('\n');
    std::fs::write(mod_rs, updated)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{append_pub_mod, maintain_module_tree};
    use crate::config::ModuleConfig;

    #[test]
    fn append_pub_mod_is_idempotent() {
        let dir = temp_dir("append_pub_mod_is_idempotent");
        let mod_rs = dir.join("mod.rs");

        append_pub_mod(mod_rs.clone(), "user").unwrap();
        append_pub_mod(mod_rs.clone(), "user").unwrap();

        let content = std::fs::read_to_string(mod_rs).unwrap();
        assert_eq!(content, "pub mod user;\n");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn maintain_module_tree_creates_and_updates_mod_rs() {
        let root = temp_dir("maintain_module_tree_creates_and_updates_mod_rs");
        let destination = root.join("src").join("models").join("user.rs");
        std::fs::create_dir_all(destination.parent().unwrap()).unwrap();
        std::fs::write(&destination, "pub struct User;").unwrap();

        maintain_module_tree(
            &root,
            &destination,
            &ModuleConfig {
                create_mod_rs: true,
                append_pub_mod: true,
            },
        )
        .unwrap();

        let mod_rs = root.join("src").join("models").join("mod.rs");
        let content = std::fs::read_to_string(mod_rs).unwrap();
        assert_eq!(content, "pub mod user;\n");

        std::fs::remove_dir_all(root).unwrap();
    }

    fn temp_dir(test_name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "next-web-code-generator-{}-{}",
            test_name,
            std::process::id()
        ));
        if path.exists() {
            std::fs::remove_dir_all(&path).unwrap();
        }
        std::fs::create_dir_all(&path).unwrap();
        path
    }
}
