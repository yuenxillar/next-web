/// 非法状态错误
#[derive( Debug)]
pub struct IllegalStateError {
   pub msg: String,
}

impl std::fmt::Display for IllegalStateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "IllegalStateError: {}", self.msg)
    }
}