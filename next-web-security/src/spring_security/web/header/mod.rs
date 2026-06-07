mod header_writer;
mod header_writer_filter;

pub mod writers;

pub use header_writer::HeaderWriter;
pub use header_writer_filter::HeaderWriterFilter;

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
}
