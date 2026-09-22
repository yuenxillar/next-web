mod bytes_stream;
mod local_file_stream;
mod network_file_stream;
mod response_stream;

pub use bytes_stream::BytesStream;
pub use local_file_stream::LocalFileStream;
pub use network_file_stream::NetworkFileStream;
pub use response_stream::ResponseStream;

/// 4kb
pub const DEFAULT_CHUNK_SIZE: usize = 4096;
