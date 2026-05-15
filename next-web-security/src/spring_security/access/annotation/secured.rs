/// Rust equivalent of Spring Security's `@Secured` annotation.
///
/// In Java, `@Secured({"ROLE_USER", "ROLE_ADMIN"})` is placed on methods/types.
/// In Rust, since annotations don't exist natively, this struct holds the
/// security configuration attributes. It can be used programmatically or in
/// combination with a proc-macro attribute (e.g. `#[secured("ROLE_ADMIN")]`)
/// for framework-level method security.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Secured {
    pub roles: Vec<String>,
}

impl Secured {
    pub fn new(roles: Vec<String>) -> Self {
        Self { roles }
    }
}

impl From<Vec<String>> for Secured {
    fn from(roles: Vec<String>) -> Self {
        Self { roles }
    }
}

impl From<Vec<&str>> for Secured {
    fn from(roles: Vec<&str>) -> Self {
        Self {
            roles: roles.into_iter().map(String::from).collect(),
        }
    }
}
