
pub trait DataDecoder: Send + Sync {
    
    fn decode(&self, data: &[u8]) -> Result<String, &'static str>;
}