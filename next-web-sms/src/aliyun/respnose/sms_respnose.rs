use std::fmt;

use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AliyunCloudSmsResponse {
    #[serde(deserialize_with = "deserialize_str")]
    pub code: RespCode,
    /// 状态码的描述。
    pub message: String,
    /// 发送回执 ID。
    /// 可根据发送回执 ID 在接口 QuerySendDetails 中查询具体的发送状态。
    pub buz_id: Option<String>,
    /// 请求 ID。
    pub request_id: String,
}

fn deserialize_str<'de, D>(deserializer: D) -> Result<RespCode, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let code: String = String::deserialize(deserializer)?;
    if code.is_empty() {
        Err(serde::de::Error::custom("code is empty!"))
    } else {
        Ok(code.into())
    }
}

#[derive(Clone, Debug, Deserialize)]
pub enum RespCode {
    /// 发送成功
    Ok,

    /// 签名和模板类型不一致
    ///
    /// 原因：模板和签名类型不一致。错误示例：用验证码签名发送了通知短信或推广短信。
    ///
    /// 解决方案：签名分为验证码和通用两种类型。模板分为验证码、短信通知和推广短信三种类型。
    /// 其中验证码类签名只能发送验证码模板，通用类签名可以发送全部类型的模板。
    /// 建议您把签名更改为通用类型。
    SMSSignatureSceneIllegal,

    /// 扩展码使用错误，相同的扩展码不可用于多个签名
    ///
    /// 原因：发送短信时，不同签名的短信使用了相同的扩展码。
    ///
    /// 解决方案：在调用短信发送接口时，不同的短信签名使用不同的扩展码。
    ExtendCodeError,

    /// 国际/港澳台消息模板不支持发送境内号码
    ///
    /// 原因：国际/港澳台消息模板仅支持发送国际、中国港澳台地区的号码。
    ///
    /// 解决方案：如果要发送中国内地短信，请申请国内消息短信模板。
    DomesticNumberNotSupported,

    /// 原IP地址所在的地区被禁用
    ///
    /// 原因：被系统检测到原IP属于非中国内地地区。
    ///
    /// 解决方案：请将原IP地址修改为中国内地地区的IP地址。国际/港澳台的IP地址禁止发送中国内地短信业务。
    /// 更多详情，请参见服务接入点。
    DenyIpRange,

    /// 触发日发送限额
    ///
    /// 原因：已经达到您在控制台设置的短信日发送量限额值。
    ///
    /// 解决方案：如果需要修改限额，请登录短信服务控制台，在通用设置 > 国内消息设置 > 安全设置页面，
    /// 修改日发送量总量阈值。
    DayLimitControl,

    /// 触发月发送限额
    ///
    /// 原因：已经达到您在控制台设置的短信月发送量限额值。
    ///
    /// 解决方案：如果需要修改限额，请登录短信服务控制台，在通用设置 > 国内消息设置 > 安全设置页面，
    /// 修改月发送量总量阈值。
    MonthLimitControl,

    /// 短信内容包含禁止发送内容
    ///
    /// 原因：短信内容包含禁止发送内容。
    ///
    /// 解决方案：修改短信文案，规范详情请参见短信模板规范。
    SMSContentIllegal,

    /// 签名禁止使用
    ///
    /// 原因：签名禁止使用。
    ///
    /// 解决方案：请在短信服务控制台申请符合规定的签名。更多操作，请参见申请短信签名。
    SMSSignIllegal,

    /// RAM权限不足
    ///
    /// 原因：RAM权限不足。
    ///
    /// 解决方案：请为当前使用的AccessKey对应的RAM用户进行授权：AliyunDysmsFullAccess（管理权限）。
    /// 具体操作请参见授权RAM用户。
    RAMPermissionDeny,

    /// 业务停机
    ///
    /// 原因：余额不足导致的业务停机，更多详情请参见欠费说明。
    ///
    /// 解决方案：请及时充值。
    OutOfService,

    /// 未开通云通信产品的阿里云客户
    ///
    /// 原因：该AccessKey所属的账号尚未开通云通信的服务，包括短信、语音、流量等服务。
    ///
    /// 解决方案：当出现此类报错信息时，需要检查当前AccessKey是否已经开通阿里云短信服务。
    /// 如果已开通短信服务，请调用接口即可。
    ProductUnSubscribe,

    /// 产品未开通
    ///
    /// 原因：该AccessKey所属的账号尚未开通当前接口的产品，如仅开通了短信服务的用户调用语音服务接口时会产生此报错信息。
    ///
    /// 解决方案：检查AccessKey对应账号是否已开通调用对应接口的服务。如需开通服务，请参见短信服务或语音服务。
    ProductUnsubscribed,

    /// 账户不存在
    ///
    /// 原因：使用了错误的账户名称或AccessKey。
    ///
    /// 解决方案：请确认账号信息。
    AccountNotExists,

    /// 账户异常
    ///
    /// 原因：账户异常。
    ///
    /// 解决方案：计费服务查询异常，请点击链接，打开钉钉扫码进入短信服务专属钉群。
    AccountAbnormal,

    /// 模板非法
    ///
    /// 原因：
    /// - 传入的模板Code有误。
    /// - 传入的模板内容或格式有误。
    ///
    /// 解决方案：
    /// - 请检查AccessKey账号和模板是否属于同一个账号。
    /// - 若账号模板属于同一账号，请登录短信服务控制台，在模板管理页面查看此模板是否审核通过，
    ///   若未通过审核，建议模板审核通过后再使用此模板。
    /// - 请检查模板传参格式。更多模板规范请参见短信模板规范。
    /// - 确保TemplateCode不含有空格。
    /// - 确保传入的TemplateCode对应的模板变量与TemplateParam中的变量一致。
    /// - 确保传入的TemplateParam为JSON字符串。
    /// - 确保TemplateParam传入的变量值个数和模板实际的变量个数相符。
    /// - 确保TemplateParam传入的变量名称与模板对应变量的名称相符。
    SMSTemplateIllegal,

    /// 签名非法
    ///
    /// 原因：
    /// - 在您的账号下找不到对应编号的签名，可能是AccessKey账号和签名归属于不同账号，或使用了未审核通过的签名。
    /// - 您传入的签名有空格、问号、错别字等导致乱码。
    ///
    /// 解决方案：
    /// - 请检查您的AccessKey和签名是否属于同一个账号，或登录短信服务控制台，在签名管理页面查看此签名是否审核通过。
    /// - 请检查您传入的签名格式是否正确，删除签名中的空格、特殊符号，修改错别字。签名规范请参见短信签名规范。
    SMSSignatureIllegal,

    /// 参数格式不正确
    ///
    /// 原因：参数格式不正确。
    ///
    /// 解决方案：请根据对应的API文档检查参数格式。
    ///
    /// 如，短信查询QuerySendDetails接口的参数SendDate日期格式为yyyyMMdd，正确格式为20170101，错误格式为2017-01-01。
    InvalidParameters,

    /// 系统错误，请重新调用
    ///
    /// 原因：系统出现错误。
    ///
    /// 解决方案：请重新调用接口。
    SystemError,

    /// 手机号码格式错误
    ///
    /// 原因：手机号码格式错误。
    ///
    /// 解决方案：参数PhoneNumbers请传入正确的格式。
    ///
    /// - 国内消息：+/+86/0086/86或无任何前缀的11位手机号码，如1595195****。
    /// - 国际/港澳台消息：国际区号+号码，如8520000****。
    MobileNumberIllegal,

    /// 调用接口时参数PhoneNumbers中指定的手机号码数量超出限制
    ///
    /// 原因：参数PhoneNumbers中指定的手机号码数量超出限制。
    ///
    /// 解决方案：
    /// - 请将手机号码数量限制在规定范围以内。
    /// - 各接口参数PhoneNumbers数量限制如下：
    ///     - SendSms接口、SendCardSms接口：上限为1000个手机号码；
    ///     - SendBatchSms接口、SendBatchCardSms接口：最多可以向100个手机号码发送短信。
    MobileCountOverLimit,

    /// 模板变量中存在未赋值变量
    ///
    /// 原因：参数TemplateParam中，变量未全部赋值。
    ///
    /// 解决方案：请用JSON格式字符串为模板变量赋值。如：模板为您好，${name}已成功预约，预约号为${code}，
    /// 则参数TemplateParam可以指定为{"name":"Tom","code":"123"}。
    TemplateMissingParameters,

    /// 触发云通信流控限制
    ///
    /// 原因：达到云通信短信发送频率上限。
    ///
    /// 说明
    /// 发送频率上限针对手机号码的维度而言，并不是针对平台而言，即该手机号码可能收到了多个平台的验证码短信，
    /// 导致触发流控限制。即使您未在阿里云短信服务有发送记录或只发送了一条短信，也有可能达到发送频率上限。
    ///
    /// 解决方案：
    /// - 请您将短信发送频率限制在正常的业务流控范围内，默认流控详情，请参见短信发送频率限制。
    /// - 您登录短信服务控制台，在通用设置 > 国内消息设置 > 发送频率设置页面，调整流控阈值。具体操作，请参见设置短信发送频率。
    /// - 您可开启验证码防盗刷保障您的资金安全和业务稳定，帮助您有效预防验证码被盗刷或短信轰炸带来的不良影响。
    BusinessLimitControl,

    /// 参数格式错误，请修改为字符串值
    ///
    /// 原因：参数格式错误，不是合法的JSON格式，修改为字符串值。
    ///
    /// 解决方案：请在参数TemplateParam中指定正确的JSON格式字符串，比如{"code":"123"}。
    InvalidJsonParam,

    /// 黑名单管控
    ///
    /// 原因：黑名单管控是指短信号码命中黑名单，此类号码曾有过退订或投诉记录（如用户在12321等平台提交过短信投诉或退订过短信导致的），不支持下发该类推广短信。
    /// 命中黑名单的号码暂无法解除。
    ///
    /// 解决方案：推广短信建议规避该号码下发。
    BlackKeyControlLimit,

    /// 参数超过长度限制
    ///
    /// 原因：参数超过长度限制。
    ///
    /// 解决方案：针对2018年01月10日之后申请的短信通知类模板，变量限制为1~35个字符；验证码类模板，变量限制为4～6个字符，请修改参数长度。
    /// 短信模板规范详情，请参见短信模板规范。
    ParamLengthLimit,

    /// 变量不支持传入URL
    ///
    /// 原因：变量内容中含有限制发送的内容，例如变量中不允许透传URL。
    ///
    /// 解决方案：请检查通过变量是否透传了URL或敏感信息。短信模板内容规范，请参见短信模板规范。
    ParamNotSupportUrl,

    /// 账户余额不足
    ///
    /// 原因：当前账户余额不足。
    ///
    /// 解决方案：
    /// - 及时充值。发送短信前可以按照国内短信服务定价、国际/港澳台短信服务定价确认当前账户余额（含套餐包的余量，如果已购买套餐包）是否足以抵扣预计发送的短信量，否则会导致短信发送失败。
    /// - 已购买的套餐包须与短信接收地区一致。
    /// - 错误示例：使用已购买的新加坡套餐包往泰国发送短信，会发送失败。建议购买全球通用套餐包或对应地区的套餐包。
    /// - 计费以及套餐包购买详情，请参见计费概述。
    AmountNotEnough,

    /// 传入的变量内容和实际申请模板时变量所选择的属性类型不配
    ///
    /// 原因：例如申请模板时对phone变量，选择变量属性为"电话号码"，但实际入参时对这个变量赋值非号码类型的内容。
    ///
    /// 解决方案：针对上述原因，phone变量入参应为：5~11位的国内标准手机号或固定电话号码，更多变量属性规范，请参见验证码模板变量属性规范或通知短信模板变量属性规范。
    TemplateParamsIllegal,

    /// 客户端生成的签名与服务端不匹配
    ///
    /// 原因：签名（Signature）加密错误。
    ///
    /// 解决方案：
    /// - 如果使用SDK调用接口，请注意AccessKey ID和AccessKey Secret字符串赋值正确。
    /// - 如果自行加密签名（Signature），请参见请求签名检查加密逻辑。
    SignatureDoesNotMatch,

    /// 时间戳或日期已过期
    ///
    /// 原因：一般由于时区差异造成时间戳错误，发出请求的时间和服务器接收到请求的时间不在15分钟内。
    ///
    /// 解决方案：请使用GMT时间。
    ///
    /// 说明
    /// 阿里云网关使用的时间是GMT时间。
    InvalidTimeStampExpired,

    /// 签名随机数已被使用
    ///
    /// 原因：唯一随机数重复，SignatureNonce为唯一随机数，用于防止网络重复攻击。
    ///
    /// 解决方案：不同请求请使用不同的随机数值。
    SignatureNonceUsed,

    /// API版本号错误
    ///
    /// 原因：版本号（Version）错误。
    ///
    /// 解决方案：请确认接口的版本号，短信服务的API版本号（Version）为2017-05-25。
    InvalidVersion,

    /// 未找到指定的API，请检查您的URL和方法
    ///
    /// 原因：参数Action中指定的接口名错误。
    ///
    /// 解决方案：请在参数Action中使用正确的接口地址和接口名。
    InvalidActionNotFound,

    /// 超过单自然日签名申请数量上限
    ///
    /// 原因：一个自然日中申请签名数量超过限制。
    ///
    /// 解决方案：合理安排每天的签名申请数量，次日重新申请。更多信息，请参见个人用户和企业用户权益区别。
    SignCountOverLimit,

    /// 超过单自然日模板申请数量上限
    ///
    /// 原因：一个自然日中申请模板数量超过限制。
    ///
    /// 解决方案：合理安排每天的模板申请数量，次日重新申请。更多信息，请参见个人用户和企业用户权益区别。
    TemplateCountOverLimit,

    /// 签名名称不符合规范
    ///
    /// 原因：签名名称不符合规范。
    ///
    /// 解决方案：请重新申请签名。签名规范，请参见短信签名规范。
    SignNameIllegal,

    /// 签名认证材料附件大小超过限制
    ///
    /// 原因：签名认证材料附件大小超过限制。
    ///
    /// 解决方案：压缩签名认证材料至2 MB以下。
    SignFileLimit,

    /// 签名字符数量超过限制
    ///
    /// 原因：签名的名称或申请说明的字数超过限制。
    ///
    /// 解决方案：修改签名名称或申请说明，并重新提交审核。签名规范，请参见短信签名规范。
    SignOverLimit,

    /// 模板字符数量超过限制
    ///
    /// 原因：模板的名称、内容或申请说明的字数超过限制。
    ///
    /// 解决方案：修改模板的名称、内容或申请说明，并重新提交审核。
    TemplateOverLimit,

    /// 签名内容涉及违规信息
    ///
    /// 原因：签名内容涉及违规信息。
    ///
    /// 解决方案：重新修改签名内容。签名规范，请参见短信签名规范。
    SignatureBlacklist,

    /// 超过单自然日短链申请数量上限
    ///
    /// 原因：一天创建短链数量超过限制。
    ///
    /// 解决方案：合理预估一天申请短链数量，次日重新创建短链。
    ShorturlOverLimit,

    /// 该账号无有效短链
    ///
    /// 原因：企业客户当前并无有效短链。
    ///
    /// 解决方案：企业客户需重新申请有效短链，保证在短链有效期内调用短链生成接口。
    NoAvailableShortUrl,

    /// 短链名不能超过13字符
    ///
    /// 原因：短链名不能超过13个字符。
    ///
    /// 解决方案：请根据短链规范重新创建。
    ShorturlNameIllegal,

    /// 原始链接字符数量超过限制
    ///
    /// 原因：原始链接字符数量超过限制。
    ///
    /// 解决方案：重新创建判断字符长度是否符合平台规则。
    SourceurlOverLimit,

    /// 短链有效期期限超过限制
    ///
    /// 原因：选择短链有效期限超过30天限制。
    ///
    /// 解决方案：请保证短链有效期输入在30天以内。
    ShorturlTimeIllegal,

    /// 上传手机号个数超过上限
    ///
    /// 原因：单次调用上传手机号个数超过50000个上限。
    ///
    /// 解决方案：分多次调用短参生成接口，单次上传手机号个数不超过50000个。
    PhonenumbersOverLimit,

    /// 原始链接生成的短链仍在有效期内
    ///
    /// 原因：原始链接生成的短链仍在有效期内，无需重复创建。
    ///
    /// 解决方案：
    /// - 待原始链接生成的短链失效或删除该短链后，重新创建。
    /// - 使用新的原始链接，创建新的短链。
    ShorturlStillAvailable,

    /// 签名文件为空
    ///
    /// 原因：签名文件为空。
    ///
    /// 解决方案：检查签名文件，补充签名资质证明、授权书等相关文件截图。相关操作，请参见申请短信签名。
    ErrorEmptyFile,

    /// 调用发送应用模块失败
    ///
    /// 原因：调用发送应用模块失败。
    ///
    /// 解决方案：调用发送应用模块失败，请尝试重新发送。
    GatewayError,

    /// 审核中的签名，暂时无法删除
    ///
    /// 原因：签名正在审核中，暂时无法删除。
    ///
    /// 解决方案：请签名审核结束后，再删除对应签名。
    ErrorSignNotDelete,

    /// 已通过的签名不支持修改
    ///
    /// 原因：已通过的签名不支持修改。
    ///
    /// 解决方案：请按签名规范，重新提交申请签名。具体操作，请参见申请短信签名。
    ErrorSignNotModify,

    /// 审核中的模板，暂时无法删除
    ///
    /// 原因：模板正在审核中，暂时无法删除。
    ///
    /// 解决方案：请模板审核结束后，再删除对应模板。
    ErrorTemplateNotDelete,

    /// 已通过的模板不支持修改
    ///
    /// 原因：已通过的模板不支持修改。
    ///
    /// 解决方案：请按模板规范，重新提交申请模板。更多信息，请参见短信模板规范。
    ErrorTemplateNotModify,

    /// 单日最多申请模板或签名100条
    ///
    /// 原因：您已超过单日最多申请模板或签名100条的上限。
    ///
    /// 解决方案：请您24小时后继续申请，或单击链接，打开钉钉扫码进入短信服务专属钉群处理。
    SMSSOverLimit,

    /// 用户已退订推广短信
    ///
    /// 原因：该手机用户已退订推广短信。
    ///
    /// 解决方案：请尊重手机用户意愿，减少对该客户的推广短信发送。
    CustomerRefused,

    /// 测试模板和签名限制
    ///
    /// 原因：测试专用签名和模板必须结合使用。
    ///
    /// 解决方案：请使用短信服务提供的测试专用签名和测试专用模板。相关操作，请参见发送测试短信。
    SMSTestSignTemplateLimit,

    /// 短链创建失败
    ///
    /// 解决方案：请先提交该链接的一级域名报备。
    ShorturlDomainEmpty,

    /// 验证码模板仅支持一个验证码作为变量
    ///
    /// 原因：在验证码模板变量中只能定义一个参数。
    ///
    /// 解决方案：请修改验证码模板变量。更多信息，请参见验证码模板规范。
    TemplateParameterCountIllegal,

    /// 测试专用模板中的变量仅支持4~6位纯数字
    ///
    /// 解决方案：使用测试模板时变量仅支持传入4~6位纯数字。
    SMSTestTemplateParamsIllegal,

    /// 只能向已绑定的手机号发送
    ///
    /// 原因：接收测试短信号码未在控制台绑定，或绑定过程未完成、未生效。
    ///
    /// 解决方案：
    /// - 通过测试功能发送短信，需要绑定测试手机号才可以发送。
    /// - 请确认绑定操作已完成并且已生效。有时系统处理或同步可能会有延时。
    ///
    /// 说明
    /// 您登录短信服务控制台，在快速学习和测试页面，发送测试区域，绑定测试手机号即可。
    /// 更多有关测试短信内容，请参见发送测试短信。
    ///
    /// 原因：接口调用与绑定手机号不匹配。
    ///
    /// 解决方案：请检查在API请求或控制台调用中指定的手机号是否与在控制台绑定的测试手机号完全一致，包括国家码和格式。
    ///
    /// 原因：如您在生产环境遇到此错误码，请排查您是否将测试环境与生产环境混淆。
    ///
    /// 解决方案：测试签名和模板（在控制台显示有绿色“测”标记）会限制发送对象仅为已绑定的测试号码。请确保在正式发送短信时使用的是非测试的签名和模板。
    SMSTestNumberLimit,

    /// 签名不能包含emoji表情
    ///
    /// 解决方案：签名中不支持使用emoji表情。
    SMSSignEmojiIllegal,

    /// 因该账号长时间未使用，出于对您的账号安全考虑，已限制您账号的短信发送
    ///
    /// 您若需要继续使用该账号，请与您的商务经理联系。
    SecurityFrozenAccount,

    /// 短信下发时通道被关停
    ///
    /// 阿里云会自动剔除被关停通道，建议稍后重试。
    IsClose,

    /// 参数错误
    ///
    /// 请检查短信签名、短信文案或手机号码等参数是否传入正确。
    ParamsIllegal,

    /// 停机、空号、暂停服务、关机、不在服务区
    ///
    /// 原因：运营商返回给阿里云平台该号码实时状态异常，如关机、停机、空号、暂停服务、不在服务区等。
    ///
    /// 解决方案：请核实接收手机号码状态是否正常。
    MobileNotOnService,

    /// 单个号码日、月发送上限，流控超限，频繁发送超限
    ///
    /// 为了限制平台短信被恶意调用、在短时间内大量发起短信发送请求，阿里云平台和运营商都进行了流控限制。
    MobileSendLimit,

    /// 用户账户异常、携号转网、欠费等
    ///
    /// 建议检查号码状态，确保号码状态正常后重新尝试。
    MobileAccountAbnormal,

    /// 手机号在黑名单
    ///
    /// 通常为手机号在运营商平台黑名单库中，一般是用户已退订此签名或命中运营商平台黑名单规则导致。
    MobileInBlack,

    /// 手机终端问题、内存满、SIM卡满、非法设备等
    ///
    /// 建议核实终端设备状况、检查手机安全软件拦截记录、重启或者更换手机终端后再次尝试。
    MoblleTerminalError,

    /// 内容关键字拦截
    ///
    /// 原因：运营商根据短信中有潜在风险或者高投诉的内容关键字进行自动拦截。
    ///
    /// 解决方案：请您检查发送的短信内容并相应修改文案。
    ContentKeyword,

    /// 号码状态异常
    ///
    /// 原因：短信接收号码状态异常，如关机、停机、空号、暂停服务、不在服务区或号码格式错误。
    ///
    /// 解决方案：请您核实号码状态是否正常、号码格式是否正确。
    InvalidNumber,

    /// 内容无退订
    ///
    /// 原因：推广短信内容中必须带退订信息。
    ///
    /// 解决方案：请您核实文案内容，增加退订信息，在推广短信结尾添加“拒收请回复R”。更多详情请参见推广短信模板规范。
    ContentError,

    /// 请求成功（平台接到请求，但未接收到运营商侧的回执状态）
    ///
    /// 原因：大概率是由于接收用户的状态异常导致。
    ///
    /// 解决方案：请您检查接收用户状态是否正常。
    RequestSuccess,

    /// 未开通国际短信
    ///
    /// 原因：收件人未开通接收国际短信功能。
    ///
    /// 解决方案：联系运营商开通国际短信功能后再进行短信发送。
    SPNotByInterSMS,

    /// 运营商未知错误
    ///
    /// 该错误码表示阿里云平台接收到的运营商回执报告为未知错误，阿里云会定期与供应商核实更新。
    SPUnknownError,

    /// 接收用户已退订此业务或产品未开通
    ///
    /// 建议将此类接收用户剔除出发送清单。
    UserReject,

    /// 当前短信内容无可用通道发送
    ///
    /// 发送的业务场景属于暂时无法支持的场景。
    NORoute,

    /// 不支持的短信内容
    ///
    /// 原因：短信内容包含不支持的发送内容，如短信内容中包含繁体字、emoji表情符号、 其他非常用字符（例如【】〖〗『』「」m² • ①★※→ ❤™）。
    ///
    /// 解决方案：请检查短信模板内容以及您传入的变量内容，修改短信文案并去掉不支持的文字或符号。更详细的短信模板及变量规范，请参见短信模板规范。
    UnsupportedContent,

    /// 短信内容和模板属性不匹配
    ///
    /// 原因：通知模板发送推广、营销的文案无法支持。
    ///
    /// 解决方案：请使用推广短信模板发送短信，模板末尾需加“拒收请回复R”。
    SMSContentMismatchTemplateType,

    /// 一码多签
    ///
    /// 原因：当前传入的扩展码和签名，与历史传入扩展码对应的签名不一致。
    ///
    /// 解决方案：建议您确保扩展码对应的签名在发送过程中保持不变，或更新扩展码签名映射关系中该扩展码对应的新签名，具体操作请参见扩展码管理。
    OneCodeMultipleSign,

    /// 自拓扩展码个数已超过上限
    ///
    /// 原因：自拓扩展码数量上限为10n，n即自拓扩展位数，每个用户的自拓扩展位数不同，超过后无法分配新的扩展码发送新签名。
    ///
    /// 解决方案：建议您联系产品运营经理修改自拓扩展码位数，或删除扩展码签名映射关系中可回收的扩展码和签名，具体操作请参见扩展码管理。
    CodeExceedLimit,

    /// 传入扩展码不可用
    ///
    /// 原因：传入的自拓扩展位数超限，不可直接作为下发扩展码使用。
    ///
    /// 解决方案：建议传入标准位长的扩展码。
    CodeError,

    /// 当前使用端口号尚未完成企业实名制报备流程
    ///
    /// 原因：短信内容提供者需要报备相关实名资质信息，若部分运营商网关尚未完成报备，可能导致短信发送被拦截。
    ///
    /// 解决方案：短信发送前必须完成实名制报备。建议您根据实名制报备操作指南进行操作，您可以登录短信服务控制台签名管理页面查看签名报备状态。
    /// 运营商实名报备流程平均需要5-7个工作日，基于近期观测，部分运营商实名报备流程需要7-10个工作日，
    PortNotRegistered,

    /// 签名来源不支持
    ///
    /// 原因：创建和修改签名时使用了不支持的签名来源。
    ///
    /// 解决方案：请选择符合规范的签名来源。
    SignSourceIllegal,

    /// 未记录字段
    Unknown(String),
}

impl fmt::Display for RespCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Into<RespCode> for String {
    fn into(self) -> RespCode {
        let s = self.as_str();
        s.into()
    }
}

impl Into<RespCode> for &str {
    fn into(self) -> RespCode {
        match self {
            "OK" => RespCode::Ok,
            "isv.SMS_SIGNATURE_SCENE_ILLEGAL" => RespCode::SMSSignatureSceneIllegal,
            "isv.EXTEND_CODE_ERROR" => RespCode::ExtendCodeError,
            "isv.DOMESTIC_NUMBER_NOT_SUPPORTED" => RespCode::DomesticNumberNotSupported,
            "isv.DENY_IP_RANGE" => RespCode::DenyIpRange,
            "isv.DAY_LIMIT_CONTROL" => RespCode::DayLimitControl,
            "isv.MONTH_LIMIT_CONTROL" => RespCode::MonthLimitControl,
            "isv.SMS_CONTENT_ILLEGAL" => RespCode::SMSContentIllegal,
            "isv.SMS_SIGN_ILLEGAL" => RespCode::SMSSignIllegal,
            "isp.RAM_PERMISSION_DENY" => RespCode::RAMPermissionDeny,
            "isv.OUT_OF_SERVICE" => RespCode::OutOfService,
            "isv.PRODUCT_UN_SUBSCRIPT" | "isv.PRODUCT_UNSUBSCRIBE" => RespCode::ProductUnSubscribe,
            "isv.ACCOUNT_NOT_EXISTS" => RespCode::AccountNotExists,
            "isv.ACCOUNT_ABNORMAL" => RespCode::AccountAbnormal,
            "isv.SMS_TEMPLATE_ILLEGAL" => RespCode::SMSTemplateIllegal,
            "isv.SMS_SIGNATURE_ILLEGAL" => RespCode::SMSSignatureIllegal,
            "isv.INVALID_PARAMETERS" => RespCode::InvalidParameters,
            "isp.SYSTEM_ERROR" => RespCode::SystemError,
            "isv.MOBILE_NUMBER_ILLEGAL" => RespCode::MobileNumberIllegal,
            "isv.MOBILE_COUNT_OVER_LIMIT" => RespCode::MobileCountOverLimit,
            "isv.TEMPLATE_MISSING_PARAMETERS" => RespCode::TemplateMissingParameters,
            "isv.BUSINESS_LIMIT_CONTROL" => RespCode::BusinessLimitControl,
            "isv.INVALID_JSON_PARAM" => RespCode::InvalidJsonParam,
            "isv.BLACK_KEY_CONTROL_LIMIT" => RespCode::BlackKeyControlLimit,
            "isv.PARAM_LENGTH_LIMIT" => RespCode::ParamLengthLimit,
            "isv.PARAM_NOT_SUPPORT_URL" => RespCode::ParamNotSupportUrl,
            "isv.AMOUNT_NOT_ENOUGH" => RespCode::AmountNotEnough,
            "isv.TEMPLATE_PARAMS_ILLEGAL" => RespCode::TemplateParamsIllegal,
            "SignatureDoesNotMatch" => RespCode::SignatureDoesNotMatch,
            "InvalidTimeStamp.Expired" => RespCode::InvalidTimeStampExpired,
            "SignatureNonceUsed" => RespCode::SignatureNonceUsed,
            "InvalidVersion" => RespCode::InvalidVersion,
            "InvalidAction.NotFound" => RespCode::InvalidActionNotFound,
            "isv.SIGN_COUNT_OVER_LIMIT" => RespCode::SignCountOverLimit,
            "isv.TEMPLATE_COUNT_OVER_LIMIT" => RespCode::TemplateCountOverLimit,
            "isv.SIGN_NAME_ILLEGAL" => RespCode::SignNameIllegal,
            "isv.SIGN_FILE_LIMIT" => RespCode::SignFileLimit,
            "isv.SIGN_OVER_LIMIT" => RespCode::SignOverLimit,
            "isv.TEMPLATE_OVER_LIMIT" => RespCode::TemplateOverLimit,
            "SIGNATURE_BLACKLIST" => RespCode::SignatureBlacklist,
            "isv.SHORTURL_OVER_LIMIT" => RespCode::ShorturlOverLimit,
            "isv.NO_AVAILABLE_SHORT_URL" => RespCode::NoAvailableShortUrl,
            "isv.SHORTURL_NAME_ILLEGAL" => RespCode::ShorturlNameIllegal,
            "isv.SOURCEURL_OVER_LIMIT" => RespCode::SourceurlOverLimit,
            "isv.SHORTURL_TIME_ILLEGAL" => RespCode::ShorturlTimeIllegal,
            "isv.PHONENUMBERS_OVER_LIMIT" => RespCode::PhonenumbersOverLimit,
            "isv.SHORTURL_STILL_AVAILABLE" => RespCode::ShorturlStillAvailable,
            "isv.ERROR_EMPTY_FILE" => RespCode::ErrorEmptyFile,
            "isp.GATEWAY_ERROR" => RespCode::GatewayError,
            "isv.ERROR_SIGN_NOT_DELETE" => RespCode::ErrorSignNotDelete,
            "isv.ERROR_SIGN_NOT_MODIFY" => RespCode::ErrorSignNotModify,
            "isv.ERROR_TEMPLATE_NOT_DELETE" => RespCode::ErrorTemplateNotDelete,
            "isv.ERROR_TEMPLATE_NOT_MODIFY" => RespCode::ErrorTemplateNotModify,
            "isv.SMS_OVER_LIMIT" => RespCode::SMSSOverLimit,
            "isv.CUSTOMER_REFUSED" => RespCode::CustomerRefused,
            "isv.SMS_TEST_SIGN_TEMPLATE_LIMIT" => RespCode::SMSTestSignTemplateLimit,
            "isv.SHORTURL_DOMAIN_EMPTY" => RespCode::ShorturlDomainEmpty,
            "template_parameter_count_illegal" => RespCode::TemplateParameterCountIllegal,
            "isv.SMS_TEST_TEMPLATE_PARAMS_ILLEGAL" => RespCode::SMSTestTemplateParamsIllegal,
            "isv.SMS_TEST_NUMBER_LIMIT" => RespCode::SMSTestNumberLimit,
            "isv.SMS_SIGN_EMOJI_ILLEGAL" => RespCode::SMSSignEmojiIllegal,
            "isv.SECURITY_FROZEN_ACCOUNT" => RespCode::SecurityFrozenAccount,
            "IS_CLOSE" => RespCode::IsClose,
            "PARAMS_ILLEGAL" => RespCode::ParamsIllegal,
            "MOBILE_NOT_ON_SERVICE" => RespCode::MobileNotOnService,
            "MOBILE_SEND_LIMIT" => RespCode::MobileSendLimit,
            "MOBILE_ACCOUNT_ABNORMAL" => RespCode::MobileAccountAbnormal,
            "MOBILE_IN_BLACK" => RespCode::MobileInBlack,
            "MOBLLE_TERMINAL_ERROR" => RespCode::MoblleTerminalError,
            "CONTENT_KEYWORD" => RespCode::ContentKeyword,
            "INVALID_NUMBER" => RespCode::InvalidNumber,
            "CONTENT_ERROR" => RespCode::ContentError,
            "REQUEST_SUCCESS" => RespCode::RequestSuccess,
            "SP_NOT_BY_INTER_SMS" => RespCode::SPNotByInterSMS,
            "SP_UNKNOWN_ERROR" => RespCode::SPUnknownError,
            "USER_REJECT" => RespCode::UserReject,
            "NO_ROUTE" => RespCode::NORoute,
            "isv.UNSUPPORTED_CONTENT" => RespCode::UnsupportedContent,
            "isv.SMS_CONTENT_MISMATCH_TEMPLATE_TYPE" => RespCode::SMSContentMismatchTemplateType,
            "isv.ONE_CODE_MULTIPLE_SIGN" => RespCode::OneCodeMultipleSign,
            "isv.CODE_EXCEED_LIMIT" => RespCode::CodeExceedLimit,
            "isv.CODE_ERROR" => RespCode::CodeError,
            "PORT_NOT_REGISTERED" => RespCode::PortNotRegistered,
            "isv.SIGN_SOURCE_ILLEGAL" => RespCode::SignSourceIllegal,
            _ => RespCode::Unknown(self.to_string()),
        }
    }
}
