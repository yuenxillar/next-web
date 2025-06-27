pub mod create_respnose {
    #[derive(serde::Deserialize, Debug, Clone)]
    #[serde(rename_all = "PascalCase")]
    pub struct CreateSmsTemplateRespnose {
        /// 短信模板 ID
        pub template_id: String,
    }
}

pub mod delete_respnose {
    #[derive(serde::Deserialize, Debug, Clone)]
    #[serde(rename_all = "PascalCase")]
    pub struct DeleteSmsTemplateRespnose {
        /// 删除状态信息
        /// 示例值：return successfully!
        pub delete_status: String,
        /// 删除时间，UNIX 时间戳（单位：秒）
        /// 示例值：1578988506
        pub delete_time: i64,
    }
}

pub mod update_respnose {
    #[derive(serde::Deserialize, Debug, Clone)]
    #[serde(rename_all = "PascalCase")]
    pub struct UpdateSmsTemplateRespnose {
        /// 模板ID
        pub template_id: u64,
    }
}

pub mod query_respnose {
    use serde::Deserialize;

    #[derive(Deserialize, Debug, Clone)]
    #[serde(rename_all = "PascalCase")]
    pub struct QuerySmsTemplateRespnose {
        /// 模板ID
        pub template_id: u64,
        /// 是否国际/港澳台短信，其中0表示国内短信，1表示国际/港澳台短信，3表示该模板既支持国内短信也支持国际/港澳台短信
        pub international: u16,
        /// 申请模板状态，其中0表示审核通过且已生效，1表示审核中，2表示审核通过待生效，-1表示审核未通过或审核失败。注：只有状态值为0时该模板才能使用
        pub status_code: u16,
        /// 审核回复，审核人员审核后给出的回复，通常是审核未通过的原因
        pub review_reply: String,
        /// 模板名称
        pub template_name: String,
        /// 提交审核时间，UNIX 时间戳（单位：秒）
        pub create_time: i64,
        /// 模板内容
        pub template_content: String,
    }
}

#[derive(Debug, serde::Deserialize)]
#[serde(untagged)]
pub enum Respnose {
    AddTemplateStatus {
        #[serde(rename = "AddTemplateStatus")]
        status: create_respnose::CreateSmsTemplateRespnose,
        #[serde(rename = "RequestId")]
        request_id: String,
    },
    DeleteTemplateStatus {
        #[serde(rename = "DeleteTemplateStatus")]
        status: delete_respnose::DeleteSmsTemplateRespnose,
        #[serde(rename = "RequestId")]
        request_id: String,
    },
    ModifyTemplateStatus {
        #[serde(rename = "ModifyTemplateStatus")]
        status: update_respnose::UpdateSmsTemplateRespnose,
        #[serde(rename = "RequestId")]
        request_id: String,
    },
    DescribeTemplateStatusSet {
        #[serde(rename = "DescribeTemplateStatusSet")]
        status_set: Vec<query_respnose::QuerySmsTemplateRespnose>,
        #[serde(rename = "RequestId")]
        request_id: String,
    },
}