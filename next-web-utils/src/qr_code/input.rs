pub struct QrCodeInput;

#[derive(Debug)]
pub struct ParsedQrData {
    pub raw_content: String,
    pub content_type: QrContentType,
}

#[derive(Debug)]
pub enum QrContentType {
    Url,
    Text,
    VCard,
    Email,
    Unknown,
}

#[derive(Debug)]
pub enum QrCodeError {
    ImageError(String),
    DecodeError(String),
}

impl QrCodeInput {
    /// 解析二维码扫描后的数据
    /// 返回解析后的数据结构
    pub fn parse(input: &str) -> ParsedQrData {
        let content_type = if input.starts_with("http://") || input.starts_with("https://") {
            QrContentType::Url
        } else if input.starts_with("BEGIN:VCARD") {
            QrContentType::VCard
        } else if input.contains('@') && !input.contains(' ') {
            QrContentType::Email
        } else {
            QrContentType::Text
        };

        ParsedQrData {
            raw_content: input.to_string(),
            content_type,
        }
    }

    /// 从原始字节数据解析二维码
    /// 参数：
    /// - data: 二维码图片的字节数据
    /// 返回：解析后的二维码数据或错误
    pub fn parse_from_bytes(data: &[u8]) -> Result<ParsedQrData, QrCodeError> {
        use image::io::Reader as ImageReader;
        use std::io::Cursor;

        // 从字节数据加载图片
        let img = ImageReader::new(Cursor::new(data))
            .with_guessed_format()
            .map_err(|e| QrCodeError::ImageError(e.to_string()))?
            .decode()
            .map_err(|e| QrCodeError::ImageError(e.to_string()))?;

        // 使用 bardecoder 解析二维码
        let decoder = bardecoder::default_decoder();
        let results = decoder.decode(&img);
        
        // 获取第一个成功的结果
        let content = results
            .into_iter()
            .find_map(|result| result.ok())
            .ok_or_else(|| QrCodeError::DecodeError("No QR code found".to_string()))?;

        // 解析内容
        Ok(Self::parse(&content))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_url() {
        let input = "https://www.example.com";
        let result = QrCodeInput::parse(input);
        assert!(matches!(result.content_type, QrContentType::Url));
    }

    #[test]
    fn test_parse_email() {
        let input = "test@example.com";
        let result = QrCodeInput::parse(input);
        assert!(matches!(result.content_type, QrContentType::Email));
    }
}