use std::{io::Cursor, time::{self, Instant}};

use axum::{
    body::{Body, Bytes},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use futures::stream::{self, Stream, StreamExt};
use next_web_document_excel::core::write::excel_writer::ExcelWriter;
use rust_xlsxwriter::*;
use tokio_util::io::ReaderStream;

#[derive(serde::Serialize, serde::Deserialize, Debug, XlsxSerialize)]
#[xlsx(table = Table::new())]
pub(crate) struct User {
    pub id: i64,
    pub number: i64,
    pub datetime: i64,
    pub test: i64,
    pub model: String,
}

pub(crate) async fn excel_write() -> Result<Response, ()> {
    let mut writer = ExcelWriter::new();

    let instant = Instant::now();
    let mut data = Vec::new();
    for i in 0..100000 {
        let buff = User {
            id: i + 1,
            number: i + 100,
            datetime: i + 200,
            test: i + 300,
            model: r"High level features  高级功能
Route requests to handlers with a macro free API.
使用无宏 API 将请求路由到处理程序。
Declaratively parse requests using extractors.
使用提取程序以声明方式解析请求。
Simple and predictable error handling model.
简单且可预测的错误处理模型。
Generate responses with minimal boilerplate.
使用最少的样板生成响应。
Take full advantage of the tower and tower-http ecosystem of middleware, services, and utilities.
充分利用中间件、服务和实用程序的 tower 和 tower-http 生态系统。
In particular the last point is what sets axum apart from other frameworks. axum doesn't have its own middleware system but instead uses tower::Service. This means axum gets timeouts, tracing, compression, authorization, and more, for free. It also enables you to share middleware with applications written using hyper or tonic.
特别是最后一点是 axum 与其他框架的不同之处。 axum 没有自己的中间件系统，而是使用 tower：：Service 的这意味着 axum 可以免费获得超时、跟踪、压缩、授权等。它还使您能够与使用 hyper 或 tonic 编写的应用程序共享中间件。".into()
        };
        data.push(buff);
    }
    writer.write("test_excel.xlsx", &data);

    println!("{:?}", instant.elapsed());
    let  buffer = writer.save_to_buffer().unwrap();

    println!("{:?}", instant.elapsed());
    let curosr = Cursor::new(buffer);
    let stream = ReaderStream::with_capacity(curosr,  5242880);
    let filename = format!("attachment;filename={}", "test_excel.xlsx");

    println!("{:?}", instant.elapsed());
    return Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .header(header::CONTENT_DISPOSITION, filename)
        .body(Body::from_stream(stream))
        .unwrap());
}

pub(crate) async fn excel_read() {}

pub(crate) fn excel_api() -> Router {
    Router::new().nest(
        "/excel",
        Router::new()
            .route("/read", post(excel_read))
            .route("/write", get(excel_write)),
    )
}
