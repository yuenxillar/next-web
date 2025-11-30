pub mod analysis;
pub mod cache;
pub mod constant;
pub mod context;
pub mod converters;
pub mod enums;
pub mod error;
pub mod event;
pub mod excel_reader;
pub mod excel_writer;
pub mod fast_excel;
pub mod fast_excel_factory;
pub mod metadata;
pub mod read;
pub mod support;
pub mod util;
pub mod write;

pub trait Closeable {
    fn close(&mut self) -> Result<(), next_web_core::error::BoxError>;
}
