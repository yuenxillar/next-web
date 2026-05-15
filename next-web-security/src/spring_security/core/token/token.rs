pub trait Token: Send + Sync {
    fn key(&self) -> &str;

    fn key_creation_time(&self) -> i64;

    fn extended_information(&self) -> &str;
}
