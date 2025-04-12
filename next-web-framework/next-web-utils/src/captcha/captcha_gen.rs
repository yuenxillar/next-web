use super::captcha_error::CaptchaError;

/// 验证码生成结果
#[derive(Debug)]
pub struct CaptchaResult {
    /// 验证码文本
    pub text: String,
    /// 原始图片数据
    pub image_data: Vec<u8>,
    /// 图片宽度
    pub width: u32,
    /// 图片高度
    pub height: u32,
    /// 验证码长度
    pub length: usize,
    /// 验证码复杂度
    pub complexity: u32,
}

// 定义 CaptchaBuilder 结构体，用于配置验证码生成器的参数
#[derive(Debug, Default, Clone)]
pub(crate) struct CaptchaBuilder {
    text: Option<String>, // 自定义文本（可选）
    length: usize,        // 验证码长度
    complexity: u32,      // 验证码复杂度
    width: u32,           // 图片宽度
    height: u32,          // 图片高度
}

impl CaptchaBuilder {
    // 创建一个新的 CaptchaBuilder 实例，并设置默认值
    pub fn builder() -> Self {
        Self {
            length: 6,     // 默认长度为 6
            complexity: 1, // 默认复杂度为 1
            width: 150,    // 默认宽度为 150
            height: 50,    // 默认高度为 50
            text: None,    // 默认没有自定义文本
        }
    }

    // 设置验证码长度
    pub fn length(mut self, length: usize) -> Self {
        self.length = length;
        self
    }

    // 设置验证码复杂度
    pub fn complexity(mut self, complexity: u32) -> Self {
        self.complexity = complexity;
        self
    }

    // 设置图片的宽度和高度
    pub fn size(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    // 设置自定义文本
    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }

    // 构建 CaptchaGen 实例
    pub fn build(self) -> CaptchaGen {
        CaptchaGen { config: self }
    }
}

// 定义 CaptchaGen 结构体，用于生成验证码
pub struct CaptchaGen {
    config: CaptchaBuilder, // 包含 CaptchaBuilder 的配置
}

impl CaptchaGen {
    // 生成验证码并返回原始数据
    fn _gen(&self, base64: bool) -> Result<(String, Vec<u8>), CaptchaError> {
        // 检查宽度是否有效
        if self.config.width <= 0 {
            return Err(CaptchaError::WidthNotApplicable);
        }
        // 检查高度是否有效
        if self.config.height <= 0 {
            return Err(CaptchaError::HeightNotApplicable);
        }

        // 使用 captcha_rs 库创建验证码生成器
        let mut captcha = captcha_rs::CaptchaBuilder::new()
            .length(self.config.length)
            .width(self.config.width)
            .height(self.config.height)
            .dark_mode(false)
            .complexity(self.config.complexity)
            .compression(40);

        // 如果设置了自定义文本，则使用该文本
        if let Some(text) = self.config.text.as_deref() {
            captcha = captcha.text(text.into());
        }

        // 构建验证码
        let captcha = captcha.build();

        // 获取生成的验证码文本
        let text = if let Some(text) = self.config.text.as_ref() {
            text.into()
        } else {
            captcha.text.clone()
        };

        if base64 {
            Ok((text, captcha.to_base64().into_bytes()))
        } else {
            // 获取原始图片数据
            let image_data = captcha.image.into_bytes();
            Ok((text, image_data))
        }
    }

    pub fn gen(self) -> Result<CaptchaResult, CaptchaError> {
        let result = self._gen(false)?;
        Ok(CaptchaResult {
            text: result.0,
            image_data: result.1,
            width: self.config.width,
            height: self.config.height,
            length: self.config.length,
            complexity: self.config.complexity,
        })
    }

    pub fn gen_to_base64(self) -> Result<CaptchaResult, CaptchaError> {
        let result = self._gen(true)?;

        Ok(CaptchaResult {
            text: result.0,
            image_data: result.1,
            width: self.config.width,
            height: self.config.height,
            length: self.config.length,
            complexity: self.config.complexity,
        })
    }

    // 提供一个静态方法来创建 CaptchaBuilder 实例
    pub fn builder() -> CaptchaBuilder {
        CaptchaBuilder::builder()
    }
}

// 测试模块
#[cfg(test)]
mod tests {
    use super::*;

    // 测试默认配置是否正确
    #[test]
    fn test_default_values() {
        let builder = CaptchaBuilder::builder();
        assert_eq!(builder.length, 6, "Default length should be 6");
        assert_eq!(builder.complexity, 1, "Default complexity should be 1");
        assert_eq!(builder.width, 150, "Default width should be 150");
        assert_eq!(builder.height, 50, "Default height should be 50");
        assert!(builder.text.is_none(), "Default text should be None");
    }

    // 测试 length 方法是否正确更新长度
    #[test]
    fn test_length_config() {
        let builder = CaptchaBuilder::builder().length(8);
        assert_eq!(builder.length, 8, "Length should be updated to 8");
    }

    // 测试 complexity 方法是否正确更新复杂度
    #[test]
    fn test_complexity_config() {
        let builder = CaptchaBuilder::builder().complexity(5);
        assert_eq!(builder.complexity, 5, "Complexity should be updated to 5");
    }

    // 测试 size 方法是否正确更新宽度和高度
    #[test]
    fn test_size_config() {
        let builder = CaptchaBuilder::builder().size(200, 100);
        assert_eq!(builder.width, 200, "Width should be updated to 200");
        assert_eq!(builder.height, 100, "Height should be updated to 100");
    }

    // 测试 text 方法是否正确设置自定义文本
    #[test]
    fn test_text_config() {
        let builder = CaptchaBuilder::builder().text("CUSTOMTEXT");
        assert_eq!(
            builder.text,
            Some("CUSTOMTEXT".to_string()),
            "Text should be set to CUSTOMTEXT"
        );
    }

    // 测试链式调用是否按预期工作
    #[test]
    fn test_chained_config() {
        let builder = CaptchaBuilder::builder()
            .length(8)
            .complexity(3)
            .size(300, 150)
            .text("CHAINTEST");

        assert_eq!(builder.length, 8, "Length should be updated to 8");
        assert_eq!(builder.complexity, 3, "Complexity should be updated to 3");
        assert_eq!(builder.width, 300, "Width should be updated to 300");
        assert_eq!(builder.height, 150, "Height should be updated to 150");
        assert_eq!(
            builder.text,
            Some("CHAINTEST".to_string()),
            "Text should be set to CHAINTEST"
        );
    }

    // 测试默认配置下生成的验证码
    #[test]
    fn test_captcha_gen_with_default_config() {
        let captcha_gen = CaptchaBuilder::builder().build();
        let result = captcha_gen.gen();
        assert!(result.is_ok(), "Captcha generation should succeed");

        let captcha_result = result.unwrap();
        assert_eq!(
            captcha_result.text.len(),
            6,
            "Generated text length should match default length of 6"
        );
        assert_eq!(captcha_result.width, 150);
        assert_eq!(captcha_result.height, 50);
        assert_eq!(captcha_result.length, 6);
        assert_eq!(captcha_result.complexity, 1);
        assert!(!captcha_result.image_data.is_empty());
    }

    // 测试自定义配置下生成的验证码
    #[test]
    fn test_captcha_gen_with_custom_config() {
        let captcha_gen = CaptchaBuilder::builder()
            .length(10)
            .complexity(5)
            .size(400, 200)
            .text("CUSTOM")
            .build();

        let result = captcha_gen.gen();
        assert!(result.is_ok(), "Captcha generation should succeed");

        let captcha_result = result.unwrap();
        assert_eq!(captcha_result.text, "CUSTOM");
        assert_eq!(captcha_result.width, 400);
        assert_eq!(captcha_result.height, 200);
        assert_eq!(captcha_result.length, 10);
        assert_eq!(captcha_result.complexity, 5);
        assert!(!captcha_result.image_data.is_empty());
    }

    // 测试未设置自定义文本时生成的验证码
    #[test]
    fn test_captcha_gen_without_custom_text() {
        let captcha_gen = CaptchaBuilder::builder().length(8).build();

        let result = captcha_gen.gen();
        assert!(result.is_ok(), "Captcha generation should succeed");

        let captcha_result = result.unwrap();
        assert_eq!(
            captcha_result.text.len(),
            8,
            "Generated text length should match configured length of 8"
        );
        assert_eq!(captcha_result.width, 150);
        assert_eq!(captcha_result.height, 50);
        assert_eq!(captcha_result.length, 8);
        assert_eq!(captcha_result.complexity, 1);
        assert!(!captcha_result.image_data.is_empty());
    }

    // 测试生成验证码并保存为图片文件
    #[test]
    fn gen_code() -> Result<(), Box<dyn std::error::Error>> {
        // 生成验证码
        let result = CaptchaGen::builder()
            .size(100, 40)
            .text("ABCD")
            .build()
            .gen();

        // 处理生成结果
        result.map(|captcha_result| {
            println!("code: {}", captcha_result.text); // 打印验证码文本
            println!("width: {}", captcha_result.width); // 打印图片宽度
            println!("height: {}", captcha_result.height); // 打印图片高度
            println!("length: {}", captcha_result.length); // 打印验证码长度
            println!("complexity: {}", captcha_result.complexity); // 打印验证码复杂度

            // 将图片数据保存为文件
            std::fs::write("test.png", &captcha_result.image_data)?;
            Ok(())
        })?
    }

    #[test]
    fn test_gen() {
        let captcha_gen = CaptchaBuilder::builder()
            .length(6)
            .complexity(3)
            .size(200, 100)
            .text("TEST123")
            .build();

        let result = captcha_gen.gen();
        assert!(result.is_ok(), "Captcha generation should succeed");

        let captcha_result = result.unwrap();
        assert_eq!(captcha_result.text, "TEST123");
        assert_eq!(captcha_result.width, 200);
        assert_eq!(captcha_result.height, 100);
        assert_eq!(captcha_result.length, 6);
        assert_eq!(captcha_result.complexity, 3);
        assert!(!captcha_result.image_data.is_empty());
    }
}
