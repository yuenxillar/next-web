use std::cmp::{Ordering, PartialEq, PartialOrd};
use std::fmt;
use std::num::ParseFloatError;
use std::ops::{Div, Mul};
use std::str::FromStr;

/// 金额类型，表示货币金额，内部以最小货币单位（如分）存储
/// 使用 `i64` 存储，单位为分
///
/// Amount type representing a monetary amount, stored internally in the smallest currency unit (e.g., cents).
/// Uses `i64` for storage in cents
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Amount {
    cents: i64,
}

// 常量定义
impl Amount {
    /// 零金额常量
    ///
    /// Zero amount constant.
    pub const ZERO: Self = Self { cents: 0 };

    /// 一元金额常量（100 分）
    ///
    /// One unit amount constant (100 cents).
    pub const ONE: Self = Self { cents: 100 };

    /// 最大可能金额常量（i64::MAX 分）
    ///
    /// Maximum possible amount constant (i64::MAX cents).
    pub const MAX: Self = Self { cents: i64::MAX };

    /// 最小可能金额常量（i64::MIN 分）
    ///
    /// Minimum possible amount constant (i64::MIN cents).
    pub const MIN: Self = Self { cents: i64::MIN };

    /// 常用金额常量 (Commonly used amount constants)

    /// 一分金额常量
    ///
    /// One cent amount constant.
    pub const CENT: Self = Self { cents: 1 };

    /// 十元金额常量（1000 分）
    ///
    /// Ten units amount constant (1000 cents).
    pub const TEN: Self = Self { cents: 1000 };

    /// 一百元金额常量（10000 分）
    ///
    /// One hundred units amount constant (10000 cents).
    pub const HUNDRED: Self = Self { cents: 10000 };

    /// 一千元金额常量（100000 分）
    ///
    /// One thousand units amount constant (100000 cents).
    pub const THOUSAND: Self = Self { cents: 100000 };

    /// 一百万元金额常量（1亿分）
    ///
    /// One million units amount constant (100,000,000 cents).
    pub const MILLION: Self = Self { cents: 100_000_000 };

    /// 十亿元金额常量（100亿分）
    ///
    /// One billion units amount constant (10,000,000,000 cents).
    pub const BILLION: Self = Self {
        cents: 10_000_000_000,
    };
}

impl Amount {
    /// 创建一个新的金额
    ///
    /// # 参数
    /// * `amount` - 以元为单位的金额（浮点数）
    ///
    /// # 返回值
    /// * `Ok(Amount)` - 成功创建的金额
    /// * `Err(AmountError::InvalidNumber)` - 如果 `amount` 是 NaN 或无穷大
    /// * `Err(AmountError::Overflow)` - 如果金额乘以100后超出 `i64` 范围
    ///
    /// Creates a new `Amount` instance from a floating-point number representing the amount in units (e.g., dollars).
    ///
    /// # Arguments
    /// * `amount` - The amount in units (e.g., dollars) as a floating-point number.
    ///
    /// # Returns
    /// * `Ok(Amount)` - The created `Amount` instance on success.
    /// * `Err(AmountError::InvalidNumber)` - If `amount` is NaN or infinite.
    /// * `Err(AmountError::Overflow)` - If the amount multiplied by 100 exceeds the `i64` range.
    pub fn new(amount: f64) -> Result<Self, AmountError> {
        if amount.is_nan() || amount.is_infinite() {
            return Err(AmountError::InvalidNumber);
        }

        // 检查溢出
        // Check for overflow
        let amount = amount * 100.00;
        if amount.abs() > i64::MAX as f64 {
            return Err(AmountError::Overflow);
        }

        let cents = amount.round() as i64;
        Ok(Self { cents })
    }

    /// 从分创建金额
    ///
    /// # 参数
    /// * `cents` - 以分为单位的金额（整数）
    ///
    /// # 返回值
    /// 返回一个新的 `Amount` 实例
    ///
    /// Creates a `Amount` instance directly from the number of cents.
    ///
    /// # Arguments
    /// * `cents` - The amount in cents as an integer.
    ///
    /// # Returns
    /// Returns a new `Amount` instance.
    pub const fn from_cents(cents: i64) -> Self {
        Self { cents }
    }

    /// 将两个金额相加
    ///
    /// # 参数
    /// * `&self` - 被加数
    /// * `other` - 加数
    ///
    /// # 返回值
    /// * `Ok(Amount)` - 相加后的金额
    /// * `Err(AmountError::Overflow)` - 如果加法运算导致溢出
    ///
    /// Adds two `Amount` amounts together.
    ///
    /// # Arguments
    /// * `&self` - The augend.
    /// * `other` - The addend.
    ///
    /// # Returns
    /// * `Ok(Amount)` - The sum of the two amounts.
    /// * `Err(AmountError::Overflow)` - If the addition operation results in an overflow.
    pub fn add(&self, other: &Self) -> Result<Self, AmountError> {
        self.cents
            .checked_add(other.cents)
            .map(Self::from_cents)
            .ok_or(AmountError::Overflow)
    }

    /// 从当前金额中减去另一个金额
    ///
    /// # 参数
    /// * `&self` - 被减数
    /// * `other` - 减数
    ///
    /// # 返回值
    /// * `Ok(Amount)` - 相减后的金额
    /// * `Err(AmountError::Overflow)` - 如果减法运算导致溢出
    ///
    /// Subtracts one `Amount` amount from another.
    ///
    /// # Arguments
    /// * `&self` - The minuend.
    /// * `other` - The subtrahend.
    ///
    /// # Returns
    /// * `Ok(Amount)` - The difference of the two amounts.
    /// * `Err(AmountError::Overflow)` - If the subtraction operation results in an overflow.
    pub fn subtract(&self, other: &Self) -> Result<Self, AmountError> {
        self.cents
            .checked_sub(other.cents)
            .map(Self::from_cents)
            .ok_or(AmountError::Overflow)
    }

    /// 将两个金额相乘（不推荐用于货币，通常用金额乘标量）
    ///
    /// # 参数
    /// * `&self` - 第一个乘数
    /// * `other` - 第二个乘数
    ///
    /// # 返回值
    /// * `Ok(Amount)` - 相乘后的金额（结果单位是“分²”）
    /// * `Err(AmountError::Overflow)` - 如果乘法运算导致溢出
    ///
    /// Multiplies two `Amount` amounts together (not commonly used for currency, usually amount * scalar).
    ///
    /// # Arguments
    /// * `&self` - The first multiplicand.
    /// * `other` - The second multiplicand.
    ///
    /// # Returns
    /// * `Ok(Amount)` - The product of the two amounts (result unit is "cents²").
    /// * `Err(AmountError::Overflow)` - If the multiplication operation results in an overflow.
    pub fn multiply(&self, other: &Self) -> Result<Self, AmountError> {
        self.cents
            .checked_mul(other.cents)
            .map(|val| val / 100)
            .map(Self::from_cents)
            .ok_or(AmountError::Overflow)
    }

    /// 将当前金额除以另一个金额（不推荐用于货币，通常用金额除标量）
    ///
    /// # 参数
    /// * `&self` - 被除数
    /// * `other` - 除数
    ///
    /// # 返回值
    /// * `Ok(Amount)` - 相除后的金额（结果单位是“单位”，如美元/美元=1，通常无意义）
    /// * `Err(AmountError::Overflow)` - 如果除法运算导致溢出（如 i64::MIN / -1）
    /// * `Err(AmountError::DivisionByZero)` - 如果除数为零
    ///
    /// Divides one `Amount` amount by another (not commonly used for currency, usually amount / scalar).
    ///
    /// # Arguments
    /// * `&self` - The dividend.
    /// * `other` - The divisor.
    ///
    /// # Returns
    /// * `Ok(Amount)` - The quotient of the two amounts (result unit is dimensionless, e.g., USD/USD=1, often meaningless).
    /// * `Err(AmountError::Overflow)` - If the division operation results in an overflow (e.g., i64::MIN / -1).
    /// * `Err(AmountError::DivisionByZero)` - If the divisor is zero.
    pub fn divide(&self, other: &Self) -> Result<Self, AmountError> {
        if other.cents == 0 {
            return Err(AmountError::DivisionByZero);
        }

        self.cents
            .checked_mul(100)
            .and_then(|val| val.checked_div(other.cents))
            .map(Self::from_cents)
            .ok_or(AmountError::Overflow)
    }

    /// 比较两个金额的大小
    ///
    /// # 参数
    /// * `&self` - 第一个金额
    /// * `other` - 第二个金额
    ///
    /// # 返回值
    /// 返回一个 `Ordering` 枚举，表示 `self` 相对于 `other` 的顺序：
    /// * `Ordering::Less` - `self < other`
    /// * `Ordering::Equal` - `self == other`
    /// * `Ordering::Greater` - `self > other`
    ///
    /// Compares two `Amount` amounts.
    ///
    /// # Arguments
    /// * `&self` - The first amount.
    /// * `other` - The second amount.
    ///
    /// # Returns
    /// Returns an `Ordering` enum indicating the order of `self` relative to `other`:
    /// * `Ordering::Less` - `self < other`
    /// * `Ordering::Equal` - `self == other`
    /// * `Ordering::Greater` - `self > other`
    pub fn compare(&self, other: &Self) -> Ordering {
        self.cents.cmp(&other.cents)
    }

    /// 获取金额（元）
    ///
    /// # 返回值
    /// 以元为单位的金额（浮点数）
    ///
    /// Gets the amount in units (e.g., dollars).
    ///
    /// # Returns
    /// The amount in units as a floating-point number.
    pub fn amount(&self) -> f64 {
        self.cents as f64 / 100.0
    }

    /// 获取金额（分）
    ///
    /// # 返回值
    /// 以分为单位的金额（整数）
    ///
    /// Gets the amount in cents.
    ///
    /// # Returns
    /// The amount in cents as an integer.
    pub const fn cents(&self) -> i64 {
        self.cents
    }

    /// 检查金额是否为零
    ///
    /// # 返回值
    /// 如果金额为零则返回 `true`，否则返回 `false`
    ///
    /// Checks if the amount is zero.
    ///
    /// # Returns
    /// `true` if the amount is zero, `false` otherwise.
    pub const fn is_zero(&self) -> bool {
        self.cents == 0
    }

    /// 检查金额是否为负数
    ///
    /// # 返回值
    /// 如果金额为负数则返回 `true`，否则返回 `false`
    ///
    /// Checks if the amount is negative.
    ///
    /// # Returns
    /// `true` if the amount is negative, `false` otherwise.
    pub const fn is_negative(&self) -> bool {
        self.cents < 0
    }

    /// 检查金额是否为正数
    ///
    /// # 返回值
    /// 如果金额为正数则返回 `true`，否则返回 `false`
    ///
    /// Checks if the amount is positive.
    ///
    /// # Returns
    /// `true` if the amount is positive, `false` otherwise.
    pub const fn is_positive(&self) -> bool {
        self.cents > 0
    }

    /// 取绝对值
    ///
    /// # 返回值
    /// 返回一个新 `Amount` 实例，其金额为当前金额的绝对值
    ///
    /// Takes the absolute value.
    ///
    /// # Returns
    /// A new `Amount` instance with the absolute value of the current amount.
    pub fn abs(&self) -> Self {
        Self {
            cents: self
                .cents
                .checked_abs()
                .expect("amount absolute value overflowed"),
        }
    }

    /// 取反
    ///
    /// # 返回值
    /// 返回一个新 `Amount` 实例，其金额为当前金额的相反数
    ///
    /// Negates the amount.
    ///
    /// # Returns
    /// A new `Amount` instance with the negated amount.
    pub fn neg(&self) -> Self {
        Self {
            cents: self
                .cents
                .checked_neg()
                .expect("amount negation overflowed"),
        }
    }

    /// 带舍入模式的乘法（金额乘以标量）
    ///
    /// # 参数
    /// * `self` - 被乘数（金额）
    /// * `factor` - 乘数（标量，浮点数）
    /// * `mode` - 舍入模式
    ///
    /// # 返回值
    /// * `Ok(Amount)` - 相乘并舍入后的金额
    /// * `Err(AmountError::InvalidNumber)` - 如果 `factor` 是 NaN 或无穷大
    /// * `Err(AmountError::Overflow)` - 如果计算结果超出 `i64` 范围
    ///
    /// Multiplies the `Amount` amount by a scalar factor with specified rounding.
    ///
    /// # Arguments
    /// * `self` - The multiplicand (the amount).
    /// * `factor` - The multiplier (a scalar, floating-point number).
    /// * `mode` - The rounding mode.
    ///
    /// # Returns
    /// * `Ok(Amount)` - The product of the amount and the factor, rounded.
    /// * `Err(AmountError::InvalidNumber)` - If `factor` is NaN or infinite.
    /// * `Err(AmountError::Overflow)` - If the result exceeds the `i64` range.
    pub fn mul_with_rounding(self, factor: f64, mode: RoundingMode) -> Result<Self, AmountError> {
        if factor.is_nan() || factor.is_infinite() {
            return Err(AmountError::InvalidNumber);
        }

        let raw = (self.cents as f64) * factor;
        if raw.abs() > i64::MAX as f64 {
            return Err(AmountError::Overflow);
        }

        let cents = Self::round_f64(raw, mode);
        Ok(Self { cents })
    }

    /// 带舍入模式的除法（金额除以整数）
    ///
    /// # 参数
    /// * `self` - 被除数（金额）
    /// * `divisor` - 除数（整数）
    /// * `mode` - 舍入模式
    ///
    /// # 返回值
    /// * `Ok(Amount)` - 相除并舍入后的金额
    /// * `Err(AmountError::DivisionByZero)` - 如果除数为零
    /// * `Err(AmountError::Overflow)` - 如果计算结果超出 `i64` 范围（如 i64::MIN / -1）
    ///
    /// Divides the `Amount` amount by an integer divisor with specified rounding.
    ///
    /// # Arguments
    /// * `self` - The dividend (the amount).
    /// * `divisor` - The divisor (an integer).
    /// * `mode` - The rounding mode.
    ///
    /// # Returns
    /// * `Ok(Amount)` - The quotient of the amount and the divisor, rounded.
    /// * `Err(AmountError::DivisionByZero)` - If the divisor is zero.
    /// * `Err(AmountError::Overflow)` - If the division results in an overflow (e.g., i64::MIN / -1).
    pub fn div_with_rounding(self, divisor: i64, mode: RoundingMode) -> Result<Self, AmountError> {
        if divisor == 0 {
            return Err(AmountError::DivisionByZero);
        }

        let raw = self.cents as f64 / divisor as f64;
        if raw.abs() > i64::MAX as f64 {
            return Err(AmountError::Overflow);
        }

        let cents = Self::round_f64(raw, mode);
        Ok(Self { cents })
    }

    /// 带舍入模式的除法（金额除以浮点数）
    ///
    /// # 参数
    /// * `self` - 被除数（金额）
    /// * `divisor` - 除数（浮点数）
    /// * `mode` - 舍入模式
    ///
    /// # 返回值
    /// * `Ok(Amount)` - 相除并舍入后的金额
    /// * `Err(AmountError::DivisionByZero)` - 如果除数为零
    /// * `Err(AmountError::InvalidNumber)` - 如果 `divisor` 是 NaN 或无穷大
    /// * `Err(AmountError::Overflow)` - 如果计算结果超出 `i64` 范围
    ///
    /// Divides the `Amount` amount by a floating-point divisor with specified rounding.
    ///
    /// # Arguments
    /// * `self` - The dividend (the amount).
    /// * `divisor` - The divisor (a floating-point number).
    /// * `mode` - The rounding mode.
    ///
    /// # Returns
    /// * `Ok(Amount)` - The quotient of the amount and the divisor, rounded.
    /// * `Err(AmountError::DivisionByZero)` - If the divisor is zero.
    /// * `Err(AmountError::InvalidNumber)` - If `divisor` is NaN or infinite.
    /// * `Err(AmountError::Overflow)` - If the result exceeds the `i64` range.
    pub fn div_f64_with_rounding(
        self,
        divisor: f64,
        mode: RoundingMode,
    ) -> Result<Self, AmountError> {
        if divisor == 0.0 {
            return Err(AmountError::DivisionByZero);
        }
        if divisor.is_nan() || divisor.is_infinite() {
            return Err(AmountError::InvalidNumber);
        }

        let raw = self.cents as f64 / divisor;
        if raw.abs() > i64::MAX as f64 {
            return Err(AmountError::Overflow);
        }

        let cents = Self::round_f64(raw, mode);
        Ok(Self {
            cents: cents as i64,
        }) // 修正：cents 是 f64，需要转换为 i64
    }

    /// 浮点数舍入工具函数
    ///
    /// # 参数
    /// * `value` - 需要舍入的浮点数值
    /// * `mode` - 舍入模式
    ///
    /// # 返回值
    /// 舍入后的整数（`i64`）
    ///
    /// Internal helper function to round a floating-point number according to a specified mode.
    ///
    /// # Arguments
    /// * `value` - The floating-point value to round.
    /// * `mode` - The rounding mode.
    ///
    /// # Returns
    /// The rounded integer value as `i64`.
    fn round_f64(value: f64, mode: RoundingMode) -> i64 {
        let sign = if value.is_sign_negative() {
            -1_i64
        } else {
            1_i64
        };
        let abs_value = value.abs();
        let integer_part = abs_value.floor() as i64;
        let fractional = abs_value - integer_part as f64;
        match mode {
            RoundingMode::Up => sign * abs_value.ceil() as i64,
            RoundingMode::Down => sign * integer_part,
            RoundingMode::Ceiling => value.ceil() as i64,
            RoundingMode::Floor => value.floor() as i64,
            RoundingMode::HalfUp => {
                if fractional >= 0.5 {
                    sign * (integer_part + 1)
                } else {
                    sign * integer_part
                }
            }
            RoundingMode::HalfDown => {
                if fractional > 0.5 {
                    sign * (integer_part + 1)
                } else {
                    sign * integer_part
                }
            }
            RoundingMode::HalfEven => {
                if fractional < 0.5 {
                    sign * integer_part
                } else if fractional > 0.5 {
                    sign * (integer_part + 1)
                } else {
                    // 当小数部分正好是0.5时，向最近的偶数舍入
                    if integer_part % 2 == 0 {
                        sign * integer_part
                    } else {
                        sign * (integer_part + 1)
                    }
                }
            }
            RoundingMode::None => value.trunc() as i64,
        }
    }

    /// 百分比计算
    ///
    /// # 参数
    /// * `self` - 基础金额
    /// * `percent` - 百分比数值（例如，5.0 表示 5%）
    /// * `mode` - 舍入模式
    ///
    /// # 返回值
    /// * `Ok(Amount)` - 计算出的百分比金额
    /// * `Err(AmountError::InvalidNumber)` - 如果 `percent` 是 NaN 或无穷大
    /// * `Err(AmountError::Overflow)` - 如果计算结果超出 `i64` 范围
    ///
    /// Calculates a percentage of the `Amount` amount.
    ///
    /// # Arguments
    /// * `self` - The base amount.
    /// * `percent` - The percentage value (e.g., 5.0 means 5%).
    /// * `mode` - The rounding mode.
    ///
    /// # Returns
    /// * `Ok(Amount)` - The calculated percentage amount.
    /// * `Err(AmountError::InvalidNumber)` - If `percent` is NaN or infinite.
    /// * `Err(AmountError::Overflow)` - If the result exceeds the `i64` range.
    pub fn percentage(self, percent: f64, mode: RoundingMode) -> Result<Self, AmountError> {
        self.mul_with_rounding(percent / 100.0, mode)
    }

    /// Returns the dimensionless ratio between two amounts.
    pub fn ratio(&self, other: &Self) -> Result<f64, AmountError> {
        if other.cents == 0 {
            return Err(AmountError::DivisionByZero);
        }

        Ok(self.cents as f64 / other.cents as f64)
    }

    /// 分配金额（将金额平均分配到多个部分）
    ///
    /// # 参数
    /// * `self` - 要分配的总金额
    /// * `parts` - 要分配的份数
    ///
    /// # 返回值
    /// * `Ok(Vec<Amount>)` - 包含 `parts` 个金额的向量，总和等于原金额
    /// * `Err(AmountError::DistributionError)` - 如果 `parts` 为零或过大
    /// * `Err(AmountError::Overflow)` - 如果计算过程中发生溢出
    ///
    /// Distributes the `Amount` amount equally into a specified number of parts.
    ///
    /// # Arguments
    /// * `self` - The total amount to distribute.
    /// * `parts` - The number of parts to divide into.
    ///
    /// # Returns
    /// * `Ok(Vec<Amount>)` - A vector of `parts` amounts, summing to the original amount.
    /// * `Err(AmountError::DistributionError)` - If `parts` is zero or too large.
    /// * `Err(AmountError::Overflow)` - If an overflow occurs during calculation.
    pub fn distribute(self, parts: usize) -> Result<Vec<Self>, AmountError> {
        if parts == 0 {
            return Err(AmountError::DistributionError(
                "Parts cannot be 0".to_string(),
            ));
        }

        let parts_i64 = i64::try_from(parts)
            .map_err(|_| AmountError::DistributionError("Parts exceed i64 range".to_string()))?;
        let base = self.cents / parts_i64;
        let remainder = self.cents % parts_i64;
        let mut result = vec![Self { cents: base }; parts];

        let remainder_abs = remainder.unsigned_abs() as usize;
        if remainder > 0 {
            for item in result.iter_mut().take(remainder_abs) {
                item.cents += 1;
            }
        } else if remainder < 0 {
            for item in result.iter_mut().take(remainder_abs) {
                item.cents -= 1;
            }
        }

        Ok(result)
    }
}

/// 金额比较结果枚举
///
/// 用于表示两个 `Amount` 值之间的比较关系
///
/// # 变体
/// * `Equal` - 两个金额相等
/// * `Less` - 第一个金额小于第二个金额
/// * `Greater` - 第一个金额大于第二个金额
///
/// Enum representing the result of comparing two `Amount` values.
///
/// # Variants
/// * `Equal` - The two amounts are equal.
/// * `Less` - The first amount is less than the second.
/// * `Greater` - The first amount is greater than the second.
#[derive(PartialEq, Eq)]
pub enum CompareResult {
    Equal,
    Less,
    Greater,
}

// 为 Amount 类型实现乘法运算符
impl Mul<i64> for Amount {
    type Output = Self;

    /// 实现 `Amount * i64`
    ///
    /// # 参数
    /// * `self` - 被乘的金额
    /// * `multiplier` - 乘数（整数）
    ///
    /// # 返回值
    /// 返回一个新的 `Amount` 实例，其金额为原金额乘以 `multiplier`
    ///
    /// # 注意
    /// 此操作**不检查溢出**如果乘法结果超出 `i64` 范围，行为是未定义的（通常是环绕）
    /// 对于可能溢出的场景，建议使用 `mul_with_rounding` 方法
    ///
    /// Implements the `Amount * i64` operation.
    ///
    /// # Arguments
    /// * `self` - The multiplicand (the amount).
    /// * `multiplier` - The multiplier (an integer).
    ///
    /// # Returns
    /// A new `Amount` instance with the amount multiplied by `multiplier`.
    ///
    /// # Note
    /// This operation **does not check for overflow**. If the result exceeds the `i64` range, the behavior is undefined (typically wraps around).
    /// For scenarios where overflow is possible, use the `mul_with_rounding` method instead.
    fn mul(self, multiplier: i64) -> Self {
        Self {
            cents: self
                .cents
                .checked_mul(multiplier)
                .expect("amount multiplication overflowed"),
        }
    }
}

impl Mul<f64> for Amount {
    type Output = Result<Self, AmountError>;

    /// 实现 `Amount * f64`
    ///
    /// # 参数
    /// * `self` - 被乘的金额
    /// * `multiplier` - 乘数（浮点数）
    ///
    /// # 返回值
    /// * `Ok(Amount)` - 相乘并使用默认舍入模式（`HalfUp`）舍入后的金额
    /// * `Err(AmountError::InvalidNumber)` - 如果 `multiplier` 是 NaN 或无穷大
    /// * `Err(AmountError::Overflow)` - 如果计算结果超出 `i64` 范围
    ///
    /// Implements the `Amount * f64` operation.
    ///
    /// # Arguments
    /// * `self` - The multiplicand (the amount).
    /// * `multiplier` - The multiplier (a floating-point number).
    ///
    /// # Returns
    /// * `Ok(Amount)` - The product of the amount and the multiplier, rounded using the default rounding mode (`HalfUp`).
    /// * `Err(AmountError::InvalidNumber)` - If `multiplier` is NaN or infinite.
    /// * `Err(AmountError::Overflow)` - If the result exceeds the `i64` range.
    fn mul(self, multiplier: f64) -> Result<Self, AmountError> {
        self.mul_with_rounding(multiplier, RoundingMode::default())
    }
}

// 为 Amount 类型实现除法运算符
impl Div<i64> for Amount {
    type Output = Result<Self, AmountError>;

    /// 实现 `Amount / i64`
    ///
    /// # 参数
    /// * `self` - 被除的金额
    /// * `divisor` - 除数（整数）
    ///
    /// # 返回值
    /// * `Ok(Amount)` - 相除并使用默认舍入模式（`HalfUp`）舍入后的金额
    /// * `Err(AmountError::DivisionByZero)` - 如果 `divisor` 为零
    /// * `Err(AmountError::Overflow)` - 如果除法运算导致溢出（如 i64::MIN / -1）或结果超出 `i64` 范围
    ///
    /// Implements the `Amount / i64` operation.
    ///
    /// # Arguments
    /// * `self` - The dividend (the amount).
    /// * `divisor` - The divisor (an integer).
    ///
    /// # Returns
    /// * `Ok(Amount)` - The quotient of the amount and the divisor, rounded using the default rounding mode (`HalfUp`).
    /// * `Err(AmountError::DivisionByZero)` - If `divisor` is zero.
    /// * `Err(AmountError::Overflow)` - If the division operation results in an overflow (e.g., i64::MIN / -1) or the result exceeds the `i64` range.
    fn div(self, divisor: i64) -> Result<Self, AmountError> {
        self.div_with_rounding(divisor, RoundingMode::default())
    }
}

impl Div<f64> for Amount {
    type Output = Result<Self, AmountError>;

    /// 实现 `Amount / f64`
    ///
    /// # 参数
    /// * `self` - 被除的金额
    /// * `divisor` - 除数（浮点数）
    ///
    /// # 返回值
    /// * `Ok(Amount)` - 相除并使用默认舍入模式（`HalfUp`）舍入后的金额
    /// * `Err(AmountError::DivisionByZero)` - 如果 `divisor` 为零
    /// * `Err(AmountError::InvalidNumber)` - 如果 `divisor` 是 NaN 或无穷大
    /// * `Err(AmountError::Overflow)` - 如果计算结果超出 `i64` 范围
    ///
    /// Implements the `Amount / f64` operation.
    ///
    /// # Arguments
    /// * `self` - The dividend (the amount).
    /// * `divisor` - The divisor (a floating-point number).
    ///
    /// # Returns
    /// * `Ok(Amount)` - The quotient of the amount and the divisor, rounded using the default rounding mode (`HalfUp`).
    /// * `Err(AmountError::DivisionByZero)` - If `divisor` is zero.
    /// * `Err(AmountError::InvalidNumber)` - If `divisor` is NaN or infinite.
    /// * `Err(AmountError::Overflow)` - If the result exceeds the `i64` range.
    fn div(self, divisor: f64) -> Result<Self, AmountError> {
        self.div_f64_with_rounding(divisor, RoundingMode::default())
    }
}

// 从整数类型转换为 Amount
macro_rules! impl_from_integer {
    ($($t:ty),*) => {
        $(
            /// 为指定的整数类型实现 `From` trait
            ///
            /// # 说明
            /// 将整数 `value` 解释为“元”，并将其转换为以“分”为单位的 `Amount`
            /// 例如，`i32::from(10)` 会创建一个 `Amount` 实例，表示 10.00 元
            ///
            /// # 参数
            /// * `value` - 要转换的整数值（表示元）
            ///
            /// # 返回值
            /// 返回一个 `Amount` 实例
            ///
            /// Implements the `From` trait for a specified integer type.
            ///
            /// # Description
            /// Interprets the integer `value` as units (e.g., dollars) and converts it to `Amount` in cents.
            /// For example, `Amount::from(10i32)` creates a `Amount` instance representing 10.00 units.
            ///
            /// # Arguments
            /// * `value` - The integer value to convert (representing units).
            ///
            /// # Returns
            /// Returns a `Amount` instance.
            impl From<$t> for Amount {
                fn from(value: $t) -> Self {
                    Self::from_cents(value as i64 * 100)
                }
            }
        )*
    };
}

// 为多种整数类型生成 From 实现
impl_from_integer!(u8, u16, u32, u64, usize, i8, i16, i32, i64, isize);

// 从浮点数类型尝试转换为 Amount
macro_rules! impl_try_from_float {
    ($($t:ty),*) => {
        $(
            /// 为指定的浮点数类型实现 `TryFrom` trait
            ///
            /// # 说明
            /// 尝试将浮点数 `value` 转换为 `Amount`
            /// 转换过程使用 `Amount::new` 方法，因此遵循相同的规则（检查 NaN/无穷大，舍入到分）
            ///
            /// # 参数
            /// * `value` - 要转换的浮点数值
            ///
            /// # 返回值
            /// * `Ok(Amount)` - 转换成功
            /// * `Err(AmountError::InvalidNumber)` - 如果 `value` 是 NaN 或无穷大
            /// * `Err(AmountError::Overflow)` - 如果 `value` 的绝对值过大，导致乘以100后超出 `i64` 范围
            ///
            /// Implements the `TryFrom` trait for a specified floating-point type.
            ///
            /// # Description
            /// Attempts to convert the floating-point `value` into `Amount`.
            /// The conversion uses the `Amount::new` method, so it follows the same rules (checks for NaN/infinity, rounds to cents).
            ///
            /// # Arguments
            /// * `value` - The floating-point value to convert.
            ///
            /// # Returns
            /// * `Ok(Amount)` - On successful conversion.
            /// * `Err(AmountError::InvalidNumber)` - If `value` is NaN or infinite.
            /// * `Err(AmountError::Overflow)` - If the absolute value of `value` is too large, causing overflow when multiplied by 100.
            impl TryFrom<$t> for Amount {
                type Error = AmountError;

                fn try_from(value: $t) -> Result<Self, Self::Error> {
                    Self::new(value as f64)
                }
            }
        )*
    };
}

// 为 f32 和 f64 生成 TryFrom 实现
impl_try_from_float!(f32, f64);

// 从字符串尝试转换为 Amount
impl TryFrom<String> for Amount {
    type Error = AmountError;

    /// 从 `String` 尝试转换为 `Amount`
    ///
    /// # 参数
    /// * `value` - 包含金额的字符串
    ///
    /// # 返回值
    /// 返回 `value.parse()` 的结果，遵循 `FromStr` 的解析规则
    ///
    /// Attempts to convert a `String` into `Amount`.
    ///
    /// # Arguments
    /// * `value` - The string containing the amount.
    ///
    /// # Returns
    /// Returns the result of `value.parse()`, following the parsing rules defined in `FromStr`.
    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl TryFrom<&str> for Amount {
    type Error = AmountError;

    /// 从 `&str` 尝试转换为 `Amount`
    ///
    /// # 参数
    /// * `value` - 包含金额的字符串切片
    ///
    /// # 返回值
    /// 返回 `value.parse()` 的结果，遵循 `FromStr` 的解析规则
    ///
    /// Attempts to convert a `&str` into `Amount`.
    ///
    /// # Arguments
    /// * `value` - The string slice containing the amount.
    ///
    /// # Returns
    /// Returns the result of `value.parse()`, following the parsing rules defined in `FromStr`.
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl TryFrom<Box<str>> for Amount {
    type Error = AmountError;

    /// 从 `Box<str>` 尝试转换为 `Amount`
    ///
    /// # 参数
    /// * `value` - 包含金额的堆分配字符串
    ///
    /// # 返回值
    /// 返回 `value.parse()` 的结果，遵循 `FromStr` 的解析规则
    ///
    /// Attempts to convert a `Box<str>` into `Amount`.
    ///
    /// # Arguments
    /// * `value` - The heap-allocated string containing the amount.
    ///
    /// # Returns
    /// Returns the result of `value.parse()`, following the parsing rules defined in `FromStr`.
    fn try_from(value: Box<str>) -> Result<Self, Self::Error> {
        value.parse()
    }
}

// 从字符串解析为 Amount
impl FromStr for Amount {
    type Err = AmountError;

    /// 从字符串解析 `Amount`
    ///
    /// # 参数
    /// * `amount` - 表示金额的字符串
    ///
    /// # 返回值
    /// * `Ok(Amount)` - 解析成功
    /// * `Err(AmountError::ParseError)` - 如果字符串无法被解析为有效的数字
    /// * `Err(AmountError::InvalidNumber)` - 如果解析出的数字是 NaN 或无穷大
    /// * `Err(AmountError::Overflow)` - 如果解析出的数字过大，导致乘以100后超出 `i64` 范围
    ///
    /// # 解析逻辑
    /// 1.  去除字符串首尾空白
    /// 2.  如果为空字符串，返回 `Amount::ZERO`
    /// 3.  优先尝试使用 `f64::parse` 解析（支持小数、科学计数法等）
    /// 4.  如果 `f64` 解析失败，则尝试使用 `i64::parse` 解析（仅整数）
    /// 5.  如果两者都失败，则返回解析错误
    ///
    /// Implements parsing `Amount` from a string.
    ///
    /// # Arguments
    /// * `amount` - The string representing the amount.
    ///
    /// # Returns
    /// * `Ok(Amount)` - On successful parsing.
    /// * `Err(AmountError::ParseError)` - If the string cannot be parsed into a valid number.
    /// * `Err(AmountError::InvalidNumber)` - If the parsed number is NaN or infinite.
    /// * `Err(AmountError::Overflow)` - If the parsed number is too large, causing overflow when multiplied by 100.
    ///
    /// # Parsing Logic
    /// 1.  Trims whitespace from the start and end of the string.
    /// 2.  If the string is empty after trimming, returns `Amount::ZERO`.
    /// 3.  First attempts to parse using `f64::parse` (supports decimals, scientific notation, etc.).
    /// 4.  If `f64` parsing fails, attempts to parse using `i64::parse` (integers only).
    /// 5.  If both attempts fail, returns a parse error.
    fn from_str(amount: &str) -> Result<Self, Self::Err> {
        let amount = amount.trim();
        if amount.is_empty() {
            return Ok(Self::ZERO);
        }

        // 优先尝试解析为 f64 (支持小数)
        if let Ok(num) = amount.parse::<f64>() {
            Self::new(num)
        } else if let Ok(num) = amount.parse::<i64>() {
            // 再尝试解析为 i64 (仅整数)
            Ok(Self::from(num))
        } else {
            Err(AmountError::ParseError(format!(
                "无法解析金额: '{}'",
                amount
            )))
        }
    }
}

impl std::iter::Sum for Amount {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::ZERO, |acc, x| {
            acc.add(&x).expect("amount sum overflowed")
        })
    }
}

impl<'a> std::iter::Sum<&'a Amount> for Amount {
    fn sum<I: Iterator<Item = &'a Amount>>(iter: I) -> Self {
        iter.fold(Self::ZERO, |acc, x| {
            acc.add(x).expect("amount sum overflowed")
        })
    }
}

impl ToString for Amount {
    /// 将 `Amount` 实例格式化为字符串
    ///
    /// # 返回值
    /// 返回一个格式化为 `X.XX` 形式的字符串，表示金额（元），保留两位小数
    /// 例如，`Amount::from_cents(1234)` 会返回 `"12.34"`
    ///
    /// Formats the `Amount` instance into a string.
    ///
    /// # Returns
    /// Returns a string formatted as `X.XX`, representing the amount in units with two decimal places.
    /// For example, `Amount::from_cents(1234)` returns `"12.34"`.
    fn to_string(&self) -> String {
        let cents = self.cents as i128;
        let sign = if cents < 0 { "-" } else { "" };
        let abs = cents.abs();
        let units = abs / 100;
        let fractional = abs % 100;
        format!("{sign}{units}.{fractional:02}")
    }
}

// impl fmt::Display for Amount {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         let amount = self.amount();
//         if amount.fract() == 0.0 {
//             write!(f, "{:.0}", amount)
//         } else {
//             write!(f, "{:.2}", amount)
//         }
//     }
// }

/// 舍入模式，用于控制金额计算中的舍入行为
///
/// Rounding modes, used to control rounding behavior in monetary calculations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoundingMode {
    /// 向远离零的方向舍入（绝对值变大）
    ///
    /// Rounds away from zero (increases absolute value).
    Up,
    /// 向零方向舍入（绝对值变小或不变）
    ///
    /// Rounds towards zero (decreases or maintains absolute value).
    Down,
    /// 向正无穷方向舍入（正数向上，负数向零）
    ///
    /// Rounds towards positive infinity (up for positive, towards zero for negative).
    Ceiling,
    /// 向负无穷方向舍入（正数向零，负数向下）
    ///
    /// Rounds towards negative infinity (towards zero for positive, down for negative).
    Floor,
    /// 四舍五入（5 及以上进位）
    ///
    /// Rounds half away from zero (5 or above rounds up).
    HalfUp,
    /// 半舍入向零（5 时向零舍入，6 及以上时远离零进位）
    ///
    /// Rounds half towards zero (5 rounds towards zero, 6 or above rounds away from zero).
    HalfDown,
    /// 银行家舍入法（四舍六入，五取偶）
    ///
    /// Rounds half to even ("banker's rounding": 4 down, 6 up, 5 to nearest even).
    HalfEven,
    /// 未指定模式（默认）
    None,
}

impl Default for RoundingMode {
    /// 默认舍入模式为 `HalfUp`（四舍五入）
    ///
    /// The default rounding mode is `HalfUp` (round half up).
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
            Self::None => write!(f, "None"),
        }
    }
}

/// 金额错误类型
///
/// Amount error types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AmountError {
    /// 无效的数值（NaN 或无穷大）
    ///
    /// Invalid number (NaN or infinite).
    InvalidNumber,
    /// 除零错误
    ///
    /// Division by zero error.
    DivisionByZero,
    /// 解析错误，附带错误信息
    ///
    /// Parsing error, with associated message.
    ParseError(String),
    /// 溢出错误（运算结果超出 i64 范围）
    ///
    /// Overflow error (result exceeds i64 range).
    Overflow,
    /// 分配错误，附带错误信息（例如，无法精确分配时）
    ///
    /// Distribution error, with associated message (e.g., when exact distribution is impossible).
    DistributionError(String),
}

// 为 AmountError 实现标准 trait
impl fmt::Display for AmountError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidNumber => write!(f, "金额不能是 NaN 或无穷大"),
            Self::DivisionByZero => write!(f, "除数不能为零"),
            Self::ParseError(s) => write!(f, "解析错误: {}", s),
            Self::Overflow => write!(f, "金额溢出"),
            Self::DistributionError(s) => write!(f, "分配错误: {}", s),
        }
    }
}

impl std::error::Error for AmountError {}

impl From<ParseFloatError> for AmountError {
    /// 将 `ParseFloatError` 转换为 `AmountError::ParseError`
    /// Converts a `ParseFloatError` into `AmountError::ParseError`.
    fn from(err: ParseFloatError) -> Self {
        Self::ParseError(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_operations() -> Result<(), AmountError> {
        let m1 = Amount::new(10.50)?;
        let m2 = Amount::new(5.25)?;

        assert_eq!((m1.add(&m2)?).amount(), 15.75);
        assert_eq!((m1.subtract(&m2)?).amount(), 5.25);
        assert_eq!((m1 * 2).amount(), 21.00);
        assert_eq!((m1 * 1.5)?.amount(), 15.75);

        assert_eq!(
            Amount::from_str("1.75")?.multiply(&Amount::from_str("6.0")?)?,
            m1
        );
        assert_eq!(
            Amount::from_str("0.02")?.multiply(&Amount::from_str("11.33")?)?,
            Amount::new(0.22)?
        );
        Ok(())
    }

    #[test]
    fn test_amount_divide_and_ratio() -> Result<(), AmountError> {
        let total = Amount::new(10.0)?;
        let unit = Amount::new(2.5)?;

        assert_eq!(total.divide(&unit)?.to_string(), "4.00");
        assert_eq!(total.ratio(&unit)?, 4.0);
        Ok(())
    }

    #[test]
    fn test_rounding_modes() -> Result<(), AmountError> {
        let amount = Amount::from_cents(1005);
        assert_eq!(
            amount
                .div_with_rounding(10, RoundingMode::HalfUp)?
                .to_string(),
            "1.01"
        );
        assert_eq!(
            amount
                .div_with_rounding(10, RoundingMode::Floor)?
                .to_string(),
            "1.00"
        );
        assert_eq!(
            amount
                .div_with_rounding(-10, RoundingMode::Ceiling)?
                .to_string(),
            "-1.00"
        );
        Ok(())
    }

    #[test]
    fn test_distribute() -> Result<(), AmountError> {
        let amount = Amount::new(10.0)?;
        let parts = amount.distribute(5)?;
        assert_eq!(parts.len(), 5);
        for part in &parts {
            assert_eq!(part.amount(), 2.0);
        }
        assert_eq!(parts.iter().sum::<Amount>(), amount);

        let amount = Amount::new(10.0)?;
        let parts = amount.distribute(2)?;
        assert_eq!(parts.iter().sum::<Amount>(), amount);

        let amount = Amount::new(1.0)?;
        let parts = amount.distribute(4)?;
        assert_eq!(parts.iter().sum::<Amount>(), amount);
        assert_eq!(
            parts
                .iter()
                .map(|item| item.to_string())
                .collect::<Vec<_>>(),
            vec!["0.25", "0.25", "0.25", "0.25"]
        );

        let uneven = Amount::from_cents(10).distribute(3)?;
        assert_eq!(
            uneven.iter().map(|item| item.cents()).collect::<Vec<_>>(),
            vec![4, 3, 3]
        );

        let negative = Amount::from_cents(-10).distribute(3)?;
        assert_eq!(
            negative.iter().map(|item| item.cents()).collect::<Vec<_>>(),
            vec![-4, -3, -3]
        );

        Ok(())
    }

    #[test]
    fn test_percentage() -> Result<(), AmountError> {
        let amount = Amount::new(100.0)?;
        let ten_percent = amount.percentage(10.0, RoundingMode::HalfUp)?;

        assert_eq!(ten_percent.amount(), 10.0);
        Ok(())
    }

    #[test]
    fn test_constants() {
        assert_eq!(Amount::ZERO.amount(), 0.0);
        assert_eq!(Amount::ONE.amount(), 1.0);
        assert_eq!(Amount::CENT.amount(), 0.01);
    }

    #[test]
    fn test_error_handling() {
        assert!(Amount::new(f64::NAN).is_err());
        assert!(Amount::new(f64::INFINITY).is_err());
        assert!(Amount::ZERO
            .div_f64_with_rounding(0.0, RoundingMode::HalfUp)
            .is_err());
    }

    #[test]
    fn test_default_rounding_mode_is_half_up() -> Result<(), AmountError> {
        assert_eq!((Amount::from_cents(1055) / 10_i64)?.to_string(), "1.06");
        assert_eq!((Amount::from_cents(-1055) / 10_i64)?.to_string(), "-1.06");
        Ok(())
    }

    #[test]
    fn test_to_string_uses_integer_formatting() {
        assert_eq!(Amount::from_cents(1234).to_string(), "12.34");
        assert_eq!(Amount::from_cents(-1234).to_string(), "-12.34");
    }
}
