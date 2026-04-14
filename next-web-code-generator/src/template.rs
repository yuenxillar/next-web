use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

use chrono::Utc;
use serde_json::{Number, Value};

use crate::error::{CodegenError, Result};
use crate::model::{to_camel_case, to_pascal_case, to_snake_case};

/// Extensible template helper contract.
pub trait TemplateHelper: Send + Sync {
    /// Resolve helper output from already-evaluated arguments.
    fn call(&self, args: &[Value]) -> Result<Value>;
}

impl<F> TemplateHelper for F
where
    F: Fn(&[Value]) -> Result<Value> + Send + Sync + 'static,
{
    fn call(&self, args: &[Value]) -> Result<Value> {
        self(args)
    }
}

/// Helper registry used by the lightweight template engine.
#[derive(Clone)]
pub struct TemplateHelperRegistry {
    helpers: BTreeMap<String, Arc<dyn TemplateHelper>>,
}

impl TemplateHelperRegistry {
    /// Create an empty helper registry.
    pub fn new() -> Self {
        Self {
            helpers: BTreeMap::new(),
        }
    }

    /// Create a helper registry with the built-in helpers pre-registered.
    pub fn with_builtins() -> Self {
        let mut registry = Self::new();
        register_builtin_helpers(&mut registry);
        registry
    }

    /// Register or replace a helper by name.
    pub fn register<F>(&mut self, name: impl Into<String>, helper: F)
    where
        F: TemplateHelper + 'static,
    {
        self.helpers.insert(name.into(), Arc::new(helper));
    }

    /// Return whether the registry contains a helper with the given name.
    pub fn contains(&self, name: &str) -> bool {
        self.helpers.contains_key(name)
    }

    fn get(&self, name: &str) -> Option<&Arc<dyn TemplateHelper>> {
        self.helpers.get(name)
    }
}

impl Default for TemplateHelperRegistry {
    fn default() -> Self {
        Self::with_builtins()
    }
}

impl fmt::Debug for TemplateHelperRegistry {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let helper_names = self.helpers.keys().collect::<Vec<_>>();
        formatter
            .debug_struct("TemplateHelperRegistry")
            .field("helpers", &helper_names)
            .finish()
    }
}

/// Lightweight template engine supporting variable interpolation, helper calls and `each` loops.
#[derive(Debug, Default)]
pub struct TemplateEngine;

impl TemplateEngine {
    /// Render a template with the provided JSON context and built-in helpers.
    pub fn render(template: &str, context: &Value) -> Result<String> {
        Self::render_with_helpers(template, context, &TemplateHelperRegistry::default())
    }

    /// Render a template with a custom helper registry.
    pub fn render_with_helpers(
        template: &str,
        context: &Value,
        helpers: &TemplateHelperRegistry,
    ) -> Result<String> {
        render_section(template, context, None, helpers)
    }
}

fn render_section(
    template: &str,
    root: &Value,
    current: Option<&Value>,
    helpers: &TemplateHelperRegistry,
) -> Result<String> {
    let mut output = String::new();
    let mut cursor = 0usize;

    while let Some(relative_start) = template[cursor..].find("{{") {
        let start = cursor + relative_start;
        output.push_str(&template[cursor..start]);

        let tag_end = find_tag_end(template, start)?;
        let tag = template[start + 2..tag_end].trim();

        if let Some(path) = tag.strip_prefix("#each ") {
            let body_start = tag_end + 2;
            let (body, next_cursor) = extract_each_body(template, body_start)?;
            let iterable = evaluate_expression(path, root, current, helpers)?;
            let values = iterable.as_array().ok_or_else(|| {
                CodegenError::Template(format!("expression '{path}' is not an array for #each"))
            })?;

            for value in values {
                output.push_str(&render_section(body, root, Some(value), helpers)?);
            }

            cursor = next_cursor;
            continue;
        }

        if tag == "/each" {
            return Err(CodegenError::Template(
                "unexpected closing tag '{{/each}}'".to_string(),
            ));
        }

        let replacement = render_value(&evaluate_expression(tag, root, current, helpers)?);
        output.push_str(&replacement);
        cursor = tag_end + 2;
    }

    output.push_str(&template[cursor..]);
    Ok(output)
}

fn render_inline_template(
    template: &str,
    context: &Value,
    helpers: &TemplateHelperRegistry,
) -> Result<String> {
    let mut output = String::new();
    let mut cursor = 0usize;

    while let Some(relative_start) = template[cursor..].find("{{") {
        let start = cursor + relative_start;
        output.push_str(&template[cursor..start]);

        let tag_end = find_tag_end(template, start)?;
        let tag = template[start + 2..tag_end].trim();
        if tag.starts_with("#each ") || tag == "/each" {
            return Err(CodegenError::Template(
                "inline templates do not support '{{#each}}' blocks".to_string(),
            ));
        }

        let replacement = render_value(&evaluate_expression(tag, context, None, helpers)?);
        output.push_str(&replacement);
        cursor = tag_end + 2;
    }

    output.push_str(&template[cursor..]);
    Ok(output)
}

fn find_tag_end(template: &str, start: usize) -> Result<usize> {
    template[start + 2..]
        .find("}}")
        .map(|pos| start + 2 + pos)
        .ok_or_else(|| CodegenError::Template("missing '}}'".to_string()))
}

fn extract_each_body<'a>(template: &'a str, body_start: usize) -> Result<(&'a str, usize)> {
    let mut cursor = body_start;
    let mut depth = 1usize;

    while let Some(relative_start) = template[cursor..].find("{{") {
        let start = cursor + relative_start;
        let tag_end = find_tag_end(template, start)?;
        let tag = template[start + 2..tag_end].trim();
        if tag.starts_with("#each ") {
            depth += 1;
        } else if tag == "/each" {
            depth -= 1;
            if depth == 0 {
                return Ok((&template[body_start..start], tag_end + 2));
            }
        }

        cursor = tag_end + 2;
    }

    Err(CodegenError::Template(
        "unclosed '{{#each ...}}' block".to_string(),
    ))
}

fn evaluate_expression(
    expression: &str,
    root: &Value,
    current: Option<&Value>,
    helpers: &TemplateHelperRegistry,
) -> Result<Value> {
    let expression = expression.trim();
    if expression.is_empty() {
        return Ok(Value::Null);
    }

    if let Some(value) = resolve_value(root, current, expression) {
        return Ok(value.clone());
    }

    let tokens = tokenize_expression(expression)?;
    let Some((helper_name, arguments)) = tokens.split_first() else {
        return Ok(Value::Null);
    };

    let ExpressionToken::Raw(helper_name) = helper_name else {
        return Ok(Value::Null);
    };
    let Some(helper) = helpers.get(helper_name) else {
        return Ok(Value::Null);
    };

    let resolved_arguments = arguments
        .iter()
        .map(|token| resolve_argument(root, current, token))
        .collect::<Result<Vec<_>>>()?;
    helper.call(&resolved_arguments)
}

fn resolve_value<'a>(root: &'a Value, current: Option<&'a Value>, path: &str) -> Option<&'a Value> {
    let path = path.trim();
    if path.is_empty() {
        return None;
    }

    if path == "this" {
        return current;
    }

    if let Some(current) = current {
        if let Some(path) = path.strip_prefix("this.") {
            return resolve_path(current, path);
        }

        if let Some(value) = resolve_path(current, path) {
            return Some(value);
        }
    }

    resolve_path(root, path)
}

fn resolve_path<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    let mut current = value;
    for segment in path.split('.').filter(|segment| !segment.is_empty()) {
        match current {
            Value::Object(object) => {
                current = object.get(segment)?;
            }
            _ => return None,
        }
    }
    Some(current)
}

fn resolve_argument(
    root: &Value,
    current: Option<&Value>,
    token: &ExpressionToken,
) -> Result<Value> {
    match token {
        ExpressionToken::StringLiteral(value) => Ok(Value::String(value.clone())),
        ExpressionToken::Raw(path) => {
            if let Some(value) = resolve_value(root, current, path) {
                return Ok(value.clone());
            }

            if path.eq_ignore_ascii_case("null") {
                return Ok(Value::Null);
            }
            if path.eq_ignore_ascii_case("true") {
                return Ok(Value::Bool(true));
            }
            if path.eq_ignore_ascii_case("false") {
                return Ok(Value::Bool(false));
            }
            if let Ok(number) = path.parse::<i64>() {
                return Ok(Value::Number(Number::from(number)));
            }
            if let Ok(number) = path.parse::<f64>() {
                if let Some(number) = Number::from_f64(number) {
                    return Ok(Value::Number(number));
                }
            }

            Ok(Value::String(path.clone()))
        }
    }
}

fn render_value(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(text) => text.clone(),
        Value::Bool(flag) => flag.to_string(),
        Value::Number(number) => number.to_string(),
        Value::Array(_) | Value::Object(_) => {
            serde_json::to_string_pretty(value).unwrap_or_default()
        }
    }
}

fn stringify_value(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(text) => text.clone(),
        Value::Bool(flag) => flag.to_string(),
        Value::Number(number) => number.to_string(),
        Value::Array(_) | Value::Object(_) => serde_json::to_string(value).unwrap_or_default(),
    }
}

fn is_empty_value(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::String(text) => text.trim().is_empty(),
        Value::Array(values) => values.is_empty(),
        Value::Object(values) => values.is_empty(),
        Value::Bool(_) | Value::Number(_) => false,
    }
}

fn register_builtin_helpers(registry: &mut TemplateHelperRegistry) {
    registry.register("snake", |args: &[Value]| {
        Ok(Value::String(to_snake_case(&required_string(
            args, 0, "snake",
        )?)))
    });
    registry.register("camel", |args: &[Value]| {
        Ok(Value::String(to_camel_case(&required_string(
            args, 0, "camel",
        )?)))
    });
    registry.register("pascal", |args: &[Value]| {
        Ok(Value::String(to_pascal_case(&required_string(
            args, 0, "pascal",
        )?)))
    });
    registry.register("upper", |args: &[Value]| {
        Ok(Value::String(
            required_string(args, 0, "upper")?.to_uppercase(),
        ))
    });
    registry.register("lower", |args: &[Value]| {
        Ok(Value::String(
            required_string(args, 0, "lower")?.to_lowercase(),
        ))
    });
    registry.register("replace", |args: &[Value]| {
        let value = required_string(args, 0, "replace")?;
        let from = required_string(args, 1, "replace")?;
        let to = required_string(args, 2, "replace")?;
        Ok(Value::String(value.replace(&from, &to)))
    });
    registry.register("default", |args: &[Value]| {
        let first = args.first().cloned().unwrap_or(Value::Null);
        if is_empty_value(&first) {
            Ok(args.get(1).cloned().unwrap_or(Value::Null))
        } else {
            Ok(first)
        }
    });
    registry.register("timestamp", |_args: &[Value]| {
        Ok(Value::Number(Number::from(Utc::now().timestamp())))
    });
    registry.register("timestamp_millis", |_args: &[Value]| {
        Ok(Value::Number(Number::from(Utc::now().timestamp_millis())))
    });
    registry.register("format_now", |args: &[Value]| {
        let pattern = args
            .first()
            .map(stringify_value)
            .unwrap_or_else(|| "%Y-%m-%d %H:%M:%S".to_string());
        Ok(Value::String(Utc::now().format(&pattern).to_string()))
    });
}

fn required_string(args: &[Value], index: usize, helper_name: &str) -> Result<String> {
    args.get(index).map(stringify_value).ok_or_else(|| {
        CodegenError::Template(format!(
            "helper '{helper_name}' is missing argument {index}"
        ))
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ExpressionToken {
    Raw(String),
    StringLiteral(String),
}

fn tokenize_expression(expression: &str) -> Result<Vec<ExpressionToken>> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    let mut escaped = false;

    for ch in expression.chars() {
        if let Some(active_quote) = quote {
            if escaped {
                current.push(ch);
                escaped = false;
                continue;
            }

            if ch == '\\' {
                escaped = true;
                continue;
            }

            if ch == active_quote {
                tokens.push(ExpressionToken::StringLiteral(std::mem::take(&mut current)));
                quote = None;
                continue;
            }

            current.push(ch);
            continue;
        }

        if ch == '"' || ch == '\'' {
            if !current.trim().is_empty() {
                tokens.push(ExpressionToken::Raw(std::mem::take(&mut current)));
            }
            quote = Some(ch);
            continue;
        }

        if ch.is_whitespace() {
            if !current.is_empty() {
                tokens.push(ExpressionToken::Raw(std::mem::take(&mut current)));
            }
            continue;
        }

        current.push(ch);
    }

    if escaped || quote.is_some() {
        return Err(CodegenError::Template(
            "unterminated quoted string in template expression".to_string(),
        ));
    }

    if !current.is_empty() {
        tokens.push(ExpressionToken::Raw(current));
    }

    Ok(tokens)
}

/// Render plain text containing `{{variable}}` placeholders with built-in helpers.
pub fn render_string_template(template: &str, context: &Value) -> Result<String> {
    render_string_template_with_helpers(template, context, &TemplateHelperRegistry::default())
}

/// Render plain text containing `{{variable}}` placeholders with a custom helper registry.
pub fn render_string_template_with_helpers(
    template: &str,
    context: &Value,
    helpers: &TemplateHelperRegistry,
) -> Result<String> {
    render_inline_template(template, context, helpers)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        TemplateEngine, TemplateHelperRegistry, render_string_template,
        render_string_template_with_helpers,
    };

    #[test]
    fn renders_simple_variables() {
        let rendered = TemplateEngine::render(
            "hello {{project.name}}",
            &json!({
                "project": { "name": "demo" }
            }),
        )
        .unwrap();

        assert_eq!(rendered, "hello demo");
    }

    #[test]
    fn renders_each_blocks() {
        let rendered = TemplateEngine::render(
            "{{#each tables}}{{name}}:{{pascal_name}}\n{{/each}}",
            &json!({
                "tables": [
                    { "name": "sys_user", "pascal_name": "SysUser" },
                    { "name": "sys_role", "pascal_name": "SysRole" }
                ]
            }),
        )
        .unwrap();

        assert_eq!(rendered, "sys_user:SysUser\nsys_role:SysRole\n");
    }

    #[test]
    fn renders_builtin_helpers() {
        let rendered = TemplateEngine::render(
            "{{snake table.pascal_name}}|{{pascal table.module_name}}|{{default table.comment \"N/A\"}}",
            &json!({
                "table": {
                    "module_name": "sys_user",
                    "pascal_name": "SysUser",
                    "comment": null
                }
            }),
        )
        .unwrap();

        assert_eq!(rendered, "sys_user|SysUser|N/A");
    }

    #[test]
    fn renders_path_templates_with_helpers() {
        let rendered = render_string_template(
            "src/models/{{snake table.pascal_name}}.rs",
            &json!({
                "table": { "pascal_name": "SysUser" }
            }),
        )
        .unwrap();

        assert_eq!(rendered, "src/models/sys_user.rs");
    }

    #[test]
    fn renders_custom_helpers() {
        let mut helpers = TemplateHelperRegistry::with_builtins();
        helpers.register("suffix", |args: &[serde_json::Value]| {
            let value = args
                .first()
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            Ok(json!(format!("{value}_entity")))
        });

        let rendered = render_string_template_with_helpers(
            "src/models/{{suffix table.module_name}}.rs",
            &json!({
                "table": { "module_name": "sys_user" }
            }),
            &helpers,
        )
        .unwrap();

        assert_eq!(rendered, "src/models/sys_user_entity.rs");
    }
}
