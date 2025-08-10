pub trait ResponseMetadata: Send {
    fn get<T>(&self, key: impl AsRef<str>) -> T;

    fn get_or_default<T>(&self, key: impl AsRef<str>, default: T) -> T;

    fn is_empty(&self) -> bool;
}
