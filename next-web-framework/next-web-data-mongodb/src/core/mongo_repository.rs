use std::str::FromStr;

use crate::MongoResult;
use crate::service::mongodb_service::MongodbService;
use futures::TryStreamExt;
use mongodb::Collection;
use mongodb::bson::oid::ObjectId;
use mongodb::bson::{Document, doc};
use mongodb::options::FindOptions;
use mongodb::results::*;
use next_web_core::async_trait;
use serde::{Serialize, de::DeserializeOwned};

use super::collection_name::CollectionName;

#[async_trait]
pub trait MongoRepository
where
    Self: CollectionName,
    Self: Serialize + DeserializeOwned,
{
    fn collection<T: Send + Sync + Serialize>(service: &MongodbService) -> Collection<T> {
        service.get_database().collection(Self::col_name())
    }

    async fn find_one(service: &MongodbService, filter: Document) -> MongoResult<Option<Self>> {
        Self::collection::<Self>(service).find_one(filter).await
    }

    async fn find_many(
        service: &MongodbService,
        filter: Option<Document>,
        options: Option<FindOptions>,
    ) -> MongoResult<Vec<Self>> {
        let cursor = Self::collection::<Self>(service)
            .find(filter.unwrap_or_default())
            .with_options(options)
            .await?;
        cursor.try_collect().await
    }

    async fn find_all(service: &MongodbService) -> MongoResult<Vec<Self>> {
        let cursor = Self::collection::<Self>(service).find(doc! {}).await?;
        cursor.try_collect().await
    }

    async fn find_by_id(service: &MongodbService, id: &str) -> MongoResult<Option<Self>> {
        Self::collection::<Self>(service)
            .find_one(doc! {"_id": id})
            .await
    }

    async fn insert_one(service: &MongodbService, doc: &Self) -> MongoResult<InsertOneResult>
    where
        Self: Sized,
    {
        Self::collection::<Self>(service).insert_one(doc).await
    }

    async fn insert_many(
        service: &MongodbService,
        docs: Vec<Self>,
    ) -> MongoResult<InsertManyResult> {
        Self::collection::<Self>(service).insert_many(docs).await
    }

    async fn update_one();

    async fn update_by_id(
        service: &MongodbService,
        id: &str,
    ) -> MongoResult<UpdateResult> {
        Self::collection::<Self>(service)
            .update_one(
                doc! {"_id": id},
                doc! {
                "$set": doc! { "name": "Jill Gillison" } },
            )
            .await
    }
    async fn update_many();

    async fn delete_one();

    async fn delete_by_id(service: &MongodbService, id: &str) -> MongoResult<DeleteResult> {
        Self::collection::<Self>(service)
            .delete_one(doc! {"_id": id})
            .await
    }

    async fn delete_many(service: &MongodbService, filter: Document) -> MongoResult<DeleteResult> {
        Self::collection::<Self>(service).delete_many(filter).await
    }

    async fn count_documents(
        service: &MongodbService,
        filter: Option<Document>,
    ) -> MongoResult<u64> {
        Self::collection::<Self>(service)
            .count_documents(filter.unwrap_or_default())
            .await
    }
}
