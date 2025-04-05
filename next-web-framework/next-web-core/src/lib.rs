use rudi::Properties;

mod context;

#[Properties(prefix = "test")]
#[derive(Debug, Clone, Default, serde::Deserialize)]
struct TestProperties {
    #[pro_name = "test"]
    test: String,
}