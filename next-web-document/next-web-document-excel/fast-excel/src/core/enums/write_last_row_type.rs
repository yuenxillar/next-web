/// The types of write last row
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WriteLastRowType {
    /// Excel are created without templates ,And any data has been written;
    CommonEmpty,
    /// Excel are created with templates ,And any data has been written;
    TemplateEmpty,
    /// Any data has been written;
    HasData,
}
