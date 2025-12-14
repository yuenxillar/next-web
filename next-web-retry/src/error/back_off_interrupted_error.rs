

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackOffInterruptedError {
    Consume(String)
}