use bytes::Bytes;

pub trait Converter<S, T> {
    fn convert(&self, source: S) -> Option<T>;
}

pub trait StructuredOutputConverter<T>: Converter<Bytes, T> {
    fn get_format(&self) -> &str;
}
