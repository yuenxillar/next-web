use std::fmt::Display;
use std::fmt::Formatter;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExpiredSessionError {}

impl Display for ExpiredSessionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "ExpiredSessionError")
    }
}
