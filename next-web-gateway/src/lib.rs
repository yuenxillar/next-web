mod core;
mod error;
mod filter;
mod server;
mod properties;
mod route;
mod service;
mod util;
mod handler;
mod context;

pub mod application;
pub mod circuit_breaker;


pub trait Ordered {
    fn order(&self) -> i32;
}