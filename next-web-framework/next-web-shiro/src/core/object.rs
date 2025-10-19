use next_web_core::traits::any_clone::AnyClone;

#[derive(Clone)]
pub enum Object {
    Str(String),
    Int(i64),
    Obj(Box<dyn AnyClone>),
}