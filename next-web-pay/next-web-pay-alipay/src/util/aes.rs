use aes::{
    Aes128, Block,
    cipher::{BlockDecrypt, BlockEncrypt, KeyInit as _},
};
use aes_gcm::{Aes256Gcm, Nonce, aead::Aead};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use rand::RngCore;

pub struct AesUtil;

impl AesUtil {
    /// Encrypts content in the format expected by Alipay OpenAPI `encrypt_type=AES`.
    pub fn encrypt_alipay_content(
        aes_key_base64: &str,
        biz_content_json: &str,
    ) -> Result<String, String> {
        let key = decode_aes128_key(aes_key_base64)?;
        let ciphertext = encrypt_cbc_pkcs7(key, *b"0102030405060708", biz_content_json.as_bytes());
        Ok(BASE64.encode(ciphertext))
    }

    /// Decrypts Alipay OpenAPI AES encrypted content.
    pub fn decrypt_alipay_content(
        aes_key_base64: &str,
        encrypted_base64: &str,
    ) -> Result<String, String> {
        let key = decode_aes128_key(aes_key_base64)?;
        let encrypted = BASE64
            .decode(encrypted_base64)
            .map_err(|error| format!("failed to decode encrypted content: {error}"))?;
        let plaintext = decrypt_cbc_pkcs7(key, *b"0102030405060708", &encrypted)?;

        String::from_utf8(plaintext)
            .map_err(|error| format!("decrypted content is not valid UTF-8: {error}"))
    }

    pub fn encrypt_biz_content(
        aes_key_base64: &str,
        biz_content_json: &str,
    ) -> Result<String, String> {
        let key = decode_aes128_key(aes_key_base64)?;

        let mut iv = [0u8; 16];
        rand::thread_rng().fill_bytes(&mut iv);

        let ciphertext = encrypt_cbc_pkcs7(key, iv, biz_content_json.as_bytes());
        let mut combined = Vec::with_capacity(iv.len() + ciphertext.len());
        combined.extend_from_slice(&iv);
        combined.extend_from_slice(&ciphertext);

        Ok(BASE64.encode(&combined))
    }

    pub fn decrypt_biz_content(
        aes_key_base64: &str,
        encrypted_base64: &str,
    ) -> Result<String, String> {
        let key = decode_aes128_key(aes_key_base64)?;
        let encrypted = BASE64
            .decode(encrypted_base64)
            .map_err(|error| format!("failed to decode encrypted biz content: {error}"))?;

        if encrypted.len() < 32 || encrypted.len() % 16 != 0 {
            return Err(
                "encrypted biz content must contain a 16-byte IV followed by AES blocks"
                    .to_string(),
            );
        }

        let mut iv = [0u8; 16];
        iv.copy_from_slice(&encrypted[..16]);
        let plaintext = decrypt_cbc_pkcs7(key, iv, &encrypted[16..])?;

        String::from_utf8(plaintext)
            .map_err(|error| format!("decrypted biz content is not valid UTF-8: {error}"))
    }

    pub fn decrypt(key: &[u8; 32], nonce: &[u8; 12], ciphertext: &[u8]) -> Result<Vec<u8>, String> {
        let cipher = Aes256Gcm::new_from_slice(key)
            .map_err(|error| format!("failed to initialize AES-256-GCM cipher: {error}"))?;

        cipher
            .decrypt(Nonce::from_slice(nonce), ciphertext)
            .map_err(|error| format!("failed to decrypt ciphertext: {error}"))
    }
}

fn decode_aes128_key(aes_key_base64: &str) -> Result<[u8; 16], String> {
    let key_bytes = BASE64
        .decode(aes_key_base64)
        .map_err(|error| format!("failed to decode AES key: {error}"))?;

    if key_bytes.len() != 16 {
        return Err(format!(
            "AES key must be 16 bytes for AES-128, got {} bytes",
            key_bytes.len()
        ));
    }

    let mut key = [0u8; 16];
    key.copy_from_slice(&key_bytes);
    Ok(key)
}

fn encrypt_cbc_pkcs7(key: [u8; 16], iv: [u8; 16], plaintext: &[u8]) -> Vec<u8> {
    let cipher = Aes128::new(&key.into());
    let padded = pkcs7_pad(plaintext);
    let mut prev_block = iv;
    let mut ciphertext = Vec::with_capacity(padded.len());

    for chunk in padded.chunks(16) {
        let mut block_bytes = [0u8; 16];
        for (index, byte) in chunk.iter().enumerate() {
            block_bytes[index] = *byte ^ prev_block[index];
        }

        let mut block = Block::from(block_bytes);
        cipher.encrypt_block(&mut block);

        let encrypted_block: &[u8] = block.as_ref();
        ciphertext.extend_from_slice(encrypted_block);
        prev_block.copy_from_slice(encrypted_block);
    }

    ciphertext
}

fn decrypt_cbc_pkcs7(key: [u8; 16], iv: [u8; 16], ciphertext: &[u8]) -> Result<Vec<u8>, String> {
    if ciphertext.is_empty() || ciphertext.len() % 16 != 0 {
        return Err("ciphertext length must be a non-zero multiple of 16 bytes".to_string());
    }

    let cipher = Aes128::new(&key.into());
    let mut prev_block = iv;
    let mut plaintext = Vec::with_capacity(ciphertext.len());

    for chunk in ciphertext.chunks(16) {
        let mut block_bytes = [0u8; 16];
        block_bytes.copy_from_slice(chunk);

        let mut block = Block::from(block_bytes);
        cipher.decrypt_block(&mut block);

        let decrypted_block: &[u8] = block.as_ref();
        let mut plain_block = [0u8; 16];
        for index in 0..16 {
            plain_block[index] = decrypted_block[index] ^ prev_block[index];
        }

        plaintext.extend_from_slice(&plain_block);
        prev_block.copy_from_slice(chunk);
    }

    pkcs7_unpad(&plaintext).map(|data| data.to_vec())
}

fn pkcs7_pad(plaintext: &[u8]) -> Vec<u8> {
    let padding_len = 16 - (plaintext.len() % 16);
    let mut padded = Vec::with_capacity(plaintext.len() + padding_len);
    padded.extend_from_slice(plaintext);
    padded.extend(std::iter::repeat_n(padding_len as u8, padding_len));
    padded
}

fn pkcs7_unpad(data: &[u8]) -> Result<&[u8], String> {
    let Some(&padding_len) = data.last() else {
        return Err("ciphertext is empty after decryption".to_string());
    };
    let padding_len = padding_len as usize;

    if padding_len == 0 || padding_len > 16 || padding_len > data.len() {
        return Err("invalid PKCS#7 padding length".to_string());
    }

    if data[data.len() - padding_len..]
        .iter()
        .any(|byte| *byte as usize != padding_len)
    {
        return Err("invalid PKCS#7 padding bytes".to_string());
    }

    Ok(&data[..data.len() - padding_len])
}

#[cfg(test)]
mod tests {
    use super::AesUtil;
    use aes_gcm::{Aes256Gcm, KeyInit as _, Nonce, aead::Aead};
    use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

    #[test]
    fn encrypt_and_decrypt_biz_content_round_trip() {
        let key_base64 = BASE64.encode([0x11_u8; 16]);
        let plaintext = r#"{"out_trade_no":"trade-001","total_amount":"88.00"}"#;

        let encrypted = AesUtil::encrypt_biz_content(&key_base64, plaintext).expect("encrypt");
        let decrypted = AesUtil::decrypt_biz_content(&key_base64, &encrypted).expect("decrypt");

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn encrypt_and_decrypt_alipay_content_round_trip() {
        let key_base64 = BASE64.encode([0x11_u8; 16]);
        let plaintext = r#"{"out_trade_no":"trade-001","total_amount":"88.00"}"#;

        let encrypted = AesUtil::encrypt_alipay_content(&key_base64, plaintext).expect("encrypt");
        let decrypted = AesUtil::decrypt_alipay_content(&key_base64, &encrypted).expect("decrypt");

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn decrypt_supports_aes_256_gcm_ciphertext() {
        let key = [0x22_u8; 32];
        let nonce = [0x33_u8; 12];
        let cipher = Aes256Gcm::new_from_slice(&key).expect("cipher");
        let ciphertext = cipher
            .encrypt(Nonce::from_slice(&nonce), b"hello world".as_ref())
            .expect("encrypt");

        let decrypted = AesUtil::decrypt(&key, &nonce, &ciphertext).expect("decrypt");

        assert_eq!(decrypted, b"hello world");
    }
}
