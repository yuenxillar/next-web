use rudi::{Context, Singleton};

#[Singleton(name = "foo")]
#[derive(Clone, Debug)]
pub struct TestAA {
    #[di(name = "number")]
    number: i32
}
#[Singleton]
#[derive(Clone, Debug)]
pub struct TestBB {
    pub test_aa: TestAA,
}
#[Singleton(name = "number")]
fn Number() -> i32 {
    42
}

#[Singleton]
fn Test() -> String {
    String::from("test111")
}

fn main() {
    let mut cx = Context::auto_register();

    cx.just_create_single_with_name::<i32>("number");

    let number = cx.resolve_with_name::<i32>("number");
    println!("number: {}", number);

}
