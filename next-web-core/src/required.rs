pub trait Required<T> {
    fn get_object(&self) -> &T;
    fn get_mut_object(&mut self) -> &mut T;
}
