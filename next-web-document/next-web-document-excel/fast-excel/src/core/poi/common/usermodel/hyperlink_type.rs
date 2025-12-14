/// Represents the type of a hyperlink.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HyperlinkType {
    /// Not a hyperlink (internal use)
    None,

    /// Link to an existing file or web page
    Url,

    /// Link to a place in this document
    Document,

    /// Link to an E-mail address
    Email,

    /// Link to a file
    File,
}
