use next_web::core::env::{
    Binder, ConfigurableEnvironment, MapPropertySource, StandardEnvironment,
};
use next_web::core::util::indexmap::IndexMap;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Datasource {
    url: String,
}

/// Creates an environment holding the URL of a datasource.
fn environment(url: String) -> StandardEnvironment {
    let mut environment = StandardEnvironment::new();
    environment
        .property_sources()
        .add_first(Box::new(MapPropertySource::new(
            "test".to_owned(),
            IndexMap::from([("next.datasource.url".to_owned(), url)]),
        )));

    environment
}

/// Binds the datasource of the given environment.
fn bind(environment: &StandardEnvironment) -> Result<Datasource, next_web::core::env::BindError> {
    Binder::new(environment, "next.datasource").bind()
}

/// Returns a variable name that is unique to the test run, so the tests do not
/// have to change the environment of the process.
fn missing_variable(label: &str) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();

    format!(
        "NEXT_WEB_TEST_MISSING_{label}_{}_{}",
        std::process::id(),
        nanos
    )
}

#[test]
fn resolves_the_environment_variable_of_a_placeholder() {
    let Ok(path) = std::env::var("PATH") else {
        return;
    };

    let datasource = bind(&environment("#{PATH:fallback}".to_owned())).unwrap();

    assert_eq!(datasource.url, path);
}

#[test]
fn uses_the_default_value_when_the_variable_is_missing() {
    let name = missing_variable("DEFAULT");

    let datasource = bind(&environment(format!(
        "#{{{name}:jdbc:mysql://localhost:3306/default}}"
    )))
    .unwrap();

    assert_eq!(datasource.url, "jdbc:mysql://localhost:3306/default");
}

#[test]
fn reports_a_variable_that_is_missing_and_has_no_default_value() {
    let name = missing_variable("ERROR");

    let error = bind(&environment(format!("#{{{name}}}"))).unwrap_err();

    assert!(error.message().contains(name.as_str()));
    assert!(error.message().contains("next.datasource.url"));
    assert!(error.message().contains("no default value"));
    assert!(error.to_string().contains("failed to bind `url`"));
}
