#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoginType {
    Email,
    Phone,
    Wechat,
    Paypal,
}

impl Default for LoginType {
    fn default() -> Self {
        LoginType::Email
    }
}
impl Into<LoginType> for u8 {
    fn into(self) -> LoginType {
        match self {
            1 => LoginType::Email,
            2 => LoginType::Phone,
            3 => LoginType::Wechat,
            4 => LoginType::Paypal,
            _ => LoginType::Email,
        }
    }
}
