#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CommonErrorCode {
    /// 接口调用成功
    Success,

    /// 服务不可用
    ServiceUnavailable,

    /// 授权权限不足
    AuthorizationInsufficient,

    /// 缺少必选参数
    MissingRequiredParameter,

    /// 非法的参数
    InvalidParameter,

    /// 条件异常
    InsufficientConditions,

    /// 业务处理失败
    BusinessProcessFailed,

    /// 调用频次超限
    CallLimited,

    /// 权限不足
    InsufficientPermissions,
}

impl TryFrom<&str> for CommonErrorCode {
    type Error = &'static str;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "10000" => Ok(Self::Success),
            "20000" => Ok(Self::ServiceUnavailable),
            "20001" => Ok(Self::AuthorizationInsufficient),
            "40001" => Ok(Self::MissingRequiredParameter),
            "40002" => Ok(Self::InvalidParameter),
            "40003" => Ok(Self::InsufficientConditions),
            "40004" => Ok(Self::BusinessProcessFailed),
            "40005" => Ok(Self::CallLimited),
            "40006" => Ok(Self::InsufficientPermissions),
            _ => Err("Unknown error code"),
        }
    }
}

impl TryFrom<u32> for CommonErrorCode {
    type Error = &'static str;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            10000 => Ok(Self::Success),
            20000 => Ok(Self::ServiceUnavailable),
            20001 => Ok(Self::AuthorizationInsufficient),
            40001 => Ok(Self::MissingRequiredParameter),
            40002 => Ok(Self::InvalidParameter),
            40003 => Ok(Self::InsufficientConditions),
            40004 => Ok(Self::BusinessProcessFailed),
            40005 => Ok(Self::CallLimited),
            40006 => Ok(Self::InsufficientPermissions),
            _ => Err("Unknown error code"),
        }
    }
}
