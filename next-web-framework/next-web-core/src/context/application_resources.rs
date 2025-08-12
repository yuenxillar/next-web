/// Resource files that need to be embedded in binary files
pub trait ApplicationResources {
    fn get(file_path: impl AsRef<str>) -> Option<std::borrow::Cow<'static, [u8]>>;

    fn iter() -> impl Iterator<Item = std::borrow::Cow<'static, str>>;
}
