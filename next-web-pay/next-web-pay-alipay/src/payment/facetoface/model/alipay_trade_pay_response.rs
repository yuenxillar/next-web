// This file is auto-generated, don't edit it.

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AlipayTradePayResponse {
    http_body: String,
    code: String,
    msg: String,
    sub_code: String,
    sub_msg: String,
    trade_no: String,
    out_trade_no: String,
    buyer_logon_id: String,
    settle_amount: String,
    pay_currency: String,
    pay_amount: String,
    settle_trans_rate: String,
    trans_pay_rate: String,
    total_amount: String,
    trans_currency: String,
    settle_currency: String,
    receipt_amount: String,
    buyer_pay_amount: String,
    point_amount: String,
    invoice_amount: String,
    gmt_payment: String,
    fund_bill_list: Vec<super::TradeFundBill>,
    card_balance: String,
    store_name: String,
    buyer_user_id: String,
    discount_goods_detail: String,
    voucher_detail_list: Vec<super::VoucherDetail>,
    advance_amount: String,
    auth_trade_pay_mode: String,
    charge_amount: String,
    charge_flags: String,
    settlement_id: String,
    business_params: String,
    buyer_user_type: String,
    mdiscount_amount: String,
    discount_amount: String,
    buyer_user_name: String,
}

impl AlipayTradePayResponse {
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

    /// Returns the `trade_no` field.
    pub fn trade_no(&self) -> &str {
        self.trade_no.as_str()
    }

    /// Returns the `out_trade_no` field.
    pub fn out_trade_no(&self) -> &str {
        self.out_trade_no.as_str()
    }

    /// Returns the `buyer_logon_id` field.
    pub fn buyer_logon_id(&self) -> &str {
        self.buyer_logon_id.as_str()
    }

    /// Returns the `settle_amount` field.
    pub fn settle_amount(&self) -> &str {
        self.settle_amount.as_str()
    }

    /// Returns the `pay_currency` field.
    pub fn pay_currency(&self) -> &str {
        self.pay_currency.as_str()
    }

    /// Returns the `pay_amount` field.
    pub fn pay_amount(&self) -> &str {
        self.pay_amount.as_str()
    }

    /// Returns the `settle_trans_rate` field.
    pub fn settle_trans_rate(&self) -> &str {
        self.settle_trans_rate.as_str()
    }

    /// Returns the `trans_pay_rate` field.
    pub fn trans_pay_rate(&self) -> &str {
        self.trans_pay_rate.as_str()
    }

    /// Returns the `total_amount` field.
    pub fn total_amount(&self) -> &str {
        self.total_amount.as_str()
    }

    /// Returns the `trans_currency` field.
    pub fn trans_currency(&self) -> &str {
        self.trans_currency.as_str()
    }

    /// Returns the `settle_currency` field.
    pub fn settle_currency(&self) -> &str {
        self.settle_currency.as_str()
    }

    /// Returns the `receipt_amount` field.
    pub fn receipt_amount(&self) -> &str {
        self.receipt_amount.as_str()
    }

    /// Returns the `buyer_pay_amount` field.
    pub fn buyer_pay_amount(&self) -> &str {
        self.buyer_pay_amount.as_str()
    }

    /// Returns the `point_amount` field.
    pub fn point_amount(&self) -> &str {
        self.point_amount.as_str()
    }

    /// Returns the `invoice_amount` field.
    pub fn invoice_amount(&self) -> &str {
        self.invoice_amount.as_str()
    }

    /// Returns the `gmt_payment` field.
    pub fn gmt_payment(&self) -> &str {
        self.gmt_payment.as_str()
    }

    /// Returns the `fund_bill_list` field.
    pub fn fund_bill_list(&self) -> &Vec<super::TradeFundBill> {
        &self.fund_bill_list
    }

    /// Returns the `card_balance` field.
    pub fn card_balance(&self) -> &str {
        self.card_balance.as_str()
    }

    /// Returns the `store_name` field.
    pub fn store_name(&self) -> &str {
        self.store_name.as_str()
    }

    /// Returns the `buyer_user_id` field.
    pub fn buyer_user_id(&self) -> &str {
        self.buyer_user_id.as_str()
    }

    /// Returns the `discount_goods_detail` field.
    pub fn discount_goods_detail(&self) -> &str {
        self.discount_goods_detail.as_str()
    }

    /// Returns the `voucher_detail_list` field.
    pub fn voucher_detail_list(&self) -> &Vec<super::VoucherDetail> {
        &self.voucher_detail_list
    }

    /// Returns the `advance_amount` field.
    pub fn advance_amount(&self) -> &str {
        self.advance_amount.as_str()
    }

    /// Returns the `auth_trade_pay_mode` field.
    pub fn auth_trade_pay_mode(&self) -> &str {
        self.auth_trade_pay_mode.as_str()
    }

    /// Returns the `charge_amount` field.
    pub fn charge_amount(&self) -> &str {
        self.charge_amount.as_str()
    }

    /// Returns the `charge_flags` field.
    pub fn charge_flags(&self) -> &str {
        self.charge_flags.as_str()
    }

    /// Returns the `settlement_id` field.
    pub fn settlement_id(&self) -> &str {
        self.settlement_id.as_str()
    }

    /// Returns the `business_params` field.
    pub fn business_params(&self) -> &str {
        self.business_params.as_str()
    }

    /// Returns the `buyer_user_type` field.
    pub fn buyer_user_type(&self) -> &str {
        self.buyer_user_type.as_str()
    }

    /// Returns the `mdiscount_amount` field.
    pub fn mdiscount_amount(&self) -> &str {
        self.mdiscount_amount.as_str()
    }

    /// Returns the `discount_amount` field.
    pub fn discount_amount(&self) -> &str {
        self.discount_amount.as_str()
    }

    /// Returns the `buyer_user_name` field.
    pub fn buyer_user_name(&self) -> &str {
        self.buyer_user_name.as_str()
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

    /// Sets the `trade_no` field.
    pub fn set_trade_no(&mut self, value: String) {
        self.trade_no = value;
    }

    /// Sets the `out_trade_no` field.
    pub fn set_out_trade_no(&mut self, value: String) {
        self.out_trade_no = value;
    }

    /// Sets the `buyer_logon_id` field.
    pub fn set_buyer_logon_id(&mut self, value: String) {
        self.buyer_logon_id = value;
    }

    /// Sets the `settle_amount` field.
    pub fn set_settle_amount(&mut self, value: String) {
        self.settle_amount = value;
    }

    /// Sets the `pay_currency` field.
    pub fn set_pay_currency(&mut self, value: String) {
        self.pay_currency = value;
    }

    /// Sets the `pay_amount` field.
    pub fn set_pay_amount(&mut self, value: String) {
        self.pay_amount = value;
    }

    /// Sets the `settle_trans_rate` field.
    pub fn set_settle_trans_rate(&mut self, value: String) {
        self.settle_trans_rate = value;
    }

    /// Sets the `trans_pay_rate` field.
    pub fn set_trans_pay_rate(&mut self, value: String) {
        self.trans_pay_rate = value;
    }

    /// Sets the `total_amount` field.
    pub fn set_total_amount(&mut self, value: String) {
        self.total_amount = value;
    }

    /// Sets the `trans_currency` field.
    pub fn set_trans_currency(&mut self, value: String) {
        self.trans_currency = value;
    }

    /// Sets the `settle_currency` field.
    pub fn set_settle_currency(&mut self, value: String) {
        self.settle_currency = value;
    }

    /// Sets the `receipt_amount` field.
    pub fn set_receipt_amount(&mut self, value: String) {
        self.receipt_amount = value;
    }

    /// Sets the `buyer_pay_amount` field.
    pub fn set_buyer_pay_amount(&mut self, value: String) {
        self.buyer_pay_amount = value;
    }

    /// Sets the `point_amount` field.
    pub fn set_point_amount(&mut self, value: String) {
        self.point_amount = value;
    }

    /// Sets the `invoice_amount` field.
    pub fn set_invoice_amount(&mut self, value: String) {
        self.invoice_amount = value;
    }

    /// Sets the `gmt_payment` field.
    pub fn set_gmt_payment(&mut self, value: String) {
        self.gmt_payment = value;
    }

    /// Sets the `fund_bill_list` field.
    pub fn set_fund_bill_list(&mut self, value: Vec<super::TradeFundBill>) {
        self.fund_bill_list = value;
    }

    /// Sets the `card_balance` field.
    pub fn set_card_balance(&mut self, value: String) {
        self.card_balance = value;
    }

    /// Sets the `store_name` field.
    pub fn set_store_name(&mut self, value: String) {
        self.store_name = value;
    }

    /// Sets the `buyer_user_id` field.
    pub fn set_buyer_user_id(&mut self, value: String) {
        self.buyer_user_id = value;
    }

    /// Sets the `discount_goods_detail` field.
    pub fn set_discount_goods_detail(&mut self, value: String) {
        self.discount_goods_detail = value;
    }

    /// Sets the `voucher_detail_list` field.
    pub fn set_voucher_detail_list(&mut self, value: Vec<super::VoucherDetail>) {
        self.voucher_detail_list = value;
    }

    /// Sets the `advance_amount` field.
    pub fn set_advance_amount(&mut self, value: String) {
        self.advance_amount = value;
    }

    /// Sets the `auth_trade_pay_mode` field.
    pub fn set_auth_trade_pay_mode(&mut self, value: String) {
        self.auth_trade_pay_mode = value;
    }

    /// Sets the `charge_amount` field.
    pub fn set_charge_amount(&mut self, value: String) {
        self.charge_amount = value;
    }

    /// Sets the `charge_flags` field.
    pub fn set_charge_flags(&mut self, value: String) {
        self.charge_flags = value;
    }

    /// Sets the `settlement_id` field.
    pub fn set_settlement_id(&mut self, value: String) {
        self.settlement_id = value;
    }

    /// Sets the `business_params` field.
    pub fn set_business_params(&mut self, value: String) {
        self.business_params = value;
    }

    /// Sets the `buyer_user_type` field.
    pub fn set_buyer_user_type(&mut self, value: String) {
        self.buyer_user_type = value;
    }

    /// Sets the `mdiscount_amount` field.
    pub fn set_mdiscount_amount(&mut self, value: String) {
        self.mdiscount_amount = value;
    }

    /// Sets the `discount_amount` field.
    pub fn set_discount_amount(&mut self, value: String) {
        self.discount_amount = value;
    }

    /// Sets the `buyer_user_name` field.
    pub fn set_buyer_user_name(&mut self, value: String) {
        self.buyer_user_name = value;
    }
}
