pub trait Nameable {
    fn name(&self) -> &str;
    fn set_name(&mut self, name: &str);
}
