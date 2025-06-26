pub mod delete_respnose {
    #[derive(serde::Deserialize, Debug, Clone)]
    #[serde(rename_all = "PascalCase")]
    pub struct DeleteSmsTemplateRespnose {
        /// 已删除的模板 Code
        pub template_code: String,
    }
}

pub mod create_respnose {
    #[derive(serde::Deserialize, Debug, Clone)]
    #[serde(rename_all = "PascalCase")]
    pub struct CreateSmsTemplateRespnose {
        /// 短信模板名称
        pub template_name: String,

        /// 短信模板 Code
        pub template_code: String,

        /// 工单号。
        /// 审核人员查询审核时会用到此参数。您需要审核加急时需要提供此工单号。
        pub order_id: String,
    }
}

pub mod update_respnose {
    #[derive(serde::Deserialize, Debug, Clone)]
    #[serde(rename_all = "PascalCase")]
    pub struct UpdateSmsTemplateRespnose {
        /// 短信模板名称
        pub template_name: String,

        /// 短信模板 Code
        pub template_code: String,

        /// 工单号。
        /// 审核人员查询审核时会用到此参数。您需要审核加急时需要提供此工单号。
        pub order_id: String,
    }
}

pub mod query_respnose {
    use serde::Deserialize;

    #[derive(Deserialize, Debug, Clone)]
    #[serde(rename_all = "PascalCase")]
    pub struct QuerySmsTemplateRespnose {
        pub sms_template_list: Vec<SmsTemplateList>,
        /// 本次查询到的模板总数
        pub total_count: u64,
        /// 当前页码。默认取值为 1
        pub current_page: u16,
        /// 每页显示的模板个数。取值范围：1~50
        pub page_size: u16,
    }

    #[derive(Deserialize, Debug, Clone)]
    #[serde(rename_all = "PascalCase")]
    pub struct SmsTemplateList {
        /// 短信模板 Code
        pub template_code: String,
        /// 短信模板名称
        pub template_name: String,
        /// 模板类型（对外使用）0：验证码短信
        /// 1：通知短信
        /// 2：推广短信
        /// 3：国际/港澳台短信
        pub outer_template_type: i32,
        /// 模板审核状态。返回值
        #[serde(deserialize_with = "deserialize_audit_status")]
        pub audit_status: AuditStatus,
        /// 模板内容
        pub template_content: String,
        /// 创建模板的时间，格式为 yyyy-MM-dd HH:mm:ss
        pub create_date: String,
        /// 审核返回值
        pub reason: Option<Reason>,
        /// 工单号
        pub order_id: String,
        /// 模板类型
        pub template_type: i32,
    }

    #[derive(Deserialize, Debug, Clone)]
    pub enum AuditStatus {
        /// 审核中
        AuditStateInit,
        /// 通过审核
        AuditStatePass,
        /// 未通过审核，请在返回参数 Reason 中查看审核未通过原因
        AuditStateNotPass,
        /// AUDIT_SATE_CANCEL
        AuditSateCancel,
    }

    impl Into<AuditStatus> for String {
        fn into(self) -> AuditStatus {
            match self.as_str() {
                "AUDIT_STATE_INIT" => AuditStatus::AuditStateInit,
                "AUDIT_STATE_PASS" => AuditStatus::AuditStatePass,
                "AUDIT_STATE_NOT_PASS" => AuditStatus::AuditStateNotPass,
                "AUDIT_SATE_CANCEL" => AuditStatus::AuditSateCancel,
                _ => AuditStatus::AuditStateNotPass,
            }
        }
    }

    fn deserialize_audit_status<'de, D>(deserializer: D) -> Result<AuditStatus, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let code: String = String::deserialize(deserializer)?;
        if code.is_empty() {
            Err(serde::de::Error::custom("audit_status is empty!"))
        } else {
            Ok(code.into())
        }
    }

    #[derive(Deserialize, Debug, Clone)]
    #[serde(rename_all = "PascalCase")]
    pub struct Reason {
        /// 审核未通过的时间，格式为 yyyy-MM-dd HH:mm:ss
        pub reject_date: String,
        /// 审核未通过的原因
        pub reject_info: String,
        /// 审核未通过的详细说明
        pub reject_sub_info: String,
    }
}
