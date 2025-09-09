// use aes_gcm::{Aes256Gcm, Nonce}; // AES-256-GCM
// use rand::RngCore;


// /// AES-GCM 工具：支持加密和解密
// pub struct AesGcmTool {
//     cipher: Aes256Gcm,
// }

// impl AesGcmTool {
//     /// 创建新实例，使用 32 字节密钥（AES-256）
//     pub fn new(key: &[u8; 32]) -> Self {
//         let key = GenericArray::from_slice(key);
//         let cipher = Aes256Gcm::new(key);
//         Self { cipher }
//     }

//     /// 加密明文
//     /// 
//     /// 返回：`Vec<u8>` 格式为 `[nonce (12 bytes)][ciphertext]`
//     pub fn encrypt(&self, plaintext: &[u8]) -> Vec<u8> {
//         let mut nonce_bytes = [0u8; 12]; // GCM 推荐 96-bit nonce
//         rand::thread_rng().fill_bytes(&mut nonce_bytes);
//         let nonce = Nonce::from_slice(&nonce_bytes);

//         let ciphertext = self.cipher.encrypt(nonce, plaintext.as_ref())
//             .expect("Encryption failed");

//         // 拼接：nonce + ciphertext
//         [&nonce_bytes[..], &ciphertext[..]].concat()
//     }

//     /// 解密数据
//     /// 
//     /// 输入格式应为 `[nonce (12 bytes)][ciphertext]`
//     pub fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>, anyhow::Error> {
//         if data.len() < 12 {
//             return Err(anyhow::anyhow!("Data too short: missing nonce"));
//         }

//         let (nonce_bytes, ciphertext) = data.split_at(12);
//         let nonce = Nonce::from_slice(nonce_bytes);

//         let plaintext = self.cipher.decrypt(nonce, ciphertext.as_ref())
//             .map_err(|_| anyhow::anyhow!("Decryption failed"))?;

//         Ok(plaintext)
//     }

//     /// 将密钥格式化为 hex（便于打印或存储）
//     pub fn key_to_hex(key: &[u8; 32]) -> String {
//         hex::encode(key)
//     }

//     /// 从 hex 字符串解析密钥
//     pub fn key_from_hex(hex_str: &str) -> Result<[u8; 32], hex::FromHexError> {
//         let mut key = [0u8; 32];
//         let bytes = hex::decode(hex_str)?;
//         if bytes.len() != 32 {
//             return Err(hex::FromHexError::InvalidStringLength);
//         }
//         key.copy_from_slice(&bytes);
//         Ok(key)
//     }
// }