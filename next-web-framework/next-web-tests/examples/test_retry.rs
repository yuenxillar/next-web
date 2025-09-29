use std::error::Error;

use next_web_core::models::any_value::AnyValue;
use next_web_dev::retry::error::retry_error::RetryError;
use next_web_dev::retry::retry_context::RetryContext;
use next_web_dev::retry::retry_operations::RetryOperations;
use next_web_dev::retry::support::retry_template::RetryTemplate;
use next_web_dev::util::local_date_time::LocalDateTime;
use next_web_dev::Retryable;
// use next_web_retry::{
//     error::retry_error::RetryError, retry_context::RetryContext, retry_operations::RetryOperations,
//     support::retry_template::RetryTemplate,
// };

#[allow(unused)]
#[derive(Debug)]
enum TestMatch {
    A,
    B(u64),
}

#[Retryable(max_attempts = 4, delay = 1000, backoff = backoff_test, retry_for = [TestMatch::A, TestMatch::B(123)], multiplier = 2)]
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
pub enum TestRetryError {
    Default(String),
}

impl Error for TestRetryError {}
impl std::fmt::Display for TestRetryError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "TestRetryError: {}", self.to_string())
    }
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
                return Err(RetryError::Any(Box::new(TestRetryError::Default("test retry error".to_string()))));
            }

            Ok(())
        })
        .await;
    println!("result: {:?}", result);
}
