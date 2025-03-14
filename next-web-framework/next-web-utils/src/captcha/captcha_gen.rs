use super::captcha_error::CaptchaError;

// 定义 CaptchaBuilder 结构体，用于配置验证码生成器的参数
#[derive(Debug, Default, Clone)]
struct CaptchaBuilder {
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
    // 生成验证码并返回结果
    pub fn gen(self) -> Result<(String, String), CaptchaError> {
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
            .complexity(self.config.complexity) // 设置复杂度（最小值为 1，最大值为 10）
            .compression(40); // 设置压缩率（最小值为 1，最大值为 99）

        // 如果设置了自定义文本，则使用该文本
        if let Some(text) = self.config.text.as_deref() {
            captcha = captcha.text(text.into());
        }

        // 构建验证码
        let captcha = captcha.build();

        // 获取生成的验证码文本
        let text = if let Some(text) = self.config.text.as_ref() {
            text.into() // 如果设置了自定义文本，则使用该文本
        } else {
            captcha.text.clone() // 否则使用随机生成的文本
        };

        // 返回验证码文本和 Base64 编码的图片数据
        Ok((text, captcha.to_base64()))
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

        let (text, _) = result.unwrap();
        assert_eq!(text.len(), 6, "Generated text length should match default length of 6");
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

        let (text, _) = result.unwrap();
        assert_eq!(text, "CUSTOM", "Generated text should match custom text");
    }

    // 测试未设置自定义文本时生成的验证码
    #[test]
    fn test_captcha_gen_without_custom_text() {
        let captcha_gen = CaptchaBuilder::builder()
            .length(8)
            .build();

        let result = captcha_gen.gen();
        assert!(result.is_ok(), "Captcha generation should succeed");

        let (text, _) = result.unwrap();
        assert_eq!(text.len(), 8, "Generated text length should match configured length of 8");
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
        let _ = result.map(|(code, base64)| {
            println!("code: {}", code); // 打印验证码文本
            println!("base64: {}", base64); // 打印 Base64 编码的图片数据

            // 将 Base64 数据保存为图片文件
            std::fs::write("test.png", base64).unwrap();
        });

        Ok(())
    }
}