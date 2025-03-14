

#[derive(Debug, Clone)]
pub enum CaptchaError {
    WidthNotApplicable,
    HeightNotApplicable,
}

impl std::fmt::Display for CaptchaError  {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CaptchaError::WidthNotApplicable => write!(f, "Width not applicable"),
            CaptchaError::HeightNotApplicable => write!(f, "Height not applicable"),
        }
    }
}

impl std::error::Error for CaptchaError {
    
}