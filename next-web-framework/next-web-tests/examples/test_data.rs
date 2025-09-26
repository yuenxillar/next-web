use next_web_dev::RequiredArgsConstructor;


#[derive(RequiredArgsConstructor)]
pub struct TestData {
    pub name: String,
    #[constructor(required, default)]
    pub age: u32,
    pub birthday: Option<String>,
}

fn main() {
    let data = TestData::from_args("zhangsan".into(), 18);
}