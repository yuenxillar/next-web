pub trait IntoBox<T> {
    fn into_boxed(self) -> Box<T>;
}


impl<T> IntoBox<T> for T   {
    fn into_boxed(self) -> Box<T> {
        Box::new(self)
    }
}