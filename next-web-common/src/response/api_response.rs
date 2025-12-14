use axum_core::response::{IntoResponse, Response};
use serde::Serialize;

/// A generic structure representing a standardized API response.
///
/// This struct is designed to encapsulate the result of an API call in a consistent format.
/// It includes fields for the HTTP status code, a human-readable message, and optional data.
/// The use of generics allows this structure to be flexible and work with any type that implements
/// the `Serialize` trait, making it suitable for JSON serialization (e.g., using `serde`).
///
/// # Type Parameters
/// - `T`: The type of the data field. It must implement the `Serialize` trait to ensure
///   compatibility with serialization libraries like `serde`.
///
/// # Fields
/// - `status: u16`  
///   Represents the HTTP status code of the response (e.g., 200 for success, 404 for not found).
///   This provides a machine-readable indicator of the response outcome.
///
/// - `message: String`  
///   A human-readable description of the response. This can be used to provide additional context
///   or details about the result (e.g., "Resource created successfully" or "Invalid input").
///
/// - `data: Option<T>`  
///   Contains the payload of the response, if any. The use of `Option` allows this field to be
///   omitted when there is no data to return (e.g., in case of an error). The type `T` is generic,
///   enabling flexibility for different kinds of data (e.g., user information, list of items, etc.).
///
/// # Example
/// ```rust
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct User {
///     id: u32,
///     name: String,
/// }
///
/// let response = ApiResponse {
///     status: 0,
///     message: "User retrieved successfully".to_string(),
///     data: Some(User {
///         id: 1,
///         name: "Alice".to_string(),
///     }),
/// };
///
/// // Serialize the response to JSON
/// let json_response = serde_json::to_string(&response).unwrap();
/// println!("{}", json_response);
/// ```
#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    status: u16,
    message: String,
    data: Option<T>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn new(status: u16, message: String, data: T) -> Self {
        ApiResponse {
            status,
            message,
            data: Some(data),
        }
    }

    pub fn get_status(&self) -> u16 {
        self.status
    }

    pub fn get_message(&self) -> &str {
        &self.message
    }

    pub fn get_data(&self) -> Option<&T> {
        self.data.as_ref()
    }

    pub fn set_status(mut self, status_code: u16) -> ApiResponse<T> {
        self.status = status_code;
        return self;
    }

    pub fn set_message(mut self, message: impl Into<String>) -> ApiResponse<T> {
        self.message = message.into();
        return self;
    }

    pub fn set_data(mut self, data: T) -> ApiResponse<T> {
        self.data = Some(data);
        return self;
    }
}

impl<T: Serialize> ApiResponse<T> {
    pub fn ok(data: T) -> ApiResponse<T> {
        return ApiResponse {
            status: 0,
            message: "ok".into(),
            data: Some(data),
        };
    }

    pub fn fail(data: T) -> ApiResponse<T> {
        return ApiResponse {
            status: 500,
            message: "fail".into(),
            data: Some(data),
        };
    }

    pub fn empty() -> ApiResponse<T> {
        return ApiResponse {
            status: 204,
            message: "".into(),
            data: None,
        };
    }
}

impl<T> IntoResponse for ApiResponse<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        Response::builder()
            .header("Content-Type", "application/json")
            .status(200)
            .body(serde_json::json!(self).to_string().into())
            .unwrap()
    }
}
