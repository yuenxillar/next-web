use std::fmt::Display;

pub struct InvalidParameterError<T>(pub T);

impl<T: Display> Display for InvalidParameterError<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Invalid parameter: {}", self.0)
    }
}