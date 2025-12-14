pub trait NotRepeatExecutor {
    /// To see if it's the same executor
    fn unique_value(&self) -> String;
}
