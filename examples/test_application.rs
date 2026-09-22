use next_web::{Application, NextWebApplication, macros::bind::get_mapping};

#[derive(Default)]
struct TestApplication;

impl Application for TestApplication {}

#[get_mapping(path = "/hello")]
async fn hello() -> &'static str {
    "Hello, World!"
}

#[tokio::main]
async fn main() {
    NextWebApplication::<TestApplication>::default().run().await
}
