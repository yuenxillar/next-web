#[cfg(feature = "official-account-customer-service")]
pub mod customer_service;
#[cfg(feature = "official-account-menu")]
pub mod menu;
#[cfg(feature = "official-account-message")]
pub mod message;
#[cfg(feature = "official-account-subscribe")]
pub mod subscribe;
#[cfg(feature = "official-account-user-management")]
pub mod user_management;

pub mod client;
pub mod credential;
pub mod error;
mod response;
pub mod user;

pub type Result<T> = std::result::Result<T, error::Error>;
