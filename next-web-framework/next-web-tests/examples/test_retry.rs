use next_web_core::util::any_map::AnyValue;
use next_web_dev::util::local_date_time::LocalDateTime;
use next_web_macros::Retry;
use next_web_retry::{error::retry_error::RetryError, retry_context::RetryContext, retry_operations::RetryOperations, support::retry_template::RetryTemplate};

#[derive(Debug)]
enum TestMatch {
    App,
    Job(u64),
}

#[Retry(max_attempts = 3, delay = 1000, backoff = backoff_test, retry_for = [TestMatch::App, TestMatch::Job(123)], multiplier = 2)]
fn test_retry() -> Result<(), TestMatch> {
    let timestamp_sec = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    println!("{}", LocalDateTime::now());
    match timestamp_sec % 2 {
        0 => Err(TestMatch::Job(123)),
        _ => Err(TestMatch::App),
    }
}

fn backoff_test(error: &TestMatch) {
    println!("function test_retry backoff: {:?}", error);
}

#[tokio::main]
async fn main() {
    // test retry
    // if let Err(e) = test_retry() {
    //     println!("error: {:?}", e);
    // }

    let result = RetryTemplate::builder()
        .build()
        .execute(|mut ctx: Box<dyn RetryContext>| async move {

            let retry_count = ctx.as_ref().get_attribute("retryCount");
            let value = retry_count.map(|s| s.as_number().unwrap()).unwrap();
            println!("value: {}", value);
            
            if  value < 3 {
                ctx.as_mut().set_attribute("retryCount", AnyValue::Number(value + 1));
                return Err(RetryError::Custom(String::from("test")));
            }
            Ok(132)
        })
        .await;
    println!("result: {:?}", result);
}
