use axum::response::IntoResponse;
use next_web_core::interface::stream::into_response_stream::IntoRespnoseStream;

pub struct ResponseStream<T> {
    target_rate: usize,
    stream: T,
}

impl<T> ResponseStream<T>
where
    T: IntoRespnoseStream,
{
    pub fn new(stream: T) -> Self {
        Self {
            target_rate: 2048,
            stream,
        }
    }

    pub fn target_rate(mut self, target_rate: usize) -> Self {
        self.target_rate = target_rate;
        self
    }
}

impl<T> IntoResponse for ResponseStream<T>
where
    T: IntoRespnoseStream,
{
    fn into_response(self) -> axum::response::Response {
        // 凡事留一线 日后好相见！
        let target_rate = if self.target_rate < 1024 {
            1024
        } else {
            self.target_rate
        };
        self.stream.into_response_stream(target_rate)
    }
}

// fn dynamic_rate_stream() -> impl Stream<Item = Result<Bytes, BoxError>> {
//     let path = std::path::Path::new(r"D:\ObsVideo\test.mkv");

//     let file = std::fs::File::open(path).unwrap();

//     let mut buff = BufReader::new(file);
//     let mut buf = Vec::new();
//     buff.read_to_end(&mut buf).unwrap();

//     if buf.is_empty() {
//         panic!("文件为空")
//     }
//     let data = Bytes::copy_from_slice(&buf);

//     let target_rate = 1024 * 1024 * 10;
//     let chunk_size = 1024; // 初始块大小

//     // 分割数据
//     let chunks: Vec<Bytes> = data
//         .chunks(chunk_size)
//         .map(|slice| Bytes::copy_from_slice(slice))
//         .collect();

//     let start_time = Instant::now();
//     let mut bytes_sent = 0;

//     let stream = stream::iter(chunks).then(move |chunk| {
//         let chunk_len = chunk.len();
//         let now = Instant::now();
//         let elapsed = now.duration_since(start_time);
//         let expected_time = Duration::from_secs_f64(bytes_sent as f64 / target_rate as f64);

//         // 动态调整延迟
//         let delay = if expected_time > elapsed {
//             expected_time - elapsed
//         } else {
//             Duration::from_secs(0)
//         };

//         bytes_sent += chunk_len;

//         async move {
//             if !delay.is_zero() {
//                 tokio::time::sleep(delay).await;
//             }
//             Ok::<_, BoxError>(chunk)
//         }
//     });

//     http_body_util::StreamBody::new(stream)
// }
