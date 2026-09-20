use crate::factory::SingletonFactory;

pub trait ListableSingletonFactory
where
    Self: SingletonFactory,
{
    fn get_singletons_of_type<T>(&self) -> Vec<&T>
    where
        T: 'static;

    fn get_singletons_mut_of_type<T>(&mut self) -> Vec<&mut T>
    where
        T: 'static;

    fn get_singleton_names_for_type<T>(&self) -> Vec<&str>
    where
        T: 'static;
}
