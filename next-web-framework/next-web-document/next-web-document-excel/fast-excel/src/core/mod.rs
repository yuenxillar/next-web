pub mod converters;
pub mod enums;
pub mod excel_reader;
pub mod excel_writer;
pub mod fast_excel;
pub mod fast_excel_factory;
pub mod metadata;
pub mod support;
pub mod write;

pub trait Closeable {
    fn close(&mut self) -> Result<(), next_web_core::error::BoxError>;
}
