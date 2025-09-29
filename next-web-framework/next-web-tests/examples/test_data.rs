use next_web_dev::{Builder, FieldName, GetSet, RequiredArgsConstructor};

#[derive(Debug, PartialEq, Eq, Builder, RequiredArgsConstructor, GetSet, FieldName)]
pub struct TestData {
    #[builder(into)]
    pub name: String,

    // #[constructor(default)]
    pub age: u32,

    // not method `set_birthday`
    #[builder(into, default)]
    #[get_set(skip_set)]
    #[constructor(into)]
    pub birthday: String,

    #[builder(into, default = "default_address")]
    pub address: Option<String>,
}

fn default_address() -> Option<String> {
    Some("Canada".into())
}

fn main() {
    let data1 = TestDataBuilder::builder()
        .name("Ferris")
        .age(10)
        .birthday("2015-05-15")
        .build()
        .unwrap();
    let mut data2 = TestData::from_args("Ferris".into(), 10, "2015-05-15");
    data2.set_address(Some("Canada".into()));

    assert_eq!(data1.get_birthday(), data1.get_birthday());
    assert_eq!(TestData::field_name(), "name");
    assert_eq!(data1, data2);
}
