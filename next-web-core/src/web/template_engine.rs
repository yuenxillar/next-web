//! The template engine of the application.
//!
//! The templates of an application are the resources of its `resources`
//! directory whose extension marks them as templates. They are compiled once
//! from the [`ResourceLoader`] of the application, which is what keeps a
//! template in the same place as every other resource and lets one template
//! extend or include another one by its resource path.
//!
//! ```ignore
//! let engine = TemplateEngine::shared();
//! let mut context = Context::new();
//! context.insert("title", "Home");
//!
//! let document = engine.render("home.html", &context)?;
//! ```

use std::{collections::HashMap, sync::OnceLock};

use tera::Value;
use tera::{Context, Tera};

use crate::{
    any::{AnyValue, Number},
    error::BoxError,
    io::{ResourceLoader, web_resource_loader},
    traits::http::http_request::HttpRequest,
};

/// The extensions that mark a resource as a template.
pub const TEMPLATE_EXTENSIONS: &[&str] = &["html", "htm", "tera", "tpl"];

/// The templates of the application, compiled from a [`ResourceLoader`].
///
/// The names of the templates are the paths of the resources they were read
/// from, so a view named `login.html` renders the resource `login.html` of the
/// loader.
pub struct TemplateEngine {
    /// The compiled templates.
    tera: Tera,
}

impl TemplateEngine {
    /// Creates an engine that holds no template.
    pub fn empty() -> Self {
        Self {
            tera: Tera::default(),
        }
    }

    /// Compiles every template of `loader`.
    ///
    /// The templates are compiled together, so one of them can extend or
    /// include another one by its path.
    ///
    /// # Errors
    ///
    /// Returns an error when a template of the loader cannot be read, or when
    /// one of them is not a valid template or refers to a template the loader
    /// does not hold.
    pub fn from_loader(loader: &dyn ResourceLoader) -> Result<Self, BoxError> {
        let mut templates = Vec::new();

        for path in loader.paths() {
            let path: &str = path.as_ref();
            if !is_template(path) {
                continue;
            }

            let content = loader
                .get_resource(path)
                .map_err(|error| format!("the template `{path}` could not be read: {error}"))?
                .get_content_as_string()
                .map_err(|error| format!("the template `{path}` could not be read: {error}"))?;

            templates.push((path.to_owned(), content));
        }

        let mut tera = Tera::default();
        tera.add_raw_templates(templates)?;

        Ok(Self { tera })
    }

    /// Returns the engine of the shared resource loader.
    ///
    /// The templates are compiled the first time the engine is used and are
    /// kept for the rest of the process. A loader that fails to compile leaves
    /// an empty engine behind, so a missing template is reported when it is
    /// rendered instead of when the framework starts.
    pub fn shared() -> &'static TemplateEngine {
        static SHARED: OnceLock<TemplateEngine> = OnceLock::new();

        SHARED.get_or_init(
            || match TemplateEngine::from_loader(web_resource_loader().as_ref()) {
                Ok(engine) => engine,
                Err(error) => {
                    tracing::warn!(
                        "the templates of the application could not be compiled: {error}"
                    );
                    TemplateEngine::empty()
                }
            },
        )
    }

    /// Returns the names of the templates, which are the paths of the resources
    /// they were read from.
    pub fn names(&self) -> impl Iterator<Item = &str> + '_ {
        self.tera.get_template_names()
    }

    /// Returns whether a template named `name` can be rendered.
    pub fn contains(&self, name: &str) -> bool {
        self.resolve(name).is_some()
    }

    /// Renders the template named `name` with `context`.
    ///
    /// # Errors
    ///
    /// Returns an error when the template is not known, or when rendering it
    /// fails.
    pub fn render(&self, name: &str, context: &Context) -> Result<String, BoxError> {
        let resolved = self
            .resolve(name)
            .ok_or_else(|| format!("the template `{name}` was not found"))?;

        Ok(self.tera.render(&resolved, context)?)
    }

    /// Builds the model of a request from its attributes.
    ///
    /// The attributes become the variables of the template, so an attribute
    /// named `error` is what `{{ error }}` renders and an attribute that holds a
    /// map or a vec is turned into the object or the array of the same shape.
    /// The attributes a request carries are the ones a filter stored with
    /// [`set_attribute`](HttpRequest::set_attribute), which is how a filter
    /// hands the model of an error page to a dispatch.
    ///
    /// An attribute that holds a type-erased object has no representation in a
    /// template and becomes null.
    pub fn context_of(request: &dyn HttpRequest) -> Context {
        let mut context = Context::new();

        if let Some(attributes) = request.attributes() {
            for (name, value) in attributes {
                context.insert_value(name.clone(), value_of(value));
            }
        }

        context
    }

    /// Returns the template a view name refers to.
    ///
    /// A name that already names a template is used as it is; otherwise the
    /// extensions of a template are tried, so a view named `login` renders the
    /// resource `login.html`.
    fn resolve(&self, name: &str) -> Option<String> {
        if self.tera.contains_template(name) {
            return Some(name.to_owned());
        }

        TEMPLATE_EXTENSIONS.iter().find_map(|extension| {
            let candidate = format!("{name}.{extension}");
            self.tera.contains_template(&candidate).then_some(candidate)
        })
    }
}

impl std::fmt::Debug for TemplateEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TemplateEngine")
            .field("templates", &self.names().collect::<Vec<_>>())
            .finish()
    }
}

/// Returns whether the extension of the resource marks it as a template.
fn is_template(path: &str) -> bool {
    std::path::Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            TEMPLATE_EXTENSIONS
                .iter()
                .any(|template| extension.eq_ignore_ascii_case(template))
        })
}

/// Turns the value of a request attribute into the value of a template.
fn value_of(any: &AnyValue) -> Value {
    match any {
        AnyValue::String(value) => Value::from(value.clone()),
        AnyValue::Number(Number::Int(value)) => Value::from(*value),
        AnyValue::Number(Number::UInt(value)) => Value::from(*value),
        AnyValue::Number(Number::Float(value)) => Value::from(*value),
        AnyValue::Boolean(value) => Value::from(*value),
        AnyValue::Vec(values) => Value::from(values.iter().map(value_of).collect::<Vec<_>>()),
        AnyValue::Map(values) => Value::from(
            values
                .iter()
                .map(|(name, value)| (name.clone(), value_of(value)))
                .collect::<HashMap<String, Value>>(),
        ),
        // A value that erases its own type, and the absence of a value, have no
        // representation in a template.
        AnyValue::BoxedValue(_) | AnyValue::Null => Value::none(),
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        fs,
        path::PathBuf,
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
    };

    use tera::Context;
    use tera::Value;

    use super::TemplateEngine;
    use crate::{
        any::AnyValue,
        io::{DefaultResourceLoader, ResourceLoader},
        traits::http::http_request::HttpRequest,
    };

    /// The number of resources directories this process has created.
    static NEXT_DIRECTORY: AtomicUsize = AtomicUsize::new(0);

    /// Creates a loader over a temporary resources directory holding `files`.
    ///
    /// The caller removes the directory it is given when the test is done.
    fn resources(files: &[(&str, &str)]) -> (PathBuf, Arc<DefaultResourceLoader>) {
        let root = std::env::temp_dir().join(format!(
            "next-web-template-engine-{}-{}",
            std::process::id(),
            NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();

        for (name, content) in files {
            let path = root.join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, content).unwrap();
        }

        let mut loader = DefaultResourceLoader::new(&root, None);
        loader.load().unwrap();

        (root, Arc::new(loader))
    }

    #[test]
    fn a_template_is_rendered_with_its_model() {
        let (root, loader) = resources(&[("login.html", "<h1>{{ title }}</h1>")]);
        let engine = TemplateEngine::from_loader(loader.as_ref()).unwrap();

        let mut context = Context::new();
        context.insert("title", "Login");

        assert_eq!(
            engine.render("login.html", &context).unwrap(),
            "<h1>Login</h1>"
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_view_name_finds_the_template_of_the_extension() {
        let (root, loader) = resources(&[("login.html", "login")]);
        let engine = TemplateEngine::from_loader(loader.as_ref()).unwrap();

        assert!(engine.contains("login"));
        assert_eq!(engine.render("login", &Context::new()).unwrap(), "login");

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_template_extends_another_one_by_its_path() {
        let (root, loader) = resources(&[
            ("base.html", "<main>{% block body %}{% endblock %}</main>"),
            (
                "home.html",
                "{% extends \"base.html\" %}{% block body %}home{% endblock %}",
            ),
        ]);
        let engine = TemplateEngine::from_loader(loader.as_ref()).unwrap();

        assert_eq!(
            engine.render("home.html", &Context::new()).unwrap(),
            "<main>home</main>"
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_resource_that_is_not_a_template_is_not_compiled() {
        let (root, loader) = resources(&[("app.css", "body {}"), ("login.html", "login")]);
        let engine = TemplateEngine::from_loader(loader.as_ref()).unwrap();

        assert!(engine.contains("login.html"));
        assert!(!engine.contains("app.css"));
        assert_eq!(engine.names().count(), 1);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_missing_template_is_reported() {
        let (root, loader) = resources(&[("login.html", "login")]);
        let engine = TemplateEngine::from_loader(loader.as_ref()).unwrap();

        assert!(engine.render("missing.html", &Context::new()).is_err());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn the_attributes_of_a_request_become_the_model_of_a_template() {
        let mut request = axum::extract::Request::new(axum::body::Body::empty());
        request.set_attribute("error", AnyValue::from("Bad credentials"));
        request.set_attribute("attempts", AnyValue::from(3));
        request.set_attribute("locked", AnyValue::from(true));

        let context = TemplateEngine::context_of(&request);

        assert_eq!(
            context.get("error").and_then(Value::as_str),
            Some("Bad credentials")
        );
        assert_eq!(context.get("attempts").and_then(Value::as_i64), Some(3));
        assert_eq!(context.get("locked").and_then(Value::as_bool), Some(true));
    }

    #[test]
    fn a_container_attribute_becomes_the_container_of_the_template() {
        let mut request = axum::extract::Request::new(axum::body::Body::empty());
        request.set_attribute(
            "tags",
            AnyValue::Vec(vec![AnyValue::from("first"), AnyValue::from("second")]),
        );
        request.set_attribute(
            "user",
            AnyValue::Map(HashMap::from([(
                "name".to_owned(),
                AnyValue::from("Alice"),
            )])),
        );

        let context = TemplateEngine::context_of(&request);

        assert_eq!(
            context
                .get("tags")
                .and_then(Value::as_array)
                .map(<[_]>::len),
            Some(2)
        );
        let name = context
            .get("user")
            .and_then(Value::as_map)
            .and_then(|user| {
                user.iter()
                    .find(|(name, _)| name.as_str() == Some("name"))
                    .map(|(_, value)| value)
            })
            .and_then(Value::as_str);

        assert_eq!(name, Some("Alice"));
    }

    #[test]
    fn an_object_attribute_becomes_null() {
        let mut request = axum::extract::Request::new(axum::body::Body::empty());
        request.set_attribute("object", AnyValue::BoxedValue(Box::new(7u8)));
        request.set_attribute("nothing", AnyValue::Null);

        let context = TemplateEngine::context_of(&request);

        assert!(context.get("object").is_some_and(Value::is_none));
        assert!(context.get("nothing").is_some_and(Value::is_none));
    }
}
