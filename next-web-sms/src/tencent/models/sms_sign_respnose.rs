pub mod create_respnose {
    #[derive(serde::Deserialize, Debug, Clone)]
    #[serde(rename_all = "PascalCase")]
    pub struct CreateSmsSignRespnose {
        /// 短信签名ID
        pub sign_id: u64,
    }
}

pub mod delete_respnose {
    #[derive(serde::Deserialize, Debug, Clone)]
    #[serde(rename_all = "PascalCase")]
    pub struct DeleteSmsSignRespnose {
        /// 删除状态信息。
        /// 示例值：return successfully!
        pub delete_status: String,
        /// 删除时间，UNIX 时间戳（单位：秒）
        pub delete_time: u64,
    }
}

pub mod update_respnose {
    #[derive(serde::Deserialize, Debug, Clone)]
    #[serde(rename_all = "PascalCase")]
    pub struct UpdateSmsSignRespnose {
        /// 短信签名ID
        pub sign_id: u64,
    }
}

pub mod query_respnose {
    use serde::Deserialize;

    #[derive(Deserialize, Debug, Clone)]
    #[serde(rename_all = "PascalCase")]
    pub struct QuerySmsSignRespnose {
        pub describe_sign_list_status: Vec<DescribeSign>,
    }

    #[derive(Deserialize, Debug, Clone)]
    #[serde(rename_all = "PascalCase")]
    pub struct DescribeSign {
        /// 签名ID
        pub sign_id: u64,
        /// 是否国际/港澳台短信，其中0表示国内短信，1表示国际/港澳台短信
        pub international: u32,
        /// 申请签名状态，其中0表示审核通过且已生效，1表示审核中，2表示审核通过待生效，-1表示审核未通过或审核失败
        pub status_code: u32,
        /// 审核回复，审核人员审核后给出的回复，通常是审核未通过的原因
        pub review_reply: String,
        /// 签名名称
        pub sign_name: String,
        /// 提交审核时间，UNIX 时间戳（单位：秒）
        pub create_time: u64,
        /// 国内短信的资质 ID。资质 ID 信息可前往国内短信的 实名资质管理 页查看
        pub qualification_id: u64,
        /// 国内短信的资质名称
        pub qualification_name: String,
        /// 国内短信的资质状态。其中0表示待审核，1表示已通过，2表示已拒绝，3表示待补充后提交，4表示变更后待审核，5表示变更后被驳回
        pub qualification_status_code: u32,
    }
}



#[derive(Clone, Debug, serde::Deserialize)]
#[serde(untagged)]
pub enum Respnose {
    AddSignStatus {
        #[serde(rename = "AddSignStatus")]
        status: create_respnose::CreateSmsSignRespnose,
        #[serde(rename = "RequestId")]
        request_id: String,
    },
    DeleteSignStatus {
        #[serde(rename = "DeleteSignStatus")]
        status: delete_respnose::DeleteSmsSignRespnose,
        #[serde(rename = "RequestId")]
        request_id: String,
    },
    ModifySignStatus {
        #[serde(rename = "ModifySignStatus")]
        status: update_respnose::UpdateSmsSignRespnose,
        #[serde(rename = "RequestId")]
        request_id: String,
    },
    DescribeSignListStatusSet {
        #[serde(rename = "DescribeSignListStatusSet")]
        status_set: query_respnose::QuerySmsSignRespnose,
        #[serde(rename = "RequestId")]
        request_id: String,
    },
}