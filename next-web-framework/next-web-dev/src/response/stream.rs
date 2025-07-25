use axum::{
    body::{Body, Bytes},
    http::{header, StatusCode},
    response::IntoResponse,
    BoxError,
};
use futures::{executor::block_on, TryStream};

pub struct  ResponseStream {
    chunk_size: usize,
    stream_type: StreamType,
}

pub enum StreamType {
    LocalFile(tokio::fs::File),
    // NetworkFile(String),
    Bytes(axum::body::Bytes),
    Ready
}

impl ResponseStream {
    
    pub fn new() -> Self {
        Self::default()
    }

    pub fn chunk_size(mut self, chunk_size: usize) -> Self {
        self.chunk_size = chunk_size;
        self
    }
}


impl Default for ResponseStream {
    fn default() -> Self {
        Self {
            chunk_size: 4096,
            stream_type: StreamType::Ready
        }
    }
}
impl IntoResponse for ResponseStream {
    fn into_response(self) -> axum::response::Response {
        // axum::body::Body::from_stream(stream)
        let mut filename: Option<String> = None;
        // let stream = match self {
        //     ResponseStream::LocalFile(path) => {
        //         let path = std::path::Path::new(&path);
        //         path.file_name()
        //             .map(|s| s.to_str().map(|s| filename = Some(s.to_string())));
        //         let file = tokio::fs::File::open(path).await.unwrap();
        //         tokio_util::io::ReaderStream::new(file)
        //     }
        //     ResponseStream::Bytes(bytes) => tokio_util::io::ReaderStream::new(bytes),
        // };

        let path = std::path::Path::new("11");
        path.file_name()
            .map(|s| s.to_str().map(|s| filename = Some(s.to_string())));

        let file = if let StreamType::LocalFile(file) = self.stream_type { file } else { panic!("not local file") };
        let stream = tokio_util::io::ReaderStream::with_capacity(file, self.chunk_size);

        let header_name = format!(
            "attachment;filename={}",
            filename.map(|s| s).unwrap_or("unknown".into())
        );


        axum::response::Response::builder()
            .status(StatusCode::OK)
            .header(header::ACCEPT_RANGES, "bytes")
            .header(header::CONTENT_TYPE, "application/octet-stream")
            .header(header::CONTENT_DISPOSITION, header_name)
            .body(Body::from_stream(Bytes::from_static(b"bytes")))
            .unwrap()
    }
}
