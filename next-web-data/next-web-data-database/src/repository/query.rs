use rbatis::async_trait;
use serde::de::DeserializeOwned;


type Result<V> = std::result::Result<V, rbatis::Error>;

#[async_trait]
pub trait Query {
    async fn select_list<T, V>(&self, wrapper: QueryWrapper<'static, T>) -> Result<Vec<T>>
    where
        T: DeserializeOwned + TableName + Send,
        V: DeserializeOwned,
    {
        self.execute(wrapper).await
    }

    async fn select_one<T, V>(&self, wrapper: QueryWrapper<'static, T>) -> Result<T>
    where
        T: DeserializeOwned + TableName + Send,
        
        V: DeserializeOwned,
    {
        self.execute(wrapper).await
    }

    async fn select_count<T, V>(&self, wrapper: QueryWrapper<'static, T>) -> Result<usize>
    where
        T: DeserializeOwned + TableName + Send,
        V: DeserializeOwned,
    {
        self.execute(wrapper.select(vec!["COUNT(1) as count"]))
            .await
    }

    async fn execute<T, V>(&self, wrapper: QueryWrapper<'static, T>) -> Result<V>
    where
        T: DeserializeOwned + TableName + Send,
        V: DeserializeOwned;
}


#[async_trait]
impl Query for rbatis::RBatis {

    async fn execute<T, V>(&self, wrapper: QueryWrapper<'static, T>) -> Result<V>
    where
        T: DeserializeOwned + TableName + Send,
        V: DeserializeOwned 
    {

    }
}