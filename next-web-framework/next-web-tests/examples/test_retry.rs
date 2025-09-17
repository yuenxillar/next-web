use next_web_dev::util::local_date_time::LocalDateTime;
use next_web_macros::Retry;
use next_web_retry::{retry_operations::RetryOperations, support::retry_template::RetryTemplate};

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
    if let Err(e) = test_retry() {
        println!("error: {:?}", e);
    }

    let s = String::from("hello");

    let func = |ctx| {
        async {
            println!("s:{}", s.clone());
            Ok(132)
        }
    };
    let result = RetryTemplate::builder().build()
    .execute(func).await;
    println!("result: {:?}", result);
}
