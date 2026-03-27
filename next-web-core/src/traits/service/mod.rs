pub mod background_service;

use crate::traits::singleton::Singleton;

pub trait Service: Singleton {}
