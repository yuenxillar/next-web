use std::cmp::{Ordering, PartialEq, PartialOrd};
use std::fmt;
use std::num::ParseFloatError;
use std::ops::{Div, Mul};
use std::str::FromStr;

/// Amount type representing a monetary amount, stored internally in the smallest currency unit (e.g., cents).
/// Uses `i64` for storage in cents
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Amount {
    cents: i64,
}

impl Amount {
    /// Zero amount constant.
    pub const ZERO: Self = Self { cents: 0 };

    /// One unit amount constant (100 cents).
    pub const ONE: Self = Self { cents: 100 };

    /// Maximum possible amount constant (i64::MAX cents).
    pub const MAX: Self = Self { cents: i64::MAX };

    /// Minimum possible amount constant (i64::MIN cents).
    pub const MIN: Self = Self { cents: i64::MIN };

    /// One cent amount constant.
    pub const CENT: Self = Self { cents: 1 };

    /// Ten units amount constant (1000 cents).
    pub const TEN: Self = Self { cents: 1000 };

    /// One hundred units amount constant (10000 cents).
    pub const HUNDRED: Self = Self { cents: 10000 };

    /// One thousand units amount constant (100000 cents).
    pub const THOUSAND: Self = Self { cents: 100000 };

    /// One million units amount constant (100,000,000 cents).
    pub const MILLION: Self = Self { cents: 100_000_000 };

    /// One billion units amount constant (10,000,000,000 cents).
    pub const BILLION: Self = Self {
        cents: 10_000_000_000,
    };
}

impl Amount {
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

        // Check for overflow
        let amount = amount * 100.00;
        if amount.abs() > i64::MAX as f64 {
            return Err(AmountError::Overflow);
        }

        let cents = amount.round() as i64;
        Ok(Self { cents })
    }

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

    /// Gets the amount in units (e.g., dollars).
    ///
    /// # Returns
    /// The amount in units as a floating-point number.
    pub fn amount(&self) -> f64 {
        self.cents as f64 / 100.0
    }

    /// Gets the amount in cents.
    ///
    /// # Returns
    /// The amount in cents as an integer.
    pub const fn cents(&self) -> i64 {
        self.cents
    }

    /// Checks if the amount is zero.
    ///
    /// # Returns
    /// `true` if the amount is zero, `false` otherwise.
    pub const fn is_zero(&self) -> bool {
        self.cents == 0
    }

    /// Checks if the amount is negative.
    ///
    /// # Returns
    /// `true` if the amount is negative, `false` otherwise.
    pub const fn is_negative(&self) -> bool {
        self.cents < 0
    }

    /// Checks if the amount is positive.
    ///
    /// # Returns
    /// `true` if the amount is positive, `false` otherwise.
    pub const fn is_positive(&self) -> bool {
        self.cents > 0
    }

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
        })
    }

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

impl Mul<i64> for Amount {
    type Output = Self;

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

impl Div<i64> for Amount {
    type Output = Result<Self, AmountError>;

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

macro_rules! impl_from_integer {
    ($($t:ty),*) => {
        $(

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

impl_from_integer!(u8, u16, u32, u64, usize, i8, i16, i32, i64, isize);

macro_rules! impl_try_from_float {
    ($($t:ty),*) => {
        $(
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

impl_try_from_float!(f32, f64);

impl TryFrom<String> for Amount {
    type Error = AmountError;

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

impl FromStr for Amount {
    type Err = AmountError;

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

        if let Ok(num) = amount.parse::<f64>() {
            Self::new(num)
        } else if let Ok(num) = amount.parse::<i64>() {
            Ok(Self::from(num))
        } else {
            Err(AmountError::ParseError(format!(
                "Unable to parse amount: '{}'",
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

impl fmt::Display for Amount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let cents = self.cents as i128;
        let sign = if cents < 0 { "-" } else { "" };
        let abs = cents.abs();
        let units = abs / 100;
        let fractional = abs % 100;
        write!(f, "{sign}{units}.{fractional:02}")
    }
}

/// Rounding modes, used to control rounding behavior in monetary calculations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoundingMode {
    /// Rounds away from zero (increases absolute value).
    Up,

    /// Rounds towards zero (decreases or maintains absolute value).
    Down,

    /// Rounds towards positive infinity (up for positive, towards zero for negative).
    Ceiling,

    /// Rounds towards negative infinity (towards zero for positive, down for negative).
    Floor,

    /// Rounds half away from zero (5 or above rounds up).
    HalfUp,

    /// Rounds half towards zero (5 rounds towards zero, 6 or above rounds away from zero).
    HalfDown,

    /// Rounds half to even ("banker's rounding": 4 down, 6 up, 5 to nearest even).
    HalfEven,

    /// Unspecified mode (default)
    None,
}

impl Default for RoundingMode {
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

/// Amount error types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AmountError {
    /// Invalid number (NaN or infinite).
    InvalidNumber,

    /// Division by zero error.
    DivisionByZero,

    /// Parsing error, with associated message.
    ParseError(String),

    /// Overflow error (result exceeds i64 range).
    Overflow,

    /// Distribution error, with associated message (e.g., when exact distribution is impossible).
    DistributionError(String),
}

impl fmt::Display for AmountError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidNumber => write!(f, "Amount cannot be NaN or infinity"),
            Self::DivisionByZero => write!(f, "Division by zero"),
            Self::ParseError(s) => write!(f, "Parse error: {}", s),
            Self::Overflow => write!(f, "Amount overflow"),
            Self::DistributionError(s) => write!(f, "Distribution error: {}", s),
        }
    }
}

impl std::error::Error for AmountError {}

impl From<ParseFloatError> for AmountError {
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
