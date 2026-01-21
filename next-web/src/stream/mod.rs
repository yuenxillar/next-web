pub mod bytes_stream;
pub mod local_file_stream;
pub mod network_file_stream;
pub mod response_stream;

/// 4kb
pub const DEFAULT_CHUNK_SIZE: usize = 4096;
