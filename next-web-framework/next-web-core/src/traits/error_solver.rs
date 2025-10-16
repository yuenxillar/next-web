use std::fmt::{Debug, Display};

use axum::response::IntoResponse;

pub trait ErrorSolver<T = String>
where
    T: serde::Serialize,
    T: Debug + Display,
    T: Send + Sync,
    T: Clone,
    T: IntoResponse,
{
    fn solve_error(error: String) -> T;
}

impl ErrorSolver for () {
    fn solve_error(error: String) -> String {
        error
    }
}
