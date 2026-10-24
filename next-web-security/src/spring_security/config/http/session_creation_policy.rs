/// Specifies the various session creation policies for Spring Security.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum SessionCreationPolicy {
    /// Always create an HttpSession
    Always,

    /// Next Security will never create an HttpSession, but will use the HttpSession if it already exists
    Never,

    /// Next Security will only create an HttpSession if required
    IfRequired,

    /// Next Security will never create an HttpSession and it will never use it to obtain the SecurityContext
    Stateless,
}
