use std::{
    fmt,
    io::{Read, Write},
    path::PathBuf,
};

use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    AeadCore, Aes128Gcm, Key, Nonce,
};
use base64::{engine::general_purpose, Engine as _};
use next_web_core::error::BoxError;
use rand::RngCore;
use scrypt::{scrypt, Params};

const KEY_LEN: usize = 16;
const SALT_LEN: usize = 32;

fn derive_key_from_password(
    password: &str,
    salt: &[u8],
) -> Result<Key<Aes128Gcm>, ConfigCryptoError> {
    let params =
        Params::new(10, 8, 1, KEY_LEN).map_err(|_| ConfigCryptoError::InvalidScryptParams)?;
    let mut key_bytes = [0u8; KEY_LEN];
    scrypt(password.as_bytes(), salt, &params, &mut key_bytes)
        .map_err(|_| ConfigCryptoError::KeyDerivationFailed)?;
    Ok(Key::<Aes128Gcm>::from(key_bytes))
}

pub fn encrypt(plaintext: &str, password: &str) -> Result<String, ConfigCryptoError> {
    let mut salt = [0u8; SALT_LEN];
    rand::thread_rng().fill_bytes(&mut salt);

    let key = derive_key_from_password(password, &salt)?;
    let cipher = Aes128Gcm::new(&key);

    let nonce = Aes128Gcm::generate_nonce(&mut OsRng);
    let ciphertext = cipher
        .encrypt(&nonce, plaintext.as_bytes())
        .map_err(|_| ConfigCryptoError::EncryptionFailed)?;

    let mut buf = Vec::with_capacity(SALT_LEN + nonce.len() + ciphertext.len());
    buf.extend_from_slice(&salt);
    buf.extend_from_slice(nonce.as_slice());
    buf.extend_from_slice(&ciphertext);

    Ok(general_purpose::STANDARD.encode(buf))
}

pub fn decrypt(encrypted_b64: &str, password: &str) -> Result<String, ConfigCryptoError> {
    let data = general_purpose::STANDARD
        .decode(encrypted_b64)
        .map_err(|_| ConfigCryptoError::InvalidBase64)?;

    if data.len() < SALT_LEN + 12 {
        return Err(ConfigCryptoError::DataTooShort);
    }

    let (salt, rest) = data.split_at(SALT_LEN);
    let (nonce_bytes, ciphertext) = rest.split_at(12);

    let key = derive_key_from_password(password, salt)?;
    let cipher = Aes128Gcm::new(&key);
    let nonce = Nonce::from_slice(nonce_bytes);

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| ConfigCryptoError::DecryptionFailed)?;

    String::from_utf8(plaintext).map_err(|_| ConfigCryptoError::InvalidUtf8)
}

/// 此函数主要是给使用者进行文本的加密使用
///
/// # 使用事项
/// - 请确保加密文本长度 > 3
/// - 在Cargo.toml 同级目录下创建 `.encrypt.txt` 然后每行输入你需要加密的文本,回车换行,依次填入
/// - 使用该函数,并指定 `password`
/// - 在同级目录生成 `.decrypt.txt` 文件, 然后与 `.encrypt.txt` 每行相对应
///
/// # 参数
/// - `password`: 加密使用的密码
///
/// # 返回值
/// - `Result<(), BoxError>`: 加密成功返回 `Ok(())`，失败返回错误信息
///
pub fn local_file_encrypt(password: &str) -> Result<(), BoxError> {
    let path = PathBuf::new().join(std::env::var("CARGO_MANIFEST_DIR")?);

    let mut file = std::fs::File::open(path.join(".encrypt.txt"))?;
    let mut buf = String::new();
    file.read_to_string(&mut buf)?;

    if buf.is_empty() {
        return Ok(());
    }

    let mut write_buf = String::new();
    for text in buf.split("\n") {
        if text.trim().is_empty() {
            continue;
        }
        write_buf += &(format!("{}\n", encrypt(text.trim(), password)?));
    }

    let mut file = std::fs::File::create(path.join(".decrypt.txt"))?;
    file.write_all(write_buf.as_bytes())?;

    Ok(())
}

/// 配置加解密过程中可能发生的错误类型
#[derive(Debug)]
pub enum ConfigCryptoError {
    /// 密钥派生失败（如 scrypt 参数无效）
    KeyDerivationFailed,
    /// 加密操作失败（如 nonce 生成或 AEAD 加密出错）
    EncryptionFailed,
    /// 解密失败（数据被篡改、密钥错误或格式无效）
    DecryptionFailed,
    /// 输入的 Base64 字符串格式无效
    InvalidBase64,
    /// 解密后数据不是合法的 UTF-8 文本
    InvalidUtf8,
    /// 加密数据长度太短，无法解析出 salt 和 nonce
    DataTooShort,
    /// scrypt 参数设置错误（如内存/迭代次数超出限制）
    InvalidScryptParams,
    /// 自定义错误信息
    Custom(String),
}

impl fmt::Display for ConfigCryptoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::KeyDerivationFailed => write!(f, "Failed to derive encryption key from password"),
            Self::EncryptionFailed => write!(f, "Encryption operation failed"),
            Self::DecryptionFailed => write!(
                f,
                "Decryption failed: data may be corrupted or password is incorrect"
            ),
            Self::InvalidBase64 => write!(f, "Encrypted data is not valid Base64"),
            Self::InvalidUtf8 => write!(f, "Decrypted data is not valid UTF-8"),
            Self::DataTooShort => write!(f, "Encrypted data is too short to be valid"),
            Self::InvalidScryptParams => {
                write!(f, "Invalid scrypt parameters (e.g., too large memory cost)")
            }
            Self::Custom(msg) => write!(f, "Custom error: {}", msg),
        }
    }
}

impl std::error::Error for ConfigCryptoError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt() {
        let plaintext = "Hello, world!";
        let password = "secret";

        let encrypted = encrypt(plaintext, password).expect("Encryption failed");
        let decrypted = decrypt(&encrypted, password).expect("Decryption failed");
        assert_eq!(decrypted, plaintext);
    }
}
