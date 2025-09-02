use std::sync::Arc;

use axum::{
    body::Bytes,
    extract::{FromRequest, Request},
    http::{header, HeaderMap, StatusCode},
};
use next_web_core::{
    state::application_state::ApplicationState, traits::data_decoder::DataDecoder,
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
        if is_json(req.headers()) {
            let decoder = match req.extensions().get::<ApplicationState>() {
                Some(state) => state
                    .context()
                    .read()
                    .await
                    .get_single_option_with_name::<Arc<dyn DataDecoder>>("defaultDataDecoder")
                    .cloned(),
                _ => None,
            };

            let bytes_result = Bytes::from_request(req, state).await;
            match bytes_result {
                Ok(bytes) => return Self::from_bytes(&bytes, decoder),
                Err(e) => tracing::error!("Failed to read request body, case: {}", e),
            }
        }

        Err((StatusCode::BAD_REQUEST, "Bad Request"))
    }
}

pub(crate) fn is_json(headers: &HeaderMap) -> bool {
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
    pub fn from_bytes(
        bytes: &[u8],
        decoder: Option<Arc<dyn DataDecoder>>,
    ) -> Result<Self, (StatusCode, &'static str)> {
        if bytes.is_empty() {
            return Err((StatusCode::BAD_REQUEST, "Body is empty"));
        }

        // Decode
        let mut raw_data = String::default();

        if let Some(decoder) = decoder {
            raw_data = decoder
                .decode(bytes)
                .or_else(|err| Err((StatusCode::BAD_REQUEST, err)))?
        }

        let deserializer = &mut serde_json::Deserializer::from_slice(if raw_data.is_empty() {
            bytes
        } else {
            raw_data.as_bytes()
        });

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
