//! A map keyed by the name an instance was registered under.
//!
//! The point of this test is what its types *do not* do. `Report` implements
//! nothing at all: no `Singleton`, no method that reports a name. The name in the
//! map comes from the context, which registered the instance under it, so any
//! type can be collected by name — where asking the instance for its own name
//! would exclude every type that does not implement the framework's trait.
//!
//! Both maps are declared, because which one a field is written as is the
//! caller's decision: the generated code collects the pairs and lets the declared
//! type take them.

use std::collections::{BTreeMap, HashMap};

use next_web::{
    ConfigurableApplicationContext,
    context::{ApplicationContextExt, DefaultApplicationContext},
    macros::bind::singleton,
};

/// A value that carries no name of its own and implements no trait of the
/// framework.
#[derive(Clone, Debug)]
pub struct Report {
    /// What the report is about.
    pub title: String,
}

/// A provider, so that the context holds an instance of `Report` under a name.
#[singleton]
fn sales_report() -> Report {
    Report {
        title: String::from("sales"),
    }
}

/// Collects the reports of the context, keyed by the names they were registered
/// under.
#[singleton]
#[derive(Clone, Debug)]
pub struct Reports {
    /// The reports in a hash map, which does not order its keys.
    #[autowired(map)]
    by_name: HashMap<String, Report>,

    /// The same instances in a tree map, which the declared type is what turns
    /// the collection into.
    #[autowired(map)]
    ordered: BTreeMap<String, Report>,
}

/// Starts the context of the test.
fn start_context() -> DefaultApplicationContext {
    let mut context = DefaultApplicationContext::default();
    context.insert_singleton_with_name(false, "mainAllowOverride");
    context.refresh().expect("the context refreshes");

    context
}

#[test]
fn a_map_is_keyed_by_the_registered_name_of_a_type_that_knows_nothing_of_singletons() {
    let mut context = start_context();

    let reports = context.resolve_with_name::<Reports>("reports");

    // The name is the one the provider was registered under, which is derived
    // from the name of the provider.
    assert_eq!(
        reports
            .by_name
            .get("salesReport")
            .map(|report| report.title.as_str()),
        Some("sales"),
        "a HashMap is keyed by the registered name: {:?}",
        reports.by_name.keys().collect::<Vec<_>>()
    );
    assert_eq!(
        reports
            .ordered
            .get("salesReport")
            .map(|report| report.title.as_str()),
        Some("sales"),
        "a BTreeMap is keyed by the registered name: {:?}",
        reports.ordered.keys().collect::<Vec<_>>()
    );
}
