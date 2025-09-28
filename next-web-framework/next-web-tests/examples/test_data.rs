use next_web_dev::{Builder, FieldName, GetSet, RequiredArgsConstructor};

#[derive(Debug, PartialEq, Eq, Builder, RequiredArgsConstructor, GetSet, FieldName)]
pub struct TestData {
    #[builder(into)]
    pub name: String,

    #[constructor(default)]
    pub age: u32,

    // not method `set_birthday`
    #[builder(into)]
    #[get_set(skip_set)]
    #[constructor(required)]
    pub birthday: Option<String>,

    #[builder(into)]
    pub address: Option<String>,
}

fn main() {
    let data1 = TestDataBuilder::builder()
        .name("Ferris")
        .age(10)
        .birthday("2015-05-15")
        .address("Canada")
        .build()
        .unwrap();
    let mut data2 = TestData::from_args("Ferris".into(), 10, Some("2015-05-15".into()));
    data2.set_address(Some("Canada".into()));

    assert_eq!(data1.get_birthday(), data1.get_birthday());
    assert_eq!(TestData::field_name(), "name");
    assert_eq!(data1, data2);
}
