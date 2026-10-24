#[cfg(feature = "T1")]
mod shiro;
#[cfg(feature = "T1")]
pub use shiro::*;

#[cfg(feature = "T2")]
mod spring_security;
#[cfg(feature = "T2")]
pub use spring_security::*;
