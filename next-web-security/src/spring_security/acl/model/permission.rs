use std::fmt::{Display, Formatter};

pub trait Permission: Send + Sync {
    fn mask(&self) -> i32;

    fn pattern(&self) -> String;
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct SimplePermission {
    mask: i32,
    code: char,
}

impl SimplePermission {
    pub const RESERVED_OFF: char = '.';

    pub fn new(mask: i32, code: char) -> Self {
        assert!(mask != 0, "mask cannot be zero");
        Self { mask, code }
    }

    pub fn code(&self) -> char {
        self.code
    }
}

impl Permission for SimplePermission {
    fn mask(&self) -> i32 {
        self.mask
    }

    fn pattern(&self) -> String {
        let mut result = String::with_capacity(32);
        for bit in (0..32).rev() {
            let active = (self.mask & (1_i32 << bit)) != 0;
            result.push(if active {
                self.code
            } else {
                Self::RESERVED_OFF
            });
        }
        result
    }
}

impl Display for SimplePermission {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.pattern(), self.mask)
    }
}
