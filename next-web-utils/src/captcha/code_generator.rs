use rand::{distributions::Uniform, Rng};

/// Generates captcha display text and the value used for verification.
pub trait CodeGenerator: Send + Sync {
    fn generate(&self) -> CaptchaCode;
}

/// Generated captcha content.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CaptchaCode {
    pub display: String,
    pub answer: String,
}

impl CaptchaCode {
    #[must_use]
    pub fn new(display: impl Into<String>, answer: impl Into<String>) -> Self {
        Self {
            display: display.into(),
            answer: answer.into(),
        }
    }
}

/// Random fixed-length generator backed by a caller-provided character set.
#[derive(Debug, Clone)]
pub struct RandomGenerator {
    chars: Vec<char>,
    length: usize,
}

impl RandomGenerator {
    #[must_use]
    pub fn new(chars: impl AsRef<str>, length: usize) -> Self {
        Self {
            chars: chars.as_ref().chars().collect(),
            length,
        }
    }
}

impl Default for RandomGenerator {
    fn default() -> Self {
        Self::new("23456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghjkmnpqrstuvwxyz", 5)
    }
}

impl CodeGenerator for RandomGenerator {
    fn generate(&self) -> CaptchaCode {
        if self.chars.is_empty() || self.length == 0 {
            return CaptchaCode::new("", "");
        }

        let mut rng = rand::thread_rng();
        let range = Uniform::from(0..self.chars.len());
        let value = (0..self.length)
            .map(|_| self.chars[rng.sample(range)])
            .collect::<String>();

        CaptchaCode::new(value.clone(), value)
    }
}

/// Simple arithmetic captcha generator.
#[derive(Debug, Clone)]
pub struct MathGenerator {
    max_number: u32,
}

impl MathGenerator {
    #[must_use]
    pub fn new(max_number: u32) -> Self {
        Self { max_number }
    }
}

impl Default for MathGenerator {
    fn default() -> Self {
        Self::new(20)
    }
}

impl CodeGenerator for MathGenerator {
    fn generate(&self) -> CaptchaCode {
        let mut rng = rand::thread_rng();
        let upper = self.max_number.max(1);
        let left = rng.gen_range(0..=upper);
        let right = rng.gen_range(1..=upper);

        match rng.gen_range(0..4) {
            0 => CaptchaCode::new(format!("{left}+{right}=?"), (left + right).to_string()),
            1 => {
                let (max, min) = if left >= right {
                    (left, right)
                } else {
                    (right, left)
                };
                CaptchaCode::new(format!("{max}-{min}=?"), (max - min).to_string())
            }
            2 => CaptchaCode::new(format!("{left}x{right}=?"), (left * right).to_string()),
            _ => {
                let product = left * right;
                CaptchaCode::new(format!("{product}/{right}=?"), left.to_string())
            }
        }
    }
}
