pub struct ErasableValue<T> {
    data: Option<T>,
    erased: bool,
}
