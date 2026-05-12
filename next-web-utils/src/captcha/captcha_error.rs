#[derive(Debug, Clone)]
pub enum CaptchaError {
    WidthNotApplicable,
    HeightNotApplicable,
    CodeLengthNotApplicable,
    FontLoadError,
    ImageEncodeError(String),
    IoError(String),
}

impl std::fmt::Display for CaptchaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CaptchaError::WidthNotApplicable => write!(f, "Width not applicable"),
            CaptchaError::HeightNotApplicable => write!(f, "Height not applicable"),
            CaptchaError::CodeLengthNotApplicable => write!(f, "Code length not applicable"),
            CaptchaError::FontLoadError => write!(f, "Failed to load captcha font"),
            CaptchaError::ImageEncodeError(message) => write!(f, "Image encode error: {message}"),
            CaptchaError::IoError(message) => write!(f, "I/O error: {message}"),
        }
    }
}

impl std::error::Error for CaptchaError {}

impl From<std::io::Error> for CaptchaError {
    fn from(value: std::io::Error) -> Self {
        CaptchaError::IoError(value.to_string())
    }
}
