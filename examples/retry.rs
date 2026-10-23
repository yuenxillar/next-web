use next_web::retry::{
    error::retry_error::RetryError, retry_callback::with_fn, retry_context::RetryContext,
    retry_operations::RetryOperations, support::retry_template::RetryTemplate,
};

/// A retryable operation that fails a few times before it succeeds.
///
/// The closure stores its own progress in the retry context, which is the supported way to
/// carry state across the attempts of one retry block.
#[tokio::main]
async fn main() {
    let template = RetryTemplate::builder()
        .max_attempts(4)
        .exponential_backoff(500, 2.0, 4_000)
        .build();

    let result = template
        .execute(with_fn(|context: &dyn RetryContext| {
            Box::pin(async move {
                let attempt = context
                    .get_attribute("attempt")
                    .and_then(|value| value.as_u64())
                    .unwrap_or_default()
                    + 1;
                context.set_attribute("attempt", attempt.into());

                println!(
                    "attempt {attempt} at retry count {}",
                    context.get_retry_count()
                );

                if attempt < 4 {
                    return Err(RetryError::custom("the dependency is not ready yet"));
                }

                Ok(attempt)
            })
        }))
        .await;

    match result {
        Ok(attempts) => println!("succeeded after {attempts} attempts"),
        Err(error) => println!("gave up: {error}"),
    }
}
