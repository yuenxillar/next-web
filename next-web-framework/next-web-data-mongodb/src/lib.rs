pub mod auto_register;
pub mod core;
pub mod properties;
pub mod service;


pub use mongodb::{bson::*,error::Result as MongoResult, results::{
    InsertManyResult, InsertOneResult, UpdateResult, DeleteResult
}};