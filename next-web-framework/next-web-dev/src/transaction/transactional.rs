use std::{error::Error, future::Future};
use tracing::error;

pub struct Transactional;

impl Transactional {
    pub async fn execute<F, Fut>(rbs: &rbatis::RBatis, func: F)
    where
        F: FnOnce(rbatis::RBatis) -> Fut,
        Fut: Future<Output = Result<(), Box<dyn Error + Send + Sync>>>,
    {
        if let Ok(mut tx) = rbs.acquire_begin().await {
            match func(rbs.to_owned()).await {
                Ok(_) => {
                    let _ = tx
                        .commit()
                        .await
                        .map_err(|err| error!("Transactional commit error: {:?}", err));
                }
                Err(_) => {
                    let _ = tx
                        .rollback()
                        .await
                        .map_err(|err| error!("Transactional rollback error: {:?}", err));
                }
            }
        }
    }
}


async fn test() {
    Transactional::execute(&rbatis::RBatis::new() , | rb | async move {
        

        Ok(())
    }).await;
}