

pub trait SecurityBuilder<O>
where 
Self: Send + Sync, 
O: Send + Sync,
 {

    fn build(&self) -> O;
}