use mongodb::bson::Document;

pub mod collection_name;
pub mod mongo_repository;

pub trait ToDocument: Send + Sync {
    fn to_document(&self) -> Document;
}