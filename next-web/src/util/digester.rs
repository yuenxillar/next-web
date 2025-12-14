use std::io::Read;
use std::path::Path;
use std::fs::File;
use std::string::String;
use std::vec::Vec;
use md5;
use sha1::Sha1;
use sha2::{Sha256, Sha384, Sha512, Digest};

/// 哈希算法枚举
///  
/// Hash Algorithm Enum
#[derive(Debug, Clone, Copy)]
pub enum HashAlgorithm {
    /// MD5 哈希算法 
    /// 
    ///  MD5 Hash Algorithm
    Md5,
    /// SHA-1 哈希算法 
    /// 
    ///  SHA-1 Hash Algorithm
    Sha1,
    /// SHA-256 哈希算法 
    /// 
    ///  SHA-256 Hash Algorithm
    Sha256,
    /// SHA-384 哈希算法 
    /// 
    ///  SHA-384 Hash Algorithm
    Sha384,
    /// SHA-512 哈希算法 
    /// 
    ///  SHA-512 Hash Algorithm
    Sha512,
}

/// 哈希计算器 trait
/// 
/// Hash Calculator Trait
pub trait Digester {
    /// 计算哈希值
    /// 
    /// Calculate hash value
    /// 
    /// # Arguments
    /// * `algorithm` - 使用的哈希算法
    /// 
    /// # Returns
    /// * `String` - 十六进制格式的哈希值
    fn hash(&self, algorithm: HashAlgorithm) -> String;
}

impl Digester for String {
    fn hash(&self, algorithm: HashAlgorithm) -> String {
        self.as_bytes().hash(algorithm)
    }
}

impl Digester for &[u8] {
    fn hash(&self, algorithm: HashAlgorithm) -> String {
        match algorithm {
            HashAlgorithm::Md5 => {
                let digest = md5::compute(self);
                format!("{:x}", digest)
            }
            HashAlgorithm::Sha1 => {
                let mut hasher = Sha1::new();
                hasher.update(self);
                format!("{:x}", hasher.finalize())
            }
            HashAlgorithm::Sha256 => {
                let mut hasher = Sha256::new();
                hasher.update(self);
                format!("{:x}", hasher.finalize())
            }
            HashAlgorithm::Sha384 => {
                let mut hasher = Sha384::new();
                hasher.update(self);
                format!("{:x}", hasher.finalize())
            }
            HashAlgorithm::Sha512 => {
                let mut hasher = Sha512::new();
                hasher.update(self);
                format!("{:x}", hasher.finalize())
            }
        }
    }
}

impl Digester for Vec<u8> {
    fn hash(&self, algorithm: HashAlgorithm) -> String {
        self.as_slice().hash(algorithm)
    }
}

impl Digester for File {
    fn hash(&self, algorithm: HashAlgorithm) -> String {
        let mut buffer = Vec::new();
        let mut file = self.try_clone().unwrap();
        file.read_to_end(&mut buffer).unwrap();
        buffer.hash(algorithm)
    }
}

/// 计算文件的哈希值 
/// 
///  Calculate file hash
/// 
/// # Arguments / 参数
/// * `path` - 文件路径 
/// 
///  File path
///
/// * `algorithm` - 使用的哈希算法 
/// 
///  Hash algorithm to use
/// 
/// # Returns 
///  返回值
/// * `std::io::Result<String>` - 十六进制格式的哈希值 
///  Hash value in hexadecimal format
/// 
/// # Errors 
///  错误
/// * 如果文件无法打开或读取 / If the file cannot be opened or read
pub fn hash_file<P: AsRef<Path>>(path: P, algorithm: HashAlgorithm) -> std::io::Result<String> {
    let file = File::open(path)?;
    Ok(file.hash(algorithm))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试字符串哈希 
    /// 
    /// Test string hashing
    #[test]
    fn test_string_hash() {
        let text = "Hello, World!".to_string();
        let md5_hash = text.hash(HashAlgorithm::Md5);
        assert_eq!(md5_hash, "65a8e27d8879283831b664bd8b7f0ad4");
        
        let sha256_hash = text.hash(HashAlgorithm::Sha256);
        assert_eq!(sha256_hash, "dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f");
    }

    /// 测试字节数组哈希 
    /// 
    /// Test byte array hashing
    #[test]
    fn test_bytes_hash() {
        let bytes = vec![1, 2, 3, 4, 5];
        let md5_hash = bytes.hash(HashAlgorithm::Md5);
        assert_eq!(md5_hash, "7cfdd07889b3295d6a550914ab35e068");
        
        let sha1_hash = bytes.hash(HashAlgorithm::Sha1);
        assert_eq!(sha1_hash, "8f9baf15c0c6aa4887d832e415735772a5a05a5c");
    }
}
