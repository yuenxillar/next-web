

/// 用于规范结构体必须要有的字段
pub trait Required<T> 
{
    fn get_object(&self) -> & T;

    fn get_object_mut(&mut self) -> &mut T;
}