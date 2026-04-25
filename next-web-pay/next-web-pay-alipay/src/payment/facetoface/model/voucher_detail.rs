// This file is auto-generated, don't edit it.

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct VoucherDetail {
    id: String,
    name: String,
    #[serde(rename = "type")]
    _type: String,
    amount: String,
    merchant_contribute: String,
    other_contribute: String,
    memo: String,
    template_id: String,
    purchase_buyer_contribute: String,
    purchase_merchant_contribute: String,
    purchase_ant_contribute: String,
}


impl VoucherDetail {
    /// Returns the `id` field.
    pub fn id(&self) -> &str {
        self.id.as_str()
    }

    /// Returns the `name` field.
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    /// Returns the `_type` field.
    pub fn _type(&self) -> &str {
        self._type.as_str()
    }

    /// Returns the `amount` field.
    pub fn amount(&self) -> &str {
        self.amount.as_str()
    }

    /// Returns the `merchant_contribute` field.
    pub fn merchant_contribute(&self) -> &str {
        self.merchant_contribute.as_str()
    }

    /// Returns the `other_contribute` field.
    pub fn other_contribute(&self) -> &str {
        self.other_contribute.as_str()
    }

    /// Returns the `memo` field.
    pub fn memo(&self) -> &str {
        self.memo.as_str()
    }

    /// Returns the `template_id` field.
    pub fn template_id(&self) -> &str {
        self.template_id.as_str()
    }

    /// Returns the `purchase_buyer_contribute` field.
    pub fn purchase_buyer_contribute(&self) -> &str {
        self.purchase_buyer_contribute.as_str()
    }

    /// Returns the `purchase_merchant_contribute` field.
    pub fn purchase_merchant_contribute(&self) -> &str {
        self.purchase_merchant_contribute.as_str()
    }

    /// Returns the `purchase_ant_contribute` field.
    pub fn purchase_ant_contribute(&self) -> &str {
        self.purchase_ant_contribute.as_str()
    }

    /// Sets the `id` field.
    pub fn set_id(&mut self, value: String) {
        self.id = value;
    }

    /// Sets the `name` field.
    pub fn set_name(&mut self, value: String) {
        self.name = value;
    }

    /// Sets the `_type` field.
    pub fn set_type(&mut self, value: String) {
        self._type = value;
    }

    /// Sets the `amount` field.
    pub fn set_amount(&mut self, value: String) {
        self.amount = value;
    }

    /// Sets the `merchant_contribute` field.
    pub fn set_merchant_contribute(&mut self, value: String) {
        self.merchant_contribute = value;
    }

    /// Sets the `other_contribute` field.
    pub fn set_other_contribute(&mut self, value: String) {
        self.other_contribute = value;
    }

    /// Sets the `memo` field.
    pub fn set_memo(&mut self, value: String) {
        self.memo = value;
    }

    /// Sets the `template_id` field.
    pub fn set_template_id(&mut self, value: String) {
        self.template_id = value;
    }

    /// Sets the `purchase_buyer_contribute` field.
    pub fn set_purchase_buyer_contribute(&mut self, value: String) {
        self.purchase_buyer_contribute = value;
    }

    /// Sets the `purchase_merchant_contribute` field.
    pub fn set_purchase_merchant_contribute(&mut self, value: String) {
        self.purchase_merchant_contribute = value;
    }

    /// Sets the `purchase_ant_contribute` field.
    pub fn set_purchase_ant_contribute(&mut self, value: String) {
        self.purchase_ant_contribute = value;
    }

}