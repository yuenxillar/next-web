use axum::{
    body::Bytes,
    extract::{FromRequest, Request},
    http::{header, HeaderMap, StatusCode},
};
use serde::de::DeserializeOwned;

pub struct Data<T>(pub T);

impl<T, S> FromRequest<S> for Data<T>
where
    T: DeserializeOwned + Send,
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        if json_content_type(req.headers()) {
            let bytes = Bytes::from_request(req, state).await;
            if bytes.is_err() {
                return Err((StatusCode::BAD_REQUEST, "Bad Request"));
            }
            Self::from_bytes(&bytes.unwrap())
        } else {
            Err((StatusCode::BAD_REQUEST, "Bad Request"))
        }
    }
}

fn json_content_type(headers: &HeaderMap) -> bool {
    let content_type = if let Some(content_type) = headers.get(header::CONTENT_TYPE) {
        content_type
    } else {
        return false;
    };

    let content_type = if let Ok(content_type) = content_type.to_str() {
        content_type
    } else {
        return false;
    };

    let mime = if let Ok(mime) = content_type.parse::<mime::Mime>() {
        mime
    } else {
        return false;
    };

    let is_json_content_type = mime.type_() == "application"
        && (mime.subtype() == "json" || mime.suffix().is_some_and(|name| name == "json"));

    is_json_content_type
}

impl<T> Data<T>
where
    T: DeserializeOwned,
{
    /// Construct a `Json<T>` from a byte slice. Most users should prefer to use the `FromRequest` impl
    /// but special cases may require first extracting a `Request` into `Bytes` then optionally
    /// constructing a `Json<T>`.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, (StatusCode, &'static str)> {
        // Decode

        let deserializer = &mut serde_json::Deserializer::from_slice(bytes);

        let value = match serde_path_to_error::deserialize(deserializer) {
            Ok(value) => value,
            Err(err) => {
                let rejection = match err.inner().classify() {
                    serde_json::error::Category::Data => (StatusCode::BAD_REQUEST, "Bad Data"),
                    serde_json::error::Category::Syntax | serde_json::error::Category::Eof => {
                        (StatusCode::BAD_REQUEST, "Bad Syntax")
                    }
                    serde_json::error::Category::Io => {
                        if cfg!(debug_assertions) {
                            // we don't use `serde_json::from_reader` and instead always buffer
                            // bodies first, so we shouldn't encounter any IO errors
                            unreachable!()
                        } else {
                            (StatusCode::BAD_REQUEST, "Bad Io")
                        }
                    }
                };
                return Err(rejection);
            }
        };
        Ok(Data(value))
    }
}
