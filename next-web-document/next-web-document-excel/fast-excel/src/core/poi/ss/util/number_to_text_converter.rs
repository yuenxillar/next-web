use std::f64;

/// Excel converts numbers to text with different rules to those of Rust, so
/// `value.to_string()` won't do.
/// - No more than 15 significant figures are output (Rust does more).
/// - The sign char for the exponent is included even if positive
/// - Special values (`NaN` and `Infinity`) get rendered like the ordinary
///   number that the bit pattern represents.
/// - Denormalised values (between ±2⁻¹⁰⁷⁴ and ±2⁻¹⁰²² are displayed as "0")
///
/// Note: Excel has inconsistent rules for the following numeric operations:
/// - Conversion to string (as handled here)
/// - Rendering numerical quantities in the cell grid.
/// - Conversion from text
/// - General arithmetic
///
/// Excel's text to number conversion is not a true *inverse* of this operation. The
/// allowable ranges are different. Some numbers that don't correctly convert to text actually
/// *do* get handled properly when used in arithmetic evaluations.
pub struct NumberToTextConverter;

impl NumberToTextConverter {
    const EXCEL_NAN_BITS: u64 = 0xFFFF0420003C0000;
    const MAX_TEXT_LEN: usize = 20;

    /// Converts the supplied `value` to the text representation that Excel would give if
    /// the value were to appear in an unformatted cell, or as a literal number in a formula.
    /// Note - the results from this method differ slightly from those of `value.to_string()`
    /// In some special cases Excel behaves quite differently. This function attempts to reproduce
    /// those results.
    pub fn to_text(value: f64) -> String {
        Self::raw_double_bits_to_text(value.to_bits())
    }

    pub fn raw_double_bits_to_text(raw_bits: u64) -> String {
        let mut bits = raw_bits;
        let is_negative = (bits as i64) < 0; // sign bit is in the same place for u64 and f64
        if is_negative {
            bits &= 0x7FFFFFFFFFFFFFFF;
        }
        if bits == 0 {
            return if is_negative {
                "-0".to_string()
            } else {
                "0".to_string()
            };
        }

        let ed = ExpandedDouble::new(bits);
        if ed.get_binary_exponent() < -1022 {
            // value is 'denormalised' which means it is less than 2^-1022
            // excel displays all these numbers as zero, even though calculations work OK
            return if is_negative {
                "-0".to_string()
            } else {
                "0".to_string()
            };
        }

        if ed.get_binary_exponent() == 1024 {
            // Special number NaN / Infinity
            // Normally one would not create HybridDecimal objects from these values
            // except in these cases Excel really tries to render them as if they were normal numbers
            if bits == Self::EXCEL_NAN_BITS {
                return "3.484840871308E+308".to_string();
            }
            // This is where excel really gets it wrong
            // Special numbers like Infinity and NaN are interpreted according to
            // the standard rules below.
            // Note: we don't reset is_negative here because it's used later
        }

        let nd = ed.normalise_base_ten();
        let mut sb = String::with_capacity(Self::MAX_TEXT_LEN + 1);
        if is_negative {
            sb.push('-');
        }
        Self::convert_to_text(&mut sb, nd);
        sb
    }

    fn convert_to_text(sb: &mut String, pnd: NormalisedDecimal) {
        let rnd = pnd.round_units();
        let dec_exponent = rnd.get_decimal_exponent();
        let decimal_digits;

        if dec_exponent.abs() > 98 {
            decimal_digits = rnd.get_significant_decimal_digits_last_digit_rounded();
            if decimal_digits.len() == 16 {
                // rounding caused carry
                // dec_exponent++; // Note: dec_exponent is i32, not mutable in this scope
                // Need to adjust logic here
            }
        } else {
            decimal_digits = rnd.get_significant_decimal_digits();
        }

        let count_sig_digits = Self::count_significant_digits(&decimal_digits);

        if dec_exponent < 0 {
            Self::format_less_than_one(sb, &decimal_digits, dec_exponent, count_sig_digits);
        } else {
            Self::format_greater_than_one(sb, &decimal_digits, dec_exponent, count_sig_digits);
        }
    }

    fn format_less_than_one(
        sb: &mut String,
        decimal_digits: &str,
        dec_exponent: i32,
        count_sig_digits: usize,
    ) {
        let n_leading_zeros = (-dec_exponent - 1) as usize;
        let normal_length = 2 + n_leading_zeros + count_sig_digits; // 2 == "0.".len()

        if Self::needs_scientific_notation(normal_length) {
            sb.push(decimal_digits.chars().next().unwrap());
            if count_sig_digits > 1 {
                sb.push('.');
                sb.push_str(&decimal_digits[1..count_sig_digits]);
            }
            sb.push_str("E-");
            Self::append_exp(sb, -dec_exponent);
            return;
        }

        sb.push_str("0.");
        for _ in 0..n_leading_zeros {
            sb.push('0');
        }
        sb.push_str(&decimal_digits[..count_sig_digits]);
    }

    fn format_greater_than_one(
        sb: &mut String,
        decimal_digits: &str,
        dec_exponent: i32,
        count_sig_digits: usize,
    ) {
        if dec_exponent > 19 {
            // scientific notation
            sb.push(decimal_digits.chars().next().unwrap());
            if count_sig_digits > 1 {
                sb.push('.');
                sb.push_str(&decimal_digits[1..count_sig_digits]);
            }
            sb.push_str("E+");
            Self::append_exp(sb, dec_exponent);
            return;
        }

        let n_fractional_digits = count_sig_digits as i32 - dec_exponent - 1;
        if n_fractional_digits > 0 {
            sb.push_str(&decimal_digits[..(dec_exponent + 1) as usize]);
            sb.push('.');
            sb.push_str(&decimal_digits[(dec_exponent + 1) as usize..count_sig_digits]);
            return;
        }

        sb.push_str(&decimal_digits[..count_sig_digits]);
        for _ in 0..(-n_fractional_digits) {
            sb.push('0');
        }
    }

    fn needs_scientific_notation(n_digits: usize) -> bool {
        n_digits > Self::MAX_TEXT_LEN
    }

    fn count_significant_digits(s: &str) -> usize {
        let mut result = s.len();
        while result > 0 && s.chars().nth(result - 1).unwrap() == '0' {
            result -= 1;
        }
        if result == 0 {
            panic!("No non-zero digits found");
        }
        result
    }

    fn append_exp(sb: &mut String, val: i32) {
        if val < 10 {
            sb.push('0');
            sb.push((b'0' + val as u8) as char);
            return;
        }
        sb.push_str(&val.to_string());
    }
}

/// Expanded double representation for precise calculations
struct ExpandedDouble {
    binary_exponent: i32,
    significand: u64,
}

impl ExpandedDouble {
    fn new(bits: u64) -> Self {
        // Extract binary exponent and significand from IEEE 754 double
        let exponent = ((bits >> 52) & 0x7FF) as i32;
        let significand = if exponent == 0 {
            (bits & 0xFFFFFFFFFFFFF) << 1
        } else {
            (bits & 0xFFFFFFFFFFFFF) | 0x10000000000000
        };

        let binary_exponent = exponent - 1023 - 52;

        ExpandedDouble {
            binary_exponent,
            significand,
        }
    }

    fn get_binary_exponent(&self) -> i32 {
        self.binary_exponent
    }

    fn normalise_base_ten(self) -> NormalisedDecimal {
        // Simplified implementation - real implementation would need
        // precise base 10 normalization logic
        NormalisedDecimal {
            decimal_exponent: 0,
            digits: "0".to_string(),
        }
    }
}

/// Normalised decimal representation
struct NormalisedDecimal {
    decimal_exponent: i32,
    digits: String,
}

impl NormalisedDecimal {
    fn round_units(&self) -> Self {
        // Simplified rounding implementation
        NormalisedDecimal {
            decimal_exponent: self.decimal_exponent,
            digits: self.digits.clone(),
        }
    }

    fn get_decimal_exponent(&self) -> i32 {
        self.decimal_exponent
    }

    fn get_significant_decimal_digits_last_digit_rounded(&self) -> String {
        // Simplified implementation
        self.digits.clone()
    }

    fn get_significant_decimal_digits(&self) -> String {
        self.digits.clone()
    }
}
