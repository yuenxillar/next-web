use fast_qr::{
    convert::{image::ImageBuilder, svg::SvgBuilder, Builder, Shape},
    QRBuilder,
};

pub struct QrCodeGenerator;

impl QrCodeGenerator {
    /// 生成二维码的方法
    /// Generates a QR code based on the provided content and type.
    /// 内容：要编码为二维码的字符串数据 | Content: The string data to encode into the QR code.
    /// 类型：指定生成的二维码格式（SVG 或 PNG） | Type: Specifies the format of the QR code (SVG or PNG).
    /// 返回值：成功时返回 `QrCodeOutput`，失败时返回错误 | Returns: `QrCodeOutput` on success, error on failure.
    /// 使用 `QRBuilder` 创建二维码对象 | Create a QR code object using `QRBuilder`.
    pub fn generate_qr_code(
        content: &str,
        qr_code_type: QrCodeType,
    ) -> Result<QrCodeOutput, Box<dyn std::error::Error>> {
        let qrcode = QRBuilder::new(content).build()?;
        // 根据指定的类型生成二维码输出
        // Generate QR code output based on the specified type.
        let output = match qr_code_type {
            QrCodeType::Svg => {
                let svg = SvgBuilder::default()
                    .shape(Shape::RoundedSquare)
                    .to_str(&qrcode);
                QrCodeOutput::Svg(svg)
            }
            QrCodeType::Png(width, height) => {
                // 使用默认的图像构建器 | Use the default image builder.
                let png = ImageBuilder::default()
                    .shape(Shape::RoundedSquare)
                    .background_color([255, 255, 255, 0])
                    .fit_width(width)
                    .fit_height(height)
                    .to_bytes(&qrcode)?;
                QrCodeOutput::Png(png)
            }
        };
        // 返回生成的二维码输出 | Return the generated QR code output.
        Ok(output)
    }
}

/// 定义二维码的输出类型
/// Defines the output format of the QR code.
pub enum QrCodeOutput {
    // SVG 格式的二维码，存储为字符串 | SVG format QR code, stored as a string.
    Svg(String),
    // PNG 格式的二维码，存储为字节数组 | PNG format QR code, stored as a byte array.
    Png(Vec<u8>),
}

/// 定义二维码的生成类型
/// Defines the type of QR code to generate.
#[derive(Debug, Clone, Copy)]
pub enum QrCodeType {
    // 表示生成 SVG 格式的二维码 | Indicates generating an SVG format QR code.
    Svg,
    // 表示生成 PNG 格式的二维码，并指定宽度和高度 | Indicates generating a PNG format QR code with specified width and height.
    Png(u32, u32),
}
