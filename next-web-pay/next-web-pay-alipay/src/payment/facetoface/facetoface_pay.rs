use std::collections::BTreeMap;

use chrono::Local;

// pub trait FacetofacePay {
//     fn pay(
//         &self,
//         subject: impl Into<String>,
//         out_trade_no: impl Into<String>,
//         total_amount: impl Into<String>,
//         auth_code: impl Into<String>,
//     ) -> impl Future<Output = AlipayResult<AlipayTradePayResponse>> + Send;
// }

// impl<T> FacetofacePay for T
// where
//     T: AsRef<AlipayClient>,
//     T: Sync,
// {
//     fn pay(
//         &self,
//         subject: impl Into<String>,
//         out_trade_no: impl Into<String>,
//         total_amount: impl Into<String>,
//         auth_code: impl Into<String>,
//     ) -> impl Future<Output = AlipayResult<AlipayTradePayResponse>> + Send {
//         let subject = subject.into();
//         let out_trade_no = out_trade_no.into();
//         let total_amount = total_amount.into();
//         let auth_code = auth_code.into();

//         async move {
//             let r = build_pay_biz_params(&subject, &out_trade_no, &total_amount, &auth_code);
//             let _ = self
//                 .as_ref()
//                 .client()
//                 .post(AlipayConfig::DEFAULT_GATEWAY_URL)
//                 .header(
//                     CONTENT_TYPE,
//                     "application/x-www-form-urlencoded;charset=utf-8",
//                 )
//                 .form(&r)
//                 .send()
//                 .await;

//             Err(AlipayError::InvalidSignature)
//         }
//     }
// }
