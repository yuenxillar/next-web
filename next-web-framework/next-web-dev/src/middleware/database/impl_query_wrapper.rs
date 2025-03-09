use async_trait::async_trait;
use next_web_common::query::query_wrapper::{QueryWrapper, TableName, Wrapper};
use rbatis::RBatis;
use serde::de::DeserializeOwned;
use std::result::Result as StdResult;

type Result<V> = StdResult<V, rbatis::Error>;

/// 可为执行器实现 trait
#[async_trait]
pub trait SelectWrapper {
    async fn select_list<T, V>(&self, wrapper: QueryWrapper<'static, T>) -> Result<Vec<T>>
    where
        T: DeserializeOwned + TableName + Send + Sync + 'static,
        V: DeserializeOwned,
    {
        self.execute(wrapper).await
    }

    async fn select_one<T, V>(&self, wrapper: QueryWrapper<'static, T>) -> Result<T>
    where
        T: DeserializeOwned + TableName + Send + Sync + 'static,
        V: DeserializeOwned,
    {
        self.execute(wrapper).await
    }

    async fn select_count<T, V>(&self, wrapper: QueryWrapper<'static, T>) -> Result<usize>
    where
        T: DeserializeOwned + TableName + Send + Sync + 'static,
        V: DeserializeOwned,
    {
        self.execute(wrapper.select(vec!["COUNT(1) as count"]))
            .await
    }

    async fn execute<T, V>(&self, wrapper: QueryWrapper<'static, T>) -> Result<V>
    where
        T: DeserializeOwned + TableName + Send + Sync + 'static,
        V: DeserializeOwned;
}

#[async_trait]
impl SelectWrapper for RBatis {
    async fn execute<T, V>(&self, wrapper: QueryWrapper<'static, T>) -> Result<V>
    where
        T: DeserializeOwned + TableName + Send + Sync + 'static,
        V: DeserializeOwned,
    {
        self.query_decode(&wrapper.generate_sql(), vec![]).await
    }
}
