use std::fmt::{Debug, Display, format};

use serde::{Deserialize, Deserializer, de::DeserializeOwned};
use serde_json::Value;

use crate::Named;

#[derive(Debug, Clone)]
pub struct AlipayResponse<T>
where
    T: serde::de::DeserializeOwned,
    T: Named,
{
    /// 网关返回码
    pub code: String,

    /// 网关返回码描述
    pub msg: String,

    /// 签名
    pub sign: String,

    /// 业务数据
    data: Response<T>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(untagged)]
pub enum Response<T> {
    Success(T),
    Error(ErrorResponse),
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ErrorResponse {
    /// 错误子码
    pub sub_code: String,
    /// 错误子描述
    pub sub_msg: String,
}

impl<T> AlipayResponse<T>
where
    T: serde::de::DeserializeOwned,
    T: Named,
{
    pub fn is_success(&self) -> bool {
        self.code == "10000"
    }

    pub fn data(self) -> T {
        match self.data {
            Response::Success(data) => data,
            Response::Error(_) => panic!("ErrorResponse does not have data"),
        }
    }

    pub fn to_error_string(self) -> String {
        match self.data {
            Response::Success(_) => panic!("SuccessResponse does not have error"),
            Response::Error(error) => format!(
                "code: {}, msg: {}, sub_code: {}, sub_msg: {}",
                self.code, self.msg, error.sub_code, error.sub_msg
            ),
        }
    }
}
impl<'de, T> Deserialize<'de> for AlipayResponse<T>
where
    T: DeserializeOwned + Named,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error;

        // 1. 解析整个响应为 Value
        let mut value: Value = Deserialize::deserialize(deserializer)?;

        // 2. 提取 sign 字段
        let sign = match value["sign"].take() {
            Value::String(sign) => sign,
            _ => String::new(),
        };

        // 3. 根据 T 的 name() 获取业务数据字段名，比如 "alipay_trade_pay_response"
        let biz_key = T::name();
        let biz_body = value
            .get_mut(biz_key)
            .ok_or_else(|| Error::custom(format!("missing field: {}", biz_key)))?;

        // 4. 从业务数据中提取公共字段
        let code = match biz_body["code"].take() {
            Value::String(code) => code,
            _ => String::new(),
        };

        let msg = match biz_body["msg"].take() {
            Value::String(msg) => msg,
            _ => String::new(),
        };

        // 5. 反序列化业务数据 T（包含 code/msg 在内的所有字段）
        let data: Response<T> = serde_json::from_value(biz_body.take()).map_err(Error::custom)?;

        Ok(AlipayResponse {
            code,
            msg,
            sign,
            data,
        })
    }
}
