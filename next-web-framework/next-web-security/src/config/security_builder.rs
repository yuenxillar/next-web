

pub trait SecurityBuilder<O> {

    fn build(&self) -> O;
}