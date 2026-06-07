#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum SessionCreationPolicy {
  
    Always,
  
    Never,
 
    IfRequired,
  
    Stateless,
}
