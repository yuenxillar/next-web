

#[derive(Clone)]
pub struct DefaultSubjectFactory {}

// The `v2` tree always includes the web integration, so the subject factory of
// an application is `DefaultWebSubjectFactory`; this type stays as the name a
// configuration may refer to.

impl Default for DefaultSubjectFactory {
    fn default() -> Self {
        Self {}
    }
}
