use crate::core::poi::ss::usermodel::shape::Shape;

pub trait SimpleShape: Shape {
    fn get_shape_id(&self) -> i32;
}
