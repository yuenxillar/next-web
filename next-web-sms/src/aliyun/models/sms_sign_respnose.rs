pub mod create_respnose {
    #[derive(serde::Deserialize, Debug, Clone)]
    #[serde(rename_all = "PascalCase")]
    pub struct CreateSmsSignRespnose {
        /// 短信签名名称
        pub sign_name: String,

        /// 工单号。
        pub order_id: String,
    }
}

pub mod delete_respnose {
    #[derive(serde::Deserialize, Debug, Clone)]
    #[serde(rename_all = "PascalCase")]
    pub struct DeleteSmsSignRespnose {
        /// 已删除的签名名称
        pub sign_name: String,
    }
}

pub mod update_respnose {
    #[derive(serde::Deserialize, Debug, Clone)]
    #[serde(rename_all = "PascalCase")]
    pub struct UpdateSmsSignRespnose {
        /// 短信签名名称
        pub sign_name: String,

        /// 工单号。
        pub order_id: String,
    }
}

pub mod query_respnose {
    use serde::Deserialize;

    #[derive(Deserialize, Debug, Clone)]
    #[serde(rename_all = "PascalCase")]
    pub struct QuerySmsSignRespnose {
        /// 短信签名名称
        pub sign_name: String,
        /// 签名审核状态。取值：
        /// 0：审核中。
        /// 1：审核通过。
        /// 2：审核失败，请在返回参数AuditInfo.RejectInfo中查看审核失败原因
        /// 10：取消审核。
        pub sign_status: i64,
        /// 短信签名的创建日期和时间
        pub create_date: String,
        /// 工单号
        pub order_id: String,
        /// 资质 ID，申请签名时关联的资质 ID
        pub qualification_id: i64,
        /// 短信签名场景说明，长度不超过 200 个字符
        pub remark: String,
        /// 审核信息
        pub audit_info: AuditInfo,
        /// 更多资料信息，补充上传业务证明文件或业务截图文件列表
        pub file_url_list: Vec<String>,
        /// 短信签名 Code
        pub sign_code: String,
        /// 签名标识
        pub sign_tag: String,
        /// 应用场景内容
        pub apply_scene: String,
        /// 签名为自用或他用
        pub third_party: bool,
        /// 签名使用场景
        pub sign_usage: String,
        /// 签名实名制报备结果。取值：
        /// 0：报备失败，原因可能为资质信息与工信注册信息不一致或运营商侧无法支持等。建议您登录短信服务控制台查看具体失败原因，并依据提示进行操作。
        /// 1：报备成功，当前至少有一个子端口号运营商已返回报备通过。建议您少量多次尝试使用该签名发送，观察短信发送效果后再开始批量发送。
        /// 2：报备失效，签名超过 6 个月无发送记录时，报备结果失效。如您需要重新启用该签名，请在短信服务控制台重新发起报备。
        /// -1：无状态，此结果表明您的签名处于报备流程中或未报备，建议您登录短信服务控制台查看报备详情，如已处于报备流程中请耐心等待；如未报备请检查签名绑定的资质是否需要更新，检查无误后重新发起报备。
        pub register_result: i32,
        /// 委托授权书 ID
        pub authorization_letter_id: i64,
        /// 委托授权书审核状态
        /// 取值：
        /// true：审核通过。
        /// false：审核未通过（包含审核通过外的其他所有状态）。
        pub authorization_letter_audit_pass: bool,
        pub sign_isp_register_detail_list: Vec<SignIspRegisterDetail>,
    }

    #[derive(Deserialize, Debug, Clone)]
    #[serde(rename_all = "PascalCase")]
    pub struct AuditInfo {
        /// 审批未通过的原因
        pub reject_info: String,
        /// 审核时间
        pub audit_date: String,
    }

    /// 官网暂无注释 暂不填写
    #[derive(Deserialize, Debug, Clone)]
    #[serde(rename_all = "PascalCase")]
    pub struct SignIspRegisterDetail {
        pub register_status: i32,
        pub operator_code: String,
        pub operator_complete_time: String,
        pub register_status_reasons: Vec<Reason>,
    }

    #[derive(Deserialize, Debug, Clone)]
    #[serde(rename_all = "PascalCase")]
    pub struct Reason {
        pub reason_code: i32,
        pub reason_desc_list: Vec<String>,
    }
}
