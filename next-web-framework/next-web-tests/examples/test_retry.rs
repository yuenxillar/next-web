use next_web_core::util::any_map::AnyValue;
use next_web_dev::util::local_date_time::LocalDateTime;
use next_web_macros::Retryable;
use next_web_retry::{
    error::retry_error::RetryError, retry_context::RetryContext, retry_operations::RetryOperations,
    support::retry_template::RetryTemplate,
};

#[allow(unused)]
#[derive(Debug)]
enum TestMatch {
    A,
    B(u64),
}

#[Retryable(max_attempts = 3, delay = 1000, backoff = backoff_test, retry_for = [TestMatch::A, TestMatch::B(123)], multiplier = 2)]
fn test_retry() -> Result<(), TestMatch> {
    let timestamp_sec = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    println!("{}", LocalDateTime::now());
    match timestamp_sec % 2 {
        0 => Err(TestMatch::B(123)),
        _ => Err(TestMatch::A),
    }
}

#[allow(unused)]
fn backoff_test(error: &TestMatch) {
    println!("function test_retry backoff: {:?}", error);
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum SelfRetryError {
    Custom(String),
}

#[tokio::main]
async fn main() {
    let result = RetryTemplate::builder()
        .max_attempts(4)
        .exponential_backoff(1000, 6000, 2.0, false)
        .build()
        .execute(|ctx: std::sync::Arc<dyn RetryContext>| async move {
            let retry_count = ctx.get_attribute("retryCount");
            let value = retry_count
                .map(|s| s.as_number().unwrap_or_default())
                .unwrap_or_default();

            if value < 4 {
                if value > 0 {
                    println!("Retry Count: {}, timestamp: {}", value, LocalDateTime::timestamp());
                }
                ctx.set_attribute("retryCount", AnyValue::Number(value + 1));
                return Err(RetryError::Custom("Test Retry Error: 123.".to_string()));
            }

            Ok(())
        })
        .await;
    println!("result: {:?}", result);
}
