mod header_writer;
mod header_writer_filter;

pub mod writers;

pub use header_writer::HeaderWriter;
pub use header_writer_filter::HeaderWriterFilter;

#[derive(Clone)]
pub struct Header {
    header_name: String,
    header_value: Vec<String>,
}

impl Header {
    pub fn new(header_name: String, header_value: Vec<String>) -> Self {
        Self {
            header_name,
            header_value,
        }
    }

    pub fn header_name(&self) -> &str {
        &self.header_name
    }

    pub fn header_value(&self) -> &[String] {
        &self.header_value
    }
}
