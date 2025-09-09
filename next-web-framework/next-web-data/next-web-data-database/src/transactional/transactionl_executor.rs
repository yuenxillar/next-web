use rbatis::async_trait;
use std::{error::Error, future::Future};
use tracing::error;

#[async_trait]
pub trait TransactionalExecutor {
    async fn execute_transaction<F, Fut>(
        &self,
        block: F,
    ) -> Result<(), Box<dyn Error + Send + Sync>>
    where
        F: Send,
        F: FnOnce(&rbatis::RBatis) -> Fut,
        Fut: Future<Output = Result<(), Box<dyn Error + Send + Sync>>> + Send;
}

#[async_trait]
impl TransactionalExecutor for rbatis::RBatis {
    async fn execute_transaction<F, Fut>(
        &self,
        block: F,
    ) -> Result<(), Box<dyn Error + Send + Sync>>
    where
        F: Send,
        F: FnOnce(&rbatis::RBatis) -> Fut,
        Fut: Future<Output = Result<(), Box<dyn Error + Send + Sync>>> + Send,
    {
        match self.acquire_begin().await {
            Ok(tx) => match block(self).await {
                Ok(_) => tx.commit().await.map_err(|err| {
                    error!("Transactional commit error: {:?}", err);
                    err.into()
                }),
                Err(msg) => {
                    error!("Transactional block error: {}", msg.to_string());

                    tx.rollback().await.map_err(|err| {
                        error!("Transactional rollback error: {:?}", err);
                        err.into()
                    })
                }
            },
            Err(err) => {
                error!("Transactional begin error: {}", err.to_string());
                return Err(Box::new(err));
            }
        }
    }
}

async fn main() {
    let rbs = rbatis::RBatis::new();

    rbs.execute_transaction(|rb| async { Ok(()) })
        .await
        .unwrap();
}
