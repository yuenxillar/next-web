
pub trait PatternMatcher
where 
Self: Send + Sync 
{
    fn matches(&self, pattern: &str, source: &str) -> bool;
}