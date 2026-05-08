use std::any::Any;

use crate::{
    AlipayError, AlipayResult, Method, Named,
    client::AlipayClient,
    payment::{common::model::*, model::AlipayResponse},
};

/// 支付宝交易支付 Trait
///
/// 定义了与支付宝交易相关的所有核心操作接口，包括：
/// - 预创建订单
/// - 支付
/// - 查询
/// - 撤销
/// - 退款
/// - 退款查询
/// - 交易关闭
/// - 账单下载地址查询
pub trait AlipayTradePay {
    /// 统一收单线下交易预创建
    ///
    /// 用于在线下场景中，通过预创建订单的方式生成交易订单
    /// 适用于扫码支付等场景，商户可先创建订单再让用户支付
    ///
    /// # 参数
    /// * `req` - 预创建订单请求参数，包含商户订单号、订单金额、商品描述等信息
    ///
    /// # 返回值
    /// 返回预创建订单的响应结果，包含支付宝交易号、二维码链接等信息
    fn precreate(
        &self,
        req: AlipayTradePrecreateRequest,
    ) -> impl Future<Output = AlipayResult<AlipayResponse<AlipayTradePrecreateResponse>>> + Send;

    /// 统一收单交易支付接口
    ///
    /// 用于直接发起支付请求，支持多种支付场景
    /// 适用于收银员通过收银设备发起支付，或者用户通过 PC/手机端直接支付
    ///
    /// # 参数
    /// * `req` - 支付请求参数，包含支付金额、支付方式、商户订单号等
    ///
    /// # 返回值
    /// 返回支付结果，包含交易状态、买家信息、支付时间等详细信息
    fn pay(
        &self,
        req: AlipayTradePayRequest,
    ) -> impl Future<Output = AlipayResult<AlipayResponse<AlipayTradePayResponse>>> + Send;

    /// 统一收单线下交易查询
    ///
    /// 查询指定订单的支付状态和详细信息
    /// 支持通过支付宝交易号或商户订单号进行查询
    ///
    /// # 参数
    /// * `req` - 查询请求参数，可传入支付宝交易号或商户订单号
    ///
    /// # 返回值
    /// 返回订单的详细信息，包括交易状态、支付金额、买家信息、清算信息等
    fn query(
        &self,
        req: AlipayTradeQueryRequest,
    ) -> impl Future<Output = AlipayResult<AlipayResponse<AlipayTradeQueryResponse>>> + Send;

    /// 统一收单交易撤销接口
    ///
    /// 用于撤销指定订单的支付交易
    /// 通常在支付异常或需要立即取消交易时使用
    /// 注意：只有特定状态的订单才支持撤销操作
    ///
    /// # 参数
    /// * `req` - 撤销请求参数，包含要撤销的订单标识信息
    ///
    /// # 返回值
    /// 返回撤销操作的结果，包含是否成功及重试标识等信息
    fn cancel(
        &self,
        req: AlipayTradeCancelRequest,
    ) -> impl Future<Output = AlipayResult<AlipayResponse<AlipayTradeCancelResponse>>> + Send;

    /// 统一收单交易退款接口
    ///
    /// 对已支付的订单进行退款操作
    /// 支持部分退款和全额退款，退款金额不能超过原订单金额
    ///
    /// # 参数
    /// * `req` - 退款请求参数，包含退款金额、退款原因、退款请求号等
    ///
    /// # 返回值
    /// 返回退款操作的结果，包含退款金额、退款时间、退款批次号等信息
    fn refund(
        &self,
        req: AlipayTradeRefundRequest,
    ) -> impl Future<Output = AlipayResult<AlipayResponse<AlipayTradeRefundResponse>>> + Send;

    /// 统一收单退款查询接口
    ///
    /// 查询指定退款请求的处理状态和结果
    /// 可通过退款请求号或支付宝交易号来查询退款详情
    ///
    /// # 参数
    /// * `req` - 退款查询请求参数，包含退款请求号等查询条件
    ///
    /// # 返回值
    /// 返回退款的详细信息，包括退款金额、退款状态、退款时间等
    fn refund_query(
        &self,
        req: AlipayTradeRefundQueryRequest,
    ) -> impl Future<Output = AlipayResult<AlipayResponse<AlipayTradeRefundQueryResponse>>> + Send;

    /// 统一收单交易关闭接口
    ///
    /// 关闭指定的交易订单，释放订单占用的资源
    /// 适用于未支付的订单，支付成功或已全额退款的订单无法关闭
    /// 关闭后的订单将无法再进行支付操作
    ///
    /// # 参数
    /// * `req` - 关闭交易请求参数，包含要关闭的订单标识
    ///
    /// # 返回值
    /// 返回交易关闭操作的结果，包含交易号和是否重试标识
    fn close(
        &self,
        req: AlipayTradeCloseRequest,
    ) -> impl Future<Output = AlipayResult<AlipayResponse<AlipayTradeCloseResponse>>> + Send;

    /// 查询账单下载地址接口
    ///
    /// 获取指定日期的对账单下载地址
    /// 用于商户下载交易对账单，进行对账和数据分析
    /// 注意：账单下载地址有时间限制，需要在获取后尽快下载
    ///
    /// # 参数
    /// * `req` - 账单查询请求参数，包含账单日期、账单类型等信息
    ///
    /// # 返回值
    /// 返回账单下载地址，包含下载链接和有效期等信息
    fn bill_downloadurl_query(
        &self,
        req: AlipayTradeBillDownloadurlQueryRequest,
    ) -> impl Future<Output = AlipayResult<AlipayResponse<AlipayTradeBillDownloadurlQueryResponse>>> + Send;

    /// 下载支付宝对账单文件
    ///
    /// 该方法用于下载指定日期和类型的支付宝交易对账单，返回账单文件的原始字节数据。
    /// 通常先调用此方法获取对账单内容，然后由调用方决定是保存为文件还是直接解析处理。
    ///
    /// # 工作流程
    /// 1. 根据请求参数（账单日期、账单类型等）向支付宝发起查询
    /// 2. 支付宝返回一个临时的账单下载地址
    /// 3. 从该地址下载账单文件内容
    /// 4. 将下载的字节数据返回给调用方
    ///
    /// # 参数
    /// * `req` - 账单下载请求参数
    ///
    /// # 返回值
    /// * `Ok(Vec<u8>)` - 账单文件的原始字节数据
    ///   - 通常为 CSV 格式的文本文件，可直接写入磁盘或用 CSV 解析器处理
    ///   - 也可能是 ZIP 压缩包，需要先解压再读取
    /// * `Err(AlipayError)` - 下载失败时返回错误，可能的原因包括：
    ///   - 网络连接失败
    ///   - 账单日期格式错误
    ///   - 账单不存在（当日无交易或账单未生成）
    ///   - 下载地址已过期
    ///   - 支付宝 API 调用失败（签名错误、参数错误等）
    ///
    fn bill_download(
        &self,
        req: AlipayTradeBillDownloadurlQueryRequest,
    ) -> impl Future<Output = AlipayResult<Vec<u8>>> + Send;
}

impl<T> AlipayTradePay for T
where
    T: AsRef<AlipayClient>,
    T: Sync,
{
    fn precreate(
        &self,
        req: AlipayTradePrecreateRequest,
    ) -> impl Future<Output = AlipayResult<AlipayResponse<AlipayTradePrecreateResponse>>> + Send
    {
        self.call(req)
    }

    fn pay(
        &self,
        req: AlipayTradePayRequest,
    ) -> impl Future<Output = AlipayResult<AlipayResponse<AlipayTradePayResponse>>> + Send {
        self.call(req)
    }

    fn query(
        &self,
        req: AlipayTradeQueryRequest,
    ) -> impl Future<Output = AlipayResult<AlipayResponse<AlipayTradeQueryResponse>>> + Send {
        self.call(req)
    }

    fn cancel(
        &self,
        req: AlipayTradeCancelRequest,
    ) -> impl Future<Output = AlipayResult<AlipayResponse<AlipayTradeCancelResponse>>> + Send {
        self.call(req)
    }

    fn refund(
        &self,
        req: AlipayTradeRefundRequest,
    ) -> impl Future<Output = AlipayResult<AlipayResponse<AlipayTradeRefundResponse>>> + Send {
        self.call(req)
    }

    fn refund_query(
        &self,
        req: AlipayTradeRefundQueryRequest,
    ) -> impl Future<Output = AlipayResult<AlipayResponse<AlipayTradeRefundQueryResponse>>> + Send
    {
        self.call(req)
    }

    fn close(
        &self,
        req: AlipayTradeCloseRequest,
    ) -> impl Future<Output = AlipayResult<AlipayResponse<AlipayTradeCloseResponse>>> + Send {
        self.call(req)
    }

    fn bill_downloadurl_query(
        &self,
        req: AlipayTradeBillDownloadurlQueryRequest,
    ) -> impl Future<Output = AlipayResult<AlipayResponse<AlipayTradeBillDownloadurlQueryResponse>>> + Send
    {
        self.call(req)
    }

    fn bill_download(
        &self,
        req: AlipayTradeBillDownloadurlQueryRequest,
    ) -> impl Future<Output = AlipayResult<Vec<u8>>> + Send {
        async move {
            let resp = self.bill_downloadurl_query(req).await?;

            if !resp.is_success() {
                return Err(AlipayError::Custom(resp.to_error_string()));
            }

            let download_url = resp.data().bill_download_url;
            if download_url.is_empty() {
                return Err(AlipayError::Custom(
                    "bill_download_url is empty".to_string(),
                ));
            }

            let resp = self.as_ref().client().get(download_url).send().await?;
            let data = resp.bytes().await?;

            Ok(data.into())
        }
    }
}

#[allow(unused)]
pub trait AlipayTradePayExt {
    fn call<Req, Resp>(
        &self,
        req: Req,
    ) -> impl Future<Output = AlipayResult<AlipayResponse<Resp>>> + Send
    where
        Req: Method + serde::Serialize,
        Req: Send + Sync,
        Req: Any,
        Resp: Named + serde::de::DeserializeOwned,
        Resp: Send + Sync;
}

impl<T> AlipayTradePayExt for T
where
    T: AsRef<AlipayClient>,
    T: Sync,
{
    fn call<Req, Resp>(
        &self,
        req: Req,
    ) -> impl Future<Output = AlipayResult<AlipayResponse<Resp>>> + Send
    where
        Req: Method + serde::Serialize,
        Req: Send + Sync,
        Req: Any,
        Resp: Named + serde::de::DeserializeOwned,
        Resp: Send + Sync,
    {
        async move {
            let query = self
                .as_ref()
                .signed_params(Req::method(), &req, None)
                .map_err(|err| AlipayError::Signing(err))?;

            let mut form_data = Vec::with_capacity(2);
            form_data.push((
                "biz_content",
                query
                    .get("biz_content")
                    .map(ToString::to_string)
                    .unwrap_or(serde_json::to_string(&req)?),
            ));
            if let Some(app_auth_token) = self.as_ref().config().app_auth_token() {
                form_data.push(("app_auth_token", app_auth_token.into()));
            }

            let url = self.as_ref().config().gateway_url();

            let resp = self
                .as_ref()
                .client()
                .post(url)
                .query(&query)
                .form(&form_data)
                .send()
                .await?
                .json::<AlipayResponse<Resp>>()
                .await?;

            Ok(resp)
        }
    }
}
