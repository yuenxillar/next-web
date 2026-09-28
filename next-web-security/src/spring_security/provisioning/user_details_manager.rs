use crate::core::userdetails::UserDetailsService;

pub trait UserDetailsManager
where
    Self: UserDetailsService,
{
}
