#[macro_export]
macro_rules! id {
    ($listener:ty, $event:ty) => {
        concat!(stringify!($listener), "<", stringify!($event), ">")
    };
}
