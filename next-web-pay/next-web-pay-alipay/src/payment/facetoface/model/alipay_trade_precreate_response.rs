// This file is auto-generated, don't edit it.

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AlipayTradePrecreateResponse {
    http_body: String,
    code: String,
    msg: String,
    sub_code: String,
    sub_msg: String,
    out_trade_no: String,
    qr_code: String,
}

impl AlipayTradePrecreateResponse {
    /// Returns the `http_body` field.
    pub fn http_body(&self) -> &str {
        self.http_body.as_str()
    }

    /// Returns the `code` field.
    pub fn code(&self) -> &str {
        self.code.as_str()
    }

    /// Returns the `msg` field.
    pub fn msg(&self) -> &str {
        self.msg.as_str()
    }

    /// Returns the `sub_code` field.
    pub fn sub_code(&self) -> &str {
        self.sub_code.as_str()
    }

    /// Returns the `sub_msg` field.
    pub fn sub_msg(&self) -> &str {
        self.sub_msg.as_str()
    }

    /// Returns the `out_trade_no` field.
    pub fn out_trade_no(&self) -> &str {
        self.out_trade_no.as_str()
    }

    /// Returns the `qr_code` field.
    pub fn qr_code(&self) -> &str {
        self.qr_code.as_str()
    }

    /// Sets the `http_body` field.
    pub fn set_http_body(&mut self, value: String) {
        self.http_body = value;
    }

    /// Sets the `code` field.
    pub fn set_code(&mut self, value: String) {
        self.code = value;
    }

    /// Sets the `msg` field.
    pub fn set_msg(&mut self, value: String) {
        self.msg = value;
    }

    /// Sets the `sub_code` field.
    pub fn set_sub_code(&mut self, value: String) {
        self.sub_code = value;
    }

    /// Sets the `sub_msg` field.
    pub fn set_sub_msg(&mut self, value: String) {
        self.sub_msg = value;
    }

    /// Sets the `out_trade_no` field.
    pub fn set_out_trade_no(&mut self, value: String) {
        self.out_trade_no = value;
    }

    /// Sets the `qr_code` field.
    pub fn set_qr_code(&mut self, value: String) {
        self.qr_code = value;
    }

}