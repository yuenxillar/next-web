use lopdf::Document;
use lopdf::encryption::crypt_filters::{Aes256CryptFilter, CryptFilter};
use lopdf::encryption::{EncryptionState, EncryptionVersion, Permissions};
use rand::Rng;
use std::collections::BTreeMap;
use std::sync::Arc;

use crate::{error::pdf_error::PdfError, operation::PdfOperation};

pub struct PdfEncryptOperation {
    pub owner_password: String,
    pub user_password: String,
}

impl PdfEncryptOperation {
    /// 创建一个新的加密操作（所有者和用户密码相同）
    pub fn new<T: Into<String>>(password: T) -> Self {
        let password = password.into();
        Self {
            owner_password: password.clone(),
            user_password: password,
        }
    }

    /// 创建带有不同权限密码的操作
    /// owner_password: 拥有完全权限（如打印、修改）
    /// user_password: 只能打开文档
    pub fn with_pairs<T: Into<String>, U: Into<String>>(owner: T, user: U) -> Self {
        Self {
            owner_password: owner.into(),
            user_password: user.into(),
        }
    }
}

impl PdfOperation<()> for PdfEncryptOperation {
    fn execute(self, doc: &mut Document) -> Result<(), PdfError> {
        // 1. 检查文档是否已经加密
        // 如果已加密，lopdf 通常需要先解密才能重新加密，或者直接报错
        if doc.is_encrypted() {
            return Err(PdfError::AnalysisError(
                "The document has been encrypted, please decrypt it before setting a new password"
                    .to_string(),
            ));
        }

        // 2. 设置权限
        // 这里默认开启：打印、复制、无障碍复制、高质量打印
        // 如果你需要更细粒度的控制，可以在 struct 中传入 Permissions
        let permissions = Permissions::PRINTABLE
            | Permissions::COPYABLE
            | Permissions::COPYABLE_FOR_ACCESSIBILITY
            | Permissions::PRINTABLE_IN_HIGH_QUALITY;

        // 3. 准备 V5 (AES-256) 加密所需的组件
        // 定义过滤器名称
        let filter_name = b"StdCF".to_vec();

        // 创建 AES-256 过滤器
        let crypt_filter: Arc<dyn CryptFilter> = Arc::new(Aes256CryptFilter);

        // 构建过滤器映射表
        let crypt_filters = BTreeMap::from([(filter_name.clone(), crypt_filter)]);

        // 生成 32 字节的随机文件加密密钥 (File Encryption Key)
        let mut file_encryption_key = [0u8; 32];
        let mut rng = rand::thread_rng();
        rng.fill(&mut file_encryption_key);

        // 4. 构建加密版本配置 (V5 - AES 256)
        let version = EncryptionVersion::V5 {
            encrypt_metadata: true, // 加密元数据
            crypt_filters,          // 加密算法过滤器
            file_encryption_key: &file_encryption_key,
            stream_filter: filter_name.clone(), // 流数据使用该过滤器
            string_filter: filter_name,         // 字符串数据使用该过滤器
            owner_password: &self.owner_password,
            user_password: &self.user_password,
            permissions,
        };

        // 5. 转换为加密状态
        // try_from 可能会失败，需要处理错误
        let state = EncryptionState::try_from(version)
            .map_err(|e| PdfError::Custom(format!("Failed to build encryption state: {:?}", e)))?;

        // 6. 执行加密
        // 这一步会修改 doc 中的对象，将 Trailer 指向加密字典，并处理 ID
        doc.encrypt(&state)
            .map_err(|e| PdfError::Custom(format!("Encryption execution failed: {:?}", e)))?;

        Ok(())
    }
}
