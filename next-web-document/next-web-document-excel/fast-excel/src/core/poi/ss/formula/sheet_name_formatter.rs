use crate::core::poi::ss::{
    spreadsheet_version::SpreadsheetVersion, util::cell_reference::CellReference,
};

/// Formats sheet names for use in formula expressions.
pub struct SheetNameFormatter;

impl SheetNameFormatter {
    const DELIMITER: char = '\'';

    /// Used to format sheet names as they would appear in cell formula expressions.
    ///
    /// # Returns
    /// The sheet name unchanged if there is no need for delimiting. Otherwise the sheet
    /// name is enclosed in single quotes ('). Any single quotes which were already present in the
    /// sheet name will be converted to double single quotes ('').
    pub fn format(raw_sheet_name: &str) -> String {
        let mut result = String::with_capacity(raw_sheet_name.len() + 2);
        Self::append_format(&mut result, raw_sheet_name);
        result
    }

    /// Convenience method for (`format()`) when a StringBuffer is already available.
    ///
    /// # Arguments
    /// * `out` - sheet name will be appended here possibly with delimiting quotes
    /// * `raw_sheet_name` - sheet name
    pub fn append_format(out: &mut dyn std::fmt::Write, raw_sheet_name: &str) {
        let needs_quotes = Self::needs_delimiting(raw_sheet_name);
        if needs_quotes {
            out.write_char(Self::DELIMITER).unwrap();
            Self::append_and_escape(out, raw_sheet_name);
            out.write_char(Self::DELIMITER).unwrap();
        } else {
            Self::append_and_escape(out, raw_sheet_name);
        }
    }

    /// Convenience method for (`format()`) when a StringBuffer is already available.
    ///
    /// # Arguments
    /// * `out` - sheet name will be appended here possibly with delimiting quotes
    /// * `workbook_name` - workbook name
    /// * `raw_sheet_name` - sheet name
    pub fn append_format_with_workbook(
        out: &mut dyn std::fmt::Write,
        workbook_name: Option<&str>,
        raw_sheet_name: &str,
    ) {
        let needs_quotes = workbook_name
            .map(|n| Self::needs_delimiting(n))
            .unwrap_or(false)
            || Self::needs_delimiting(raw_sheet_name);

        if needs_quotes {
            out.write_char(Self::DELIMITER).unwrap();
            if let Some(workbook_name) = workbook_name {
                out.write_char('[').unwrap();
                let escaped_workbook_name = workbook_name.replace('[', "(").replace(']', ")");
                Self::append_and_escape(out, &escaped_workbook_name);
                out.write_char(']').unwrap();
            }
            Self::append_and_escape(out, raw_sheet_name);
            out.write_char(Self::DELIMITER).unwrap();
        } else {
            if let Some(workbook_name) = workbook_name {
                out.write_char('[').unwrap();
                Self::append_or_ref(out, workbook_name);
                out.write_char(']').unwrap();
            }
            Self::append_or_ref(out, raw_sheet_name);
        }
    }

    fn append_or_ref(out: &mut dyn std::fmt::Write, name: &str) {
        if name.is_empty() {
            out.write_str("#REF").unwrap();
        } else {
            out.write_str(name).unwrap();
        }
    }

    fn append_and_escape(sb: &mut dyn std::fmt::Write, raw_sheet_name: &str) {
        if raw_sheet_name.is_empty() {
            sb.write_str("#REF").unwrap();
            return;
        }

        for ch in raw_sheet_name.chars() {
            if ch == Self::DELIMITER {
                // single quotes (') are encoded as ('')
                sb.write_char(Self::DELIMITER).unwrap();
            }
            sb.write_char(ch).unwrap();
        }
    }

    /// Tell if the given raw sheet name needs screening/delimiting.
    ///
    /// # Arguments
    /// * `raw_sheet_name` - the sheet name
    ///
    /// # Returns
    /// `true` if the given raw sheet name needs screening/delimiting, `false` otherwise or
    /// if the sheet name is empty.
    fn needs_delimiting(raw_sheet_name: &str) -> bool {
        if raw_sheet_name.is_empty() {
            return false;
        }

        let len = raw_sheet_name.len();
        if len < 1 {
            return false; // some cases we get missing external references, resulting in empty sheet names
        }

        if raw_sheet_name.chars().next().unwrap().is_ascii_digit() {
            // sheet name with digit in the first position always requires delimiting
            return true;
        }

        for ch in raw_sheet_name.chars() {
            if Self::is_special_char(ch) {
                return true;
            }
        }

        let first_char = raw_sheet_name.chars().next().unwrap();
        let last_char = raw_sheet_name.chars().last().unwrap();
        if first_char.is_ascii_alphabetic() && last_char.is_ascii_digit() {
            // note - values like "A$1:$C$20" don't get this far
            if Self::name_looks_like_plain_cell_reference(raw_sheet_name) {
                return true;
            }
        }

        if Self::name_looks_like_boolean_literal(raw_sheet_name) {
            return true;
        }

        if Self::name_starts_with_r1c1_cell_reference(raw_sheet_name) {
            return true;
        }

        // Error constant literals all contain '#' and other special characters
        // so they don't get this far
        false
    }

    fn name_looks_like_boolean_literal(raw_sheet_name: &str) -> bool {
        match raw_sheet_name.chars().next() {
            Some('T') | Some('t') => raw_sheet_name.eq_ignore_ascii_case("TRUE"),
            Some('F') | Some('f') => raw_sheet_name.eq_ignore_ascii_case("FALSE"),
            _ => false,
        }
    }

    /// Returns `true` if the presence of the specified character in a sheet name would
    /// require the sheet name to be delimited in formulas. This includes every non-alphanumeric
    /// character besides underscore '_' and dot '.'.
    pub fn is_special_char(ch: char) -> bool {
        // note - Character.isJavaIdentifierPart() would allow dollars '$'
        if ch.is_ascii_alphanumeric() {
            return false;
        }

        match ch {
            '.' | '_' => false, // dot and underscore are OK
            '\n' | '\r' | '\t' => {
                panic!("Illegal character (0x{:x}) found in sheet name", ch as u32)
            }
            _ => true,
        }
    }

    /// Used to decide whether sheet names like 'AB123' need delimiting due to the fact that they
    /// look like cell references.
    ///
    /// This code is currently being used for translating formulas represented with `Ptg`
    /// tokens into human readable text form. In formula expressions, a sheet name always has a
    /// trailing '!' so there is little chance for ambiguity.
    fn cell_reference_is_within_range(letters_prefix: &str, numbers_suffix: &str) -> bool {
        CellReference::cell_reference_is_within_range(
            letters_prefix,
            numbers_suffix,
            SpreadsheetVersion::Excel97,
        )
    }

    /// Note - this method assumes the specified raw_sheet_name has only letters and digits. It
    /// cannot be used to match absolute or range references (using the dollar or colon char).
    ///
    /// Some notable cases:
    /// * "A1" -> true
    /// * "a111" -> true
    /// * "AA" -> false
    /// * "aa1" -> true
    /// * "A1A" -> false
    /// * "A1A1" -> false
    /// * "A$1:$C$20" -> false (Not a plain cell reference)
    /// * "SALES20080101" -> true (Still needs delimiting even though well out of range)
    ///
    /// # Returns
    /// `true` if there is any possible ambiguity that the specified raw_sheet_name
    /// could be interpreted as a valid cell name.
    fn name_looks_like_plain_cell_reference(raw_sheet_name: &str) -> bool {
        // Simplified implementation - would need regex for exact pattern matching
        // For now, check if it matches the pattern letters+digits
        let mut has_letters = false;
        let mut has_digits = false;
        let mut in_digits = false;

        for ch in raw_sheet_name.chars() {
            if ch.is_ascii_alphabetic() {
                if in_digits {
                    // Letters after digits -> not a plain cell reference
                    return false;
                }
                has_letters = true;
            } else if ch.is_ascii_digit() {
                has_digits = true;
                in_digits = true;
            } else {
                // Contains non-alphanumeric character
                return false;
            }
        }

        if !has_letters || !has_digits {
            return false;
        }

        // Extract letters prefix and numbers suffix (simplified)
        let letters_prefix: String = raw_sheet_name
            .chars()
            .take_while(|c| c.is_ascii_alphabetic())
            .collect();
        let numbers_suffix: String = raw_sheet_name
            .chars()
            .skip_while(|c| c.is_ascii_alphabetic())
            .take_while(|c| c.is_ascii_digit())
            .collect();

        Self::cell_reference_is_within_range(&letters_prefix, &numbers_suffix)
    }

    /// Checks if the sheet name starts with R1C1 style cell reference.
    /// If this is the case Excel requires the sheet name to be enclosed in single quotes.
    ///
    /// # Returns
    /// `true` if the specified raw_sheet_name starts with R1C1 style cell reference
    fn name_starts_with_r1c1_cell_reference(raw_sheet_name: &str) -> bool {
        let len = raw_sheet_name.len();
        if len == 0 {
            return false;
        }

        let first_char = raw_sheet_name.chars().next().unwrap();
        match first_char {
            'R' | 'r' => {
                if len > 1 {
                    let second_char = raw_sheet_name.chars().nth(1).unwrap();
                    if second_char == 'C' || second_char == 'c' {
                        if len > 2 {
                            let third_char = raw_sheet_name.chars().nth(2).unwrap();
                            return third_char.is_ascii_digit();
                        } else {
                            return true;
                        }
                    } else {
                        return second_char.is_ascii_digit();
                    }
                } else {
                    return true;
                }
            }
            'C' | 'c' => {
                if len > 1 {
                    let second_char = raw_sheet_name.chars().nth(1).unwrap();
                    return second_char.is_ascii_digit();
                } else {
                    return true;
                }
            }
            _ => false,
        }
    }
}
