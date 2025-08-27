use std::ops::{Add, Sub, Mul, Div};
use std::cmp::{PartialEq, PartialOrd, Ordering};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Money {
    cents: i64,
}

// 常量定义
impl Money {
    pub const ZERO: Self = Self { cents: 0 };
    pub const ONE: Self = Self { cents: 100 };
    pub const MAX: Self = Self { cents: i64::MAX };
    pub const MIN: Self = Self { cents: i64::MIN };
    
    // 常用金额常量
    pub const CENT: Self = Self { cents: 1 };
    pub const TEN: Self = Self { cents: 1000 };
    pub const HUNDRED: Self = Self { cents: 10000 };
}

impl Money {
    /// 创建新的金额（单位：元）
    pub fn new(amount: f64) -> Result<Self, String> {
        if amount.is_nan() || amount.is_infinite() {
            return Err("金额不能是 NaN 或无穷大".to_string());
        }
        
        // 更精确的浮点数处理
        let cents = (amount * 100.0).round() as i64;
        Ok(Self { cents })
    }
    
    /// 从分创建金额
    pub const fn from_cents(cents: i64) -> Self {
        Self { cents }
    }
    
    /// 获取金额（元）
    pub fn amount(&self) -> f64 {
        self.cents as f64 / 100.0
    }
    
    /// 获取金额（分）
    pub const fn cents(&self) -> i64 {
        self.cents
    }
    
    /// 检查金额是否为零
    pub const fn is_zero(&self) -> bool {
        self.cents == 0
    }
    
    /// 检查金额是否为负数
    pub const fn is_negative(&self) -> bool {
        self.cents < 0
    }
    
    /// 检查金额是否为正数
    pub const fn is_positive(&self) -> bool {
        self.cents > 0
    }
    
    /// 取绝对值
    pub const fn abs(&self) -> Self {
        Self { cents: self.cents.abs() }
    }
    
    /// 取反
    pub const fn neg(&self) -> Self {
        Self { cents: -self.cents }
    }
    
    /// 带舍入模式的乘法
    pub fn mul_with_rounding(self, factor: f64, mode: RoundingMode) -> Result<Self, String> {
        if factor.is_nan() || factor.is_infinite() {
            return Err("乘数不能是 NaN 或无穷大".to_string());
        }

        let raw = self.cents as f64 * factor;
        let cents = Self::round_f64(raw, mode);
        Ok(Self { cents })
    }

    /// 带舍入模式的除法（除以整数）
    pub fn div_with_rounding(self, divisor: i64, mode: RoundingMode) -> Result<Self, String> {
        if divisor == 0 {
            return Err("除数不能为零".to_string());
        }
        let raw = self.cents as f64 / divisor as f64;
        let cents = Self::round_f64(raw, mode);
        Ok(Self { cents })
    }
    
    /// 带舍入模式的除法（除以浮点数）
    pub fn div_f64_with_rounding(self, divisor: f64, mode: RoundingMode) -> Result<Self, String> {
        if divisor == 0.0 {
            return Err("除数不能为零".to_string());
        }
        if divisor.is_nan() || divisor.is_infinite() {
            return Err("除数不能是 NaN 或无穷大".to_string());
        }
        
        let raw = self.cents as f64 / divisor;
        let cents = Self::round_f64(raw, mode);
        Ok(Self { cents })
    }
    
    /// 浮点数舍入工具函数
    fn round_f64(value: f64, mode: RoundingMode) -> i64 {
        match mode {
            RoundingMode::Up => (value.signum() * value.abs().ceil()) as i64,
            RoundingMode::Down => value.trunc() as i64,
            RoundingMode::Ceiling => value.ceil() as i64,
            RoundingMode::Floor => value.floor() as i64,
            RoundingMode::HalfUp => value.round() as i64,
            RoundingMode::HalfDown => {
                let abs_value = value.abs();
                let fractional = abs_value - abs_value.floor();
                if fractional < 0.5 {
                    value.floor() as i64
                } else if fractional > 0.5 {
                    value.ceil() as i64
                } else {
                    value.trunc() as i64
                }
            }
            RoundingMode::HalfEven => {
                // Banker's rounding
                let abs_value = value.abs();
                let integer_part = abs_value.floor();
                let fractional = abs_value - integer_part;
                
                if fractional < 0.5 {
                    value.floor() as i64
                } else if fractional > 0.5 {
                    value.ceil() as i64
                } else {
                    // 当小数部分正好是0.5时，向最近的偶数舍入
                    if integer_part as i64 % 2 == 0 {
                        value.floor() as i64
                    } else {
                        value.ceil() as i64
                    }
                }
            }
        }
    }
    
    /// 百分比计算
    pub fn percentage(self, percent: f64, mode: RoundingMode) -> Result<Self, String> {
        self.mul_with_rounding(percent / 100.0, mode)
    }
    
    /// 分配金额（将金额平均分配到多个部分）
    pub fn distribute(self, parts: usize, mode: RoundingMode) -> Result<Vec<Self>, String> {
        if parts == 0 {
            return Err("部分数不能为零".to_string());
        }
        
        let part_cents = Self::round_f64(self.cents as f64 / parts as f64, mode);
        let mut result = vec![Self::from_cents(part_cents); parts];
        
        // 处理舍入误差
        let total_distributed = part_cents * parts as i64;
        let difference = self.cents - total_distributed;
        
        if difference != 0 {
            // 将误差分配到前面的部分
            let adjustment = if difference > 0 { 1 } else { -1 };
            for i in 0..difference.abs() as usize {
                if i < parts {
                    result[i] = Self::from_cents(result[i].cents + adjustment);
                }
            }
        }
        
        Ok(result)
    }
}

// 基本算术运算
impl Add for Money {
    type Output = Self;
    
    fn add(self, other: Self) -> Self {
        Self { cents: self.cents + other.cents }
    }
}

impl Sub for Money {
    type Output = Self;
    
    fn sub(self, other: Self) -> Self {
        Self { cents: self.cents - other.cents }
    }
}

// 乘法运算
impl Mul<i64> for Money {
    type Output = Self;
    
    fn mul(self, multiplier: i64) -> Self {
        Self { cents: self.cents * multiplier }
    }
}

impl Mul<f64> for Money {
    type Output = Result<Self, String>;
    
    fn mul(self, multiplier: f64) -> Result<Self, String> {
        self.mul_with_rounding(multiplier, RoundingMode::default())
    }
}

// 除法运算
impl Div<i64> for Money {
    type Output = Result<Self, String>;
    
    fn div(self, divisor: i64) -> Result<Self, String> {
        self.div_with_rounding(divisor, RoundingMode::default())
    }
}

impl Div<f64> for Money {
    type Output = Result<Self, String>;
    
    fn div(self, divisor: f64) -> Result<Self, String> {
        self.div_f64_with_rounding(divisor, RoundingMode::default())
    }
}

// From 实现优化
macro_rules! impl_from_integer {
    ($($t:ty),*) => {
        $(
            impl From<$t> for Money {
                fn from(value: $t) -> Self {
                    Self::from_cents(value as i64 * 100)
                }
            }
        )*
    };
}

impl_from_integer!(u8, u16, u32, u64, usize, i8, i16, i32, i64, isize);

// TryFrom 浮点数
macro_rules! impl_try_from_float {
    ($($t:ty),*) => {
        $(
            impl TryFrom<$t> for Money {
                type Error = String;
                
                fn try_from(value: $t) -> Result<Self, Self::Error> {
                    Self::new(value as f64)
                }
            }
        )*
    };
}

impl_try_from_float!(f32, f64);

impl TryFrom<String> for Money {
    type Error = String;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl TryFrom<&str> for Money {
    type Error = String;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse() 
    }
}

impl TryFrom<Box<str>> for Money {
    type Error = String;
    fn try_from(value: Box<str>) -> Result<Self, Self::Error> {
        value.parse() 
    }
}

// 解析字符串优化
impl std::str::FromStr for Money {
    type Err = String;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if s.is_empty() {
            return Ok(Self::ZERO);
        }
        // 移除货币符号
        let s = s.trim_start_matches(|c: char| c == '¥' || c == '￥' || c == '$' || c == '€' || c == '£');
        
        // 解析数字
        if let Ok(num) = s.parse::<f64>() {
            Self::new(num)
        } else if let Ok(num) = s.parse::<i64>() {
            Ok(Self::from(num))
        } else {
            Err(format!("无法解析金额: '{}'", s))
        }
    }
}

// 格式化显示优化
impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let amount = self.amount();
        if amount.fract() == 0.0 {
            write!(f, "¥{:.0}", amount)
        } else {
            write!(f, "¥{:.2}", amount)
        }
    }
}

// 舍入模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoundingMode {
    Up,        // 向远离零的方向舍入
    Down,      // 向零方向舍入
    Ceiling,   // 向正无穷方向舍入
    Floor,     // 向负无穷方向舍入
    HalfUp,    // 四舍五入
    HalfDown,  // 五舍六入
    HalfEven,  // 银行家舍入法
}

impl Default for RoundingMode {
    fn default() -> Self {
        Self::HalfUp
    }
}

impl fmt::Display for RoundingMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Up => write!(f, "Up"),
            Self::Down => write!(f, "Down"),
            Self::Ceiling => write!(f, "Ceiling"),
            Self::Floor => write!(f, "Floor"),
            Self::HalfUp => write!(f, "HalfUp"),
            Self::HalfDown => write!(f, "HalfDown"),
            Self::HalfEven => write!(f, "HalfEven"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_operations() {
        let m1 = Money::new(10.50).unwrap();
        let m2 = Money::new(5.25).unwrap();
        
        assert_eq!((m1 + m2).amount(), 15.75);
        assert_eq!((m1 - m2).amount(), 5.25);
        assert_eq!((m1 * 2).amount(), 21.00);
        assert_eq!((m1 * 1.5).unwrap().amount(), 15.75);
    }

    #[test]
    fn test_rounding_modes() {
        let money = Money::new(10.555).unwrap();
        
        assert_eq!(money.mul_with_rounding(1.0, RoundingMode::HalfUp).unwrap().amount(), 10.56);
        assert_eq!(money.mul_with_rounding(1.0, RoundingMode::Floor).unwrap().amount(), 10.55);
        assert_eq!(money.mul_with_rounding(1.0, RoundingMode::Ceiling).unwrap().amount(), 10.56);
    }

    #[test]
    fn test_distribution() {
        let total = Money::new(10.00).unwrap();
        let parts = total.distribute(3, RoundingMode::HalfUp).unwrap();
        
        assert_eq!(parts.len(), 3);
        let sum: Money = parts.iter().copied().sum();
        assert_eq!(sum, total);
    }

    #[test]
    fn test_percentage() {
        let money = Money::new(100.0).unwrap();
        let ten_percent = money.percentage(10.0, RoundingMode::HalfUp).unwrap();
        
        assert_eq!(ten_percent.amount(), 10.0);
    }

    #[test]
    fn test_constants() {
        assert_eq!(Money::ZERO.amount(), 0.0);
        assert_eq!(Money::ONE.amount(), 1.0);
        assert_eq!(Money::CENT.amount(), 0.01);
    }
}

// 为迭代器实现 sum
impl std::iter::Sum for Money {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::ZERO, |acc, x| acc + x)
    }
}

// 为 Option<Money> 提供便利方法
impl Money {
    pub fn unwrap_or_zero(option: Option<Self>) -> Self {
        option.unwrap_or(Self::ZERO)
    }
    
    pub fn option_new(amount: f64) -> Option<Self> {
        Self::new(amount).ok()
    }
}