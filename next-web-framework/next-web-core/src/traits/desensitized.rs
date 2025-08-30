

pub trait Desensitized: Sync + Send {
    fn desensitize(&mut self);
}