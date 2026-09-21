use next_web::{Application, NextWebApplication};

#[derive(Default)]
struct TestApplication;

impl Application for TestApplication {}

#[tokio::main]
async fn main() {
    NextWebApplication::<TestApplication>::default().run().await
}
