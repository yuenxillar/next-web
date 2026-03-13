pub trait Id {
    fn id(&self) -> &'static str;
}
