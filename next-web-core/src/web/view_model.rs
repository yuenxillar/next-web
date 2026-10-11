//! The model a view of a request is rendered with.

use serde::Serialize;
use tera::{Context, Value};

use crate::error::BoxError;

/// The model a view is rendered with.
///
/// The model holds the variables of a template and their [`Value`], which is
/// what the engine of a view renders. It is the counterpart of the attributes
/// of a request: an attribute is a value a filter or a handler passes around,
/// and it may hold anything, while a [`ViewModel`] only holds what a view
/// renders and is therefore a separate store on the request.
///
/// The model is reached through
/// [`HttpRequest::view_model_mut`](crate::traits::http::http_request::HttpRequest::view_model_mut),
/// which is how a filter hands the variables of an error page to a dispatch:
///
/// ```ignore
/// if let Some(model) = request.view_model_mut() {
///     model.insert("error", Value::from("Bad credentials"));
/// }
/// ```
///
/// # Examples
///
/// ```rust
/// use next_web_core::tera::Value;
/// use next_web_core::web::ViewModel;
///
/// let mut model = ViewModel::new();
/// model.insert("user", Value::from("Alice"));
///
/// assert_eq!(model.get("user").and_then(Value::as_str), Some("Alice"));
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ViewModel {
    /// The variables of the model, in the shape the engine renders.
    context: Context,
}

impl ViewModel {
    /// Creates an empty model.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a model holding the fields of `value`.
    ///
    /// The fields of a struct or the entries of a map become the variables of
    /// the model, so a model of an application object does not have to name its
    /// variables one by one.
    ///
    /// # Errors
    ///
    /// Returns an error when `value` cannot be read as a value of a template.
    pub fn from_serialize<T: Serialize + ?Sized>(value: &T) -> Result<Self, BoxError> {
        Ok(Self {
            context: Context::from_serialize(value)?,
        })
    }

    /// Creates a model that holds `context`.
    pub fn from_context(context: Context) -> Self {
        Self { context }
    }

    /// Returns the variables of the model as the context of the engine.
    pub fn context(&self) -> &Context {
        &self.context
    }

    /// Returns the variables of the model as the context of the engine,
    /// mutably, so a caller that works with the engine can write to them.
    pub fn context_mut(&mut self) -> &mut Context {
        &mut self.context
    }

    /// Returns the variables of the model, which the engine renders.
    pub fn into_context(self) -> Context {
        self.context
    }

    /// Binds `value` to `name`.
    ///
    /// An existing variable of the same name is replaced.
    pub fn insert(&mut self, name: impl Into<String>, value: Value) {
        self.context.insert_value(name.into(), value);
    }

    /// Returns the variable bound to `name`, if any.
    pub fn get(&self, name: &str) -> Option<&Value> {
        self.context.get(name)
    }

    /// Returns whether a variable is bound to `name`.
    pub fn contains(&self, name: &str) -> bool {
        self.context.contains_key(name)
    }

    /// Removes the variable bound to `name` and returns it, if any.
    pub fn remove(&mut self, name: &str) -> Option<Value> {
        self.context.remove(name)
    }

    /// Adds every variable of `other` to the model.
    ///
    /// A variable that both models hold is taken from `other`.
    pub fn extend(&mut self, other: ViewModel) {
        self.context.extend(other.context);
    }

    /// Returns whether the model holds no variable.
    pub fn is_empty(&self) -> bool {
        self.context == Context::new()
    }
}

impl From<Context> for ViewModel {
    fn from(context: Context) -> Self {
        Self::from_context(context)
    }
}

impl From<ViewModel> for Context {
    fn from(model: ViewModel) -> Self {
        model.into_context()
    }
}

#[cfg(test)]
mod tests {
    use tera::{Context, Value};

    use super::ViewModel;

    #[test]
    fn a_variable_is_bound_read_and_removed() {
        let mut model = ViewModel::new();

        assert!(model.is_empty());
        assert!(!model.contains("user"));

        model.insert("user", Value::from("Alice"));

        assert!(!model.is_empty());
        assert!(model.contains("user"));
        assert_eq!(model.get("user").and_then(Value::as_str), Some("Alice"));
        assert_eq!(model.remove("user"), Some(Value::from("Alice")));
        assert!(model.get("user").is_none());
    }

    #[test]
    fn a_bound_variable_replaces_the_variable_of_the_same_name() {
        let mut model = ViewModel::new();
        model.insert("count", Value::from(1));
        model.insert("count", Value::from(2));

        assert_eq!(model.get("count").and_then(Value::as_i64), Some(2));
    }

    #[test]
    fn a_model_can_be_built_from_the_fields_of_a_value() {
        #[derive(serde::Serialize)]
        struct User {
            name: String,
            admin: bool,
        }

        let model = ViewModel::from_serialize(&User {
            name: "Alice".to_owned(),
            admin: true,
        })
        .expect("the user is read as a model");

        assert_eq!(model.get("name").and_then(Value::as_str), Some("Alice"));
        assert_eq!(model.get("admin").and_then(Value::as_bool), Some(true));
    }

    #[test]
    fn the_variables_of_another_model_are_added() {
        let mut model = ViewModel::new();
        model.insert("first", Value::from(1));

        let mut other = ViewModel::new();
        other.insert("second", Value::from(2));
        other.insert("first", Value::from(3));

        model.extend(other);

        assert_eq!(model.get("first").and_then(Value::as_i64), Some(3));
        assert_eq!(model.get("second").and_then(Value::as_i64), Some(2));
    }

    #[test]
    fn the_context_of_a_model_is_the_context_of_the_engine() {
        let context: Context = ViewModel::from_context(Context::new()).into();

        assert_eq!(context, Context::new());
    }
}
