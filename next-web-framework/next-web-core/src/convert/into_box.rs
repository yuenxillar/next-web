


pub trait IntoBox<T> {
    fn into_box(self) -> Box<T>;
}

impl<T> IntoBox<T> for T   {
    fn into_box(self) -> Box<T> {
        Box::new(self)
    }
}