use crate::env::ParseError;

pub enum EnvError {
    Parse(ParseError),
}

impl From<ParseError> for EnvError {
    fn from(value: ParseError) -> Self {
        Self::Parse(value)
    }
}
