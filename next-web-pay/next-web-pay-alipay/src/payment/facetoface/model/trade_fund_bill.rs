// This file is auto-generated, don't edit it.

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct TradeFundBill {
    fund_channel: String,
    bank_code: String,
    amount: String,
    real_amount: String,
}

impl TradeFundBill {
    /// Returns the `fund_channel` field.
    pub fn fund_channel(&self) -> &str {
        self.fund_channel.as_str()
    }

    /// Returns the `bank_code` field.
    pub fn bank_code(&self) -> &str {
        self.bank_code.as_str()
    }

    /// Returns the `amount` field.
    pub fn amount(&self) -> &str {
        self.amount.as_str()
    }

    /// Returns the `real_amount` field.
    pub fn real_amount(&self) -> &str {
        self.real_amount.as_str()
    }

    /// Sets the `fund_channel` field.
    pub fn set_fund_channel(&mut self, value: String) {
        self.fund_channel = value;
    }

    /// Sets the `bank_code` field.
    pub fn set_bank_code(&mut self, value: String) {
        self.bank_code = value;
    }

    /// Sets the `amount` field.
    pub fn set_amount(&mut self, value: String) {
        self.amount = value;
    }

    /// Sets the `real_amount` field.
    pub fn set_real_amount(&mut self, value: String) {
        self.real_amount = value;
    }
}
