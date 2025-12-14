use std::char;
use std::cmp::min;
use std::iter::FromIterator;
use std::str;
use std::string::String as RustString;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use once_cell::sync::Lazy;
use std::io::{Read, Write};

use crate::core::poi::util::little_endian_input::LittleEndianInput;
use crate::core::poi::util::little_endian_output::LittleEndianOutput;

static MAX_RECORD_LENGTH: Lazy<Arc<AtomicUsize>> =
    Lazy::new(|| Arc::new(AtomicUsize::new(StringUtil::DEFAULT_MAX_RECORD_LENGTH)));

/// Collection of string handling utilities
pub(crate) struct StringUtil;

impl StringUtil {
    /// Arbitrarily selected; may need to increase
    const DEFAULT_MAX_RECORD_LENGTH: usize = 10_000_000;

    /// Sets the maximum record length allowed for StringUtil
    pub(crate) fn set_max_record_length(length: usize) {
        MAX_RECORD_LENGTH.store(length, Ordering::Relaxed);
    }

    /// Gets the maximum record length allowed for StringUtil
    pub(crate) fn get_max_record_length() -> usize {
        MAX_RECORD_LENGTH.load(Ordering::Relaxed)
    }

    /// Given a byte array of 16-bit unicode characters in Little Endian
    /// format (most important byte last), return a Java String representation
    /// of it.
    ///
    /// # Arguments
    /// * `string` - the byte array to be converted
    /// * `offset` - the initial offset into the byte array
    /// * `len` - the length of the final string
    ///
    /// # Returns
    /// The converted string, never empty.
    pub(crate) fn get_from_unicode_le(string: &[u8], offset: usize, len: usize) -> RustString {
        if len == 0 {
            return RustString::new();
        }

        if offset >= string.len() {
            panic!(
                "Illegal offset {} (String data is of length {})",
                offset,
                string.len()
            );
        }

        if len * 2 > string.len() - offset {
            panic!("Illegal length {}", len);
        }

        // UTF-16LE decoding
        let mut result = RustString::with_capacity(len);
        let mut i = offset;

        for _ in 0..len {
            if i + 1 >= string.len() {
                break;
            }
            let low = string[i] as u16;
            let high = string[i + 1] as u16;
            let code_point = (high << 8) | low;

            if let Some(c) = char::from_u32(code_point as u32) {
                result.push(c);
            } else {
                // Invalid character, push replacement character
                result.push('\u{FFFD}');
            }

            i += 2;
        }

        result
    }

    /// Given a byte array of 16-bit unicode characters in little endian
    /// format (most important byte last), return a Java String representation
    /// of it.
    pub(crate) fn get_from_unicode_le_bytes(string: &[u8]) -> RustString {
        if string.is_empty() {
            return RustString::new();
        }
        Self::get_from_unicode_le(string, 0, string.len() / 2)
    }

    /// Convert String to 16-bit unicode characters in little endian format
    pub(crate) fn get_to_unicode_le(string: &str) -> Vec<u8> {
        let mut result = Vec::with_capacity(string.len() * 2);

        for c in string.chars() {
            let code_point = c as u32;
            if code_point <= 0xFFFF {
                result.push((code_point & 0xFF) as u8);
                result.push((code_point >> 8) as u8);
            } else {
                // Handle surrogate pairs for characters beyond BMP
                let code_point = code_point - 0x10000;
                let high_surrogate = 0xD800 + (code_point >> 10);
                let low_surrogate = 0xDC00 + (code_point & 0x3FF);

                result.push((high_surrogate & 0xFF) as u8);
                result.push((high_surrogate >> 8) as u8);
                result.push((low_surrogate & 0xFF) as u8);
                result.push((low_surrogate >> 8) as u8);
            }
        }

        result
    }

    /// Read 8 bit data (in ISO-8859-1 codepage) into a (unicode) Java
    /// String and return.
    pub(crate) fn get_from_compressed_unicode(
        string: &[u8],
        offset: usize,
        len: usize,
    ) -> RustString {
        let len_to_use = min(len, string.len() - offset);

        // ISO-8859-1 decoding (1 byte per character)
        let mut result = RustString::with_capacity(len_to_use);

        for i in offset..offset + len_to_use {
            let byte = string[i];
            // ISO-8859-1 maps directly to Unicode code points 0-255
            result.push(byte as char);
        }

        result
    }

    /// Read 8 bit data (in UTF-8 codepage) into a (unicode) Java
    /// String and return.
    pub(crate) fn get_from_compressed_utf8(string: &[u8], offset: usize, len: usize) -> RustString {
        let len_to_use = min(len, string.len() - offset);

        // UTF-8 decoding
        match str::from_utf8(&string[offset..offset + len_to_use]) {
            Ok(s) => s.to_string(),
            Err(_) => {
                // Invalid UTF-8, try to recover
                let mut result = RustString::with_capacity(len_to_use);
                let mut i = offset;

                while i < offset + len_to_use {
                    let byte = string[i];
                    if byte < 0x80 {
                        result.push(byte as char);
                        i += 1;
                    } else if byte < 0xE0 && i + 1 < offset + len_to_use {
                        // 2-byte UTF-8
                        let b1 = (byte & 0x1F) as u32;
                        let b2 = (string[i + 1] & 0x3F) as u32;
                        let code_point = (b1 << 6) | b2;
                        if let Some(c) = char::from_u32(code_point) {
                            result.push(c);
                        }
                        i += 2;
                    } else if byte < 0xF0 && i + 2 < offset + len_to_use {
                        // 3-byte UTF-8
                        let b1 = (byte & 0x0F) as u32;
                        let b2 = (string[i + 1] & 0x3F) as u32;
                        let b3 = (string[i + 2] & 0x3F) as u32;
                        let code_point = (b1 << 12) | (b2 << 6) | b3;
                        if let Some(c) = char::from_u32(code_point) {
                            result.push(c);
                        }
                        i += 3;
                    } else if i + 3 < offset + len_to_use {
                        // 4-byte UTF-8
                        let b1 = (byte & 0x07) as u32;
                        let b2 = (string[i + 1] & 0x3F) as u32;
                        let b3 = (string[i + 2] & 0x3F) as u32;
                        let b4 = (string[i + 3] & 0x3F) as u32;
                        let code_point = (b1 << 18) | (b2 << 12) | (b3 << 6) | b4;
                        if let Some(c) = char::from_u32(code_point) {
                            result.push(c);
                        }
                        i += 4;
                    } else {
                        // Invalid sequence, skip
                        i += 1;
                    }
                }

                result
            }
        }
    }

    /// Read compressed Unicode from a LittleEndianInput stream
    pub(crate) fn read_compressed_unicode(
        in_: &mut dyn LittleEndianInput,
        n_chars: u16,
    ) -> RustString {
        let max_len = Self::get_max_record_length();
        let buf_len = min(n_chars, max_len);
        let mut buf = vec![0u8; buf_len];

        if let Ok(_) = in_.read_exact(&mut buf) {
            Self::get_from_compressed_unicode(&buf, 0, buf_len)
        } else {
            RustString::new()
        }
    }

    /// Read Unicode string from a LittleEndianInput stream
    pub(crate) fn read_unicode_string(in_: &mut dyn LittleEndianInput) -> RustString {
        let n_chars = in_.read_u_short();
        let flag = in_.read_byte();

        if (flag & 0x01) == 0 {
            Self::read_compressed_unicode(in_, n_chars)
        } else {
            Self::read_unicode_le(in_, n_chars)
        }
    }

    /// Read Unicode string with known character count
    pub(crate) fn read_unicode_string_with_count<R: Read + ReadBytesExt>(
        in_: &mut R,
        n_chars: usize,
    ) -> RustString {
        let is_16bit = in_.read_u8().unwrap_or(0);

        if (is_16bit & 0x01) == 0 {
            Self::read_compressed_unicode(in_, n_chars)
        } else {
            Self::read_unicode_le(in_, n_chars)
        }
    }

    /// Write Unicode string to a LittleEndianOutput stream
    pub(crate) fn write_unicode_string(out: &mut dyn LittleEndianOutput, value: &str) {
        let n_chars = value.len();
        out.write_u16::<LittleEndian>(n_chars as u16).unwrap();

        let is_16bit = Self::has_multibyte(value);
        out.write_u8(if is_16bit { 0x01 } else { 0x00 }).unwrap();

        if is_16bit {
            Self::put_unicode_le(value, out);
        } else {
            Self::put_compressed_unicode(value, out);
        }
    }

    /// Write Unicode string flag and data only
    pub(crate) fn write_unicode_string_flag_and_data<W: Write + WriteBytesExt>(
        out: &mut W,
        value: &str,
    ) {
        let is_16bit = Self::has_multibyte(value);
        out.write_u8(if is_16bit { 0x01 } else { 0x00 }).unwrap();

        if is_16bit {
            Self::put_unicode_le(value, out);
        } else {
            Self::put_compressed_unicode(value, out);
        }
    }

    /// Get the encoded size of a Unicode string
    pub(crate) fn get_encoded_size(value: &str) -> usize {
        let mut result = 2 + 1; // n_chars (2 bytes) + flag (1 byte)
        result += value.len() * (if Self::has_multibyte(value) { 2 } else { 1 });
        result
    }

    /// Put compressed Unicode into byte array
    pub(crate) fn put_compressed_unicode_bytes(input: &str, output: &mut [u8], offset: usize) {
        let bytes = input.as_bytes();
        let len = min(bytes.len(), output.len() - offset);
        output[offset..offset + len].copy_from_slice(&bytes[..len]);
    }

    /// Put compressed Unicode into output stream
    pub(crate) fn put_compressed_unicode<W: Write>(input: &str, out: &mut W) {
        out.write_all(input.as_bytes()).unwrap();
    }

    /// Put Unicode LE into byte array
    pub(crate) fn put_unicode_le_bytes(input: &str, output: &mut [u8], offset: usize) {
        let bytes = Self::get_to_unicode_le(input);
        let len = min(bytes.len(), output.len() - offset);
        output[offset..offset + len].copy_from_slice(&bytes[..len]);
    }

    /// Put Unicode LE into output stream
    pub(crate) fn put_unicode_le<W: Write>(input: &str, out: &mut W) {
        let bytes = Self::get_to_unicode_le(input);
        out.write_all(&bytes).unwrap();
    }

    /// Read Unicode LE from input stream
    pub(crate) fn read_unicode_le<R: Read>(in_: &mut R, n_chars: usize) -> RustString {
        let max_len = Self::get_max_record_length();
        let buf_len = min(n_chars * 2, max_len);
        let mut buf = vec![0u8; buf_len];

        if let Ok(_) = in_.read_exact(&mut buf) {
            Self::get_from_unicode_le(&buf, 0, n_chars)
        } else {
            RustString::new()
        }
    }

    /// Get preferred encoding
    pub(crate) fn get_preferred_encoding() -> &'static str {
        "ISO-8859-1"
    }

    /// Check if the string has multibyte character
    pub(crate) fn has_multibyte(value: &str) -> bool {
        value.chars().any(|c| c as u32 > 0xFF)
    }

    /// Tests if the string starts with the specified prefix, ignoring case consideration.
    pub(crate) fn starts_with_ignore_case(haystack: &str, prefix: &str) -> bool {
        if haystack.len() < prefix.len() {
            return false;
        }
        haystack[..prefix.len()].eq_ignore_ascii_case(prefix)
    }

    /// Tests if the string ends with the specified suffix, ignoring case consideration.
    pub(crate) fn ends_with_ignore_case(haystack: &str, suffix: &str) -> bool {
        if haystack.len() < suffix.len() {
            return false;
        }
        let start = haystack.len() - suffix.len();
        haystack[start..].eq_ignore_ascii_case(suffix)
    }

    /// Convert character to lowercase
    pub(crate) fn to_lowercase_char(c: char) -> RustString {
        c.to_lowercase().collect()
    }

    /// Convert character to uppercase
    pub(crate) fn to_uppercase_char(c: char) -> RustString {
        c.to_uppercase().collect()
    }

    /// Check if character is uppercase
    pub(crate) fn is_uppercase_char(c: char) -> bool {
        c.is_uppercase()
    }

    /// Map Microsoft codepoint string to Unicode
    pub(crate) fn map_ms_codepoint_string(string: &str) -> RustString {
        if string.is_empty() {
            return RustString::new();
        }

        let mut result = RustString::with_capacity(string.len());

        for c in string.chars() {
            let cp = c as u32;
            let mapped = Self::map_ms_codepoint(cp);
            if let Some(mapped_char) = char::from_u32(mapped) {
                result.push(mapped_char);
            } else {
                result.push(c);
            }
        }

        result
    }

    fn map_ms_codepoint(cp: u32) -> u32 {
        match cp {
            0xf020..=0xf07f => {
                let idx = (cp - 0xf020) as usize;
                if idx < SYMBOL_MAP_F020.len() {
                    SYMBOL_MAP_F020[idx]
                } else {
                    cp
                }
            }
            0xf0a0..=0xf0ff => {
                let idx = (cp - 0xf0a0) as usize;
                if idx < SYMBOL_MAP_F0A0.len() {
                    SYMBOL_MAP_F0A0[idx]
                } else {
                    cp
                }
            }
            _ => cp,
        }
    }

    /// Join array with separator
    pub(crate) fn join<T: ToString>(array: &[T], separator: &str) -> RustString {
        if array.is_empty() {
            return RustString::new();
        }

        let mut result = array[0].to_string();

        for i in 1..array.len() {
            result.push_str(separator);
            result.push_str(&array[i].to_string());
        }

        result
    }

    /// Join array without separator
    pub(crate) fn join_simple<T: ToString>(array: &[T]) -> RustString {
        let mut result = RustString::new();

        for item in array {
            result.push_str(&item.to_string());
        }

        result
    }

    /// Count number of occurrences of needle in haystack
    pub(crate) fn count_matches(haystack: &str, needle: char) -> usize {
        haystack.chars().filter(|&c| c == needle).count()
    }

    /// Get from Unicode LE with null termination
    pub(crate) fn get_from_unicode_le_0_terminated(
        string: &[u8],
        offset: usize,
        len: usize,
    ) -> RustString {
        if offset >= string.len() {
            panic!(
                "Illegal offset {} (String data is of length {})",
                offset,
                string.len()
            );
        }

        if len * 2 > string.len() - offset {
            panic!("Illegal length {}", len);
        }

        let (new_offset, prefix, new_max_len) = if len > 0
            && offset < string.len() - 1
            && string[offset] == 0
            && string[offset + 1] == 0
        {
            let new_offset = offset + 2;
            let prefix = "?";

            // Check if next char is valid
            let cp = if len > 1 && offset + 3 < string.len() {
                let low = string[offset + 2] as u16;
                let high = string[offset + 3] as u16;
                (high << 8) | low
            } else {
                0
            };

            let new_max_len = if Self::is_java_identifier_part(cp as u32) {
                len - 1
            } else {
                0
            };

            (new_offset, prefix, new_max_len)
        } else {
            (offset, "", len)
        };

        let mut new_len = 0;

        // Find null termination
        while new_len < new_max_len {
            let idx = new_offset + new_len * 2;
            if idx + 1 >= string.len() {
                break;
            }
            if string[idx] == 0 && string[idx + 1] == 0 {
                break;
            }
            new_len += 1;
        }

        let new_len = min(new_len, new_max_len);

        let mut result = RustString::from(prefix);
        if new_len > 0 {
            result.push_str(&Self::get_from_unicode_le(string, new_offset, new_len));
        }

        result
    }

    /// Get length of CharSequence or 0 if null
    pub(crate) fn length(cs: Option<&str>) -> usize {
        cs.map_or(0, |s| s.len())
    }

    /// Check if CharSequence is blank (empty, null, or whitespace only)
    pub(crate) fn is_blank(cs: Option<&str>) -> bool {
        match cs {
            Some(s) => s.chars().all(|c| c.is_whitespace()),
            None => true,
        }
    }

    /// Check if CharSequence is not blank
    pub(crate) fn is_not_blank(cs: Option<&str>) -> bool {
        !Self::is_blank(cs)
    }

    /// Repeat character n times
    pub(crate) fn repeat(ch: char, repeat: usize) -> RustString {
        if repeat == 0 {
            return RustString::new();
        }
        RustString::from_iter(std::iter::repeat(ch).take(repeat))
    }

    // Helper method to check if character is Java identifier part
    fn is_java_identifier_part(cp: u32) -> bool {
        match cp {
            0..=0x7F => {
                // ASCII range
                let c = cp as u8 as char;
                c.is_alphanumeric() || c == '_' || c == '$'
            }
            _ => {
                // Use Unicode properties
                if let Some(c) = char::from_u32(cp) {
                    c.is_alphanumeric() || c == '_' || c == '$'
                } else {
                    false
                }
            }
        }
    }
}

// Symbol mapping tables
static SYMBOL_MAP_F020: [u32; 96] = [
    ' ' as u32, // 0xf020 space
    '!' as u32, // 0xf021 exclam
    8704,       // 0xf022 universal
    '#' as u32, // 0xf023 numbersign
    8707,       // 0xf024 existential
    '%' as u32, // 0xf025 percent
    '&' as u32, // 0xf026 ampersand
    8717,       // 0xf027 suchthat
    '(' as u32, // 0xf028 parenleft
    ')' as u32, // 0xf029 parentright
    8727,       // 0xf02a asteriskmath
    '+' as u32, // 0xf02b plus
    ',' as u32, // 0xf02c comma
    8722,       // 0xf02d minus sign (long -)
    '.' as u32, // 0xf02e period
    '/' as u32, // 0xf02f slash
    '0' as u32, // 0xf030 0
    '1' as u32, // 0xf031 1
    '2' as u32, // 0xf032 2
    '3' as u32, // 0xf033 3
    '4' as u32, // 0xf034 4
    '5' as u32, // 0xf035 5
    '6' as u32, // 0xf036 6
    '7' as u32, // 0xf037 7
    '8' as u32, // 0xf038 8
    '9' as u32, // 0xf039 9
    ':' as u32, // 0xf03a colon
    ';' as u32, // 0xf03b semicolon
    '<' as u32, // 0xf03c less
    '=' as u32, // 0xf03d equal
    '>' as u32, // 0xf03e greater
    '?' as u32, // 0xf03f question
    8773,       // 0xf040 congruent
    913,        // 0xf041 alpha (upper)
    914,        // 0xf042 beta (upper)
    935,        // 0xf043 chi (upper)
    916,        // 0xf044 delta (upper)
    917,        // 0xf045 epsilon (upper)
    934,        // 0xf046 phi (upper)
    915,        // 0xf047 gamma (upper)
    919,        // 0xf048 eta (upper)
    921,        // 0xf049 iota (upper)
    977,        // 0xf04a theta1 (lower)
    922,        // 0xf04b kappa (upper)
    923,        // 0xf04c lambda (upper)
    924,        // 0xf04d mu (upper)
    925,        // 0xf04e nu (upper)
    927,        // 0xf04f omicron (upper)
    928,        // 0xf050 pi (upper)
    920,        // 0xf051 theta (upper)
    929,        // 0xf052 rho (upper)
    931,        // 0xf053 sigma (upper)
    932,        // 0xf054 tau (upper)
    933,        // 0xf055 upsilon (upper)
    962,        // 0xf056 simga1 (lower)
    937,        // 0xf057 omega (upper)
    926,        // 0xf058 xi (upper)
    936,        // 0xf059 psi (upper)
    918,        // 0xf05a zeta (upper)
    '[' as u32, // 0xf05b bracketleft
    8765,       // 0xf05c therefore
    ']' as u32, // 0xf05d bracketright
    8869,       // 0xf05e perpendicular
    '_' as u32, // 0xf05f underscore
    ' ' as u32, // 0xf060 radicalex (doesn't exist in unicode)
    945,        // 0xf061 alpha (lower)
    946,        // 0xf062 beta (lower)
    967,        // 0xf063 chi (lower)
    948,        // 0xf064 delta (lower)
    949,        // 0xf065 epsilon (lower)
    966,        // 0xf066 phi (lower)
    947,        // 0xf067 gamma (lower)
    951,        // 0xf068 eta (lower)
    953,        // 0xf069 iota (lower)
    981,        // 0xf06a phi1 (lower)
    954,        // 0xf06b kappa (lower)
    955,        // 0xf06c lambda (lower)
    956,        // 0xf06d mu (lower)
    957,        // 0xf06e nu (lower)
    959,        // 0xf06f omnicron (lower)
    960,        // 0xf070 pi (lower)
    952,        // 0xf071 theta (lower)
    961,        // 0xf072 rho (lower)
    963,        // 0xf073 sigma (lower)
    964,        // 0xf074 tau (lower)
    965,        // 0xf075 upsilon (lower)
    982,        // 0xf076 piv (lower)
    969,        // 0xf077 omega (lower)
    958,        // 0xf078 xi (lower)
    968,        // 0xf079 psi (lower)
    950,        // 0xf07a zeta (lower)
    '{' as u32, // 0xf07b braceleft
    '|' as u32, // 0xf07c bar
    '}' as u32, // 0xf07d braceright
    8764,       // 0xf07e similar '~'
    ' ' as u32, // 0xf07f not defined
];

static SYMBOL_MAP_F0A0: [u32; 96] = [
    8364,       // 0xf0a0 not defined / euro symbol
    978,        // 0xf0a1 upsilon1 (upper)
    8242,       // 0xf0a2 minute
    8804,       // 0xf0a3 lessequal
    8260,       // 0xf0a4 fraction
    8734,       // 0xf0a5 infinity
    402,        // 0xf0a6 florin
    9827,       // 0xf0a7 club
    9830,       // 0xf0a8 diamond
    9829,       // 0xf0a9 heart
    9824,       // 0xf0aa spade
    8596,       // 0xf0ab arrowboth
    8591,       // 0xf0ac arrowleft
    8593,       // 0xf0ad arrowup
    8594,       // 0xf0ae arrowright
    8595,       // 0xf0af arrowdown
    176,        // 0xf0b0 degree
    177,        // 0xf0b1 plusminus
    8243,       // 0xf0b2 second
    8805,       // 0xf0b3 greaterequal
    215,        // 0xf0b4 multiply
    181,        // 0xf0b5 proportional
    8706,       // 0xf0b6 partialdiff
    8729,       // 0xf0b7 bullet
    247,        // 0xf0b8 divide
    8800,       // 0xf0b9 notequal
    8801,       // 0xf0ba equivalence
    8776,       // 0xf0bb approxequal
    8230,       // 0xf0bc ellipsis
    9168,       // 0xf0bd arrowvertex
    9135,       // 0xf0be arrowhorizex
    8629,       // 0xf0bf carriagereturn
    8501,       // 0xf0c0 aleph
    8475,       // 0xf0c1 Ifraktur
    8476,       // 0xf0c2 Rfraktur
    8472,       // 0xf0c3 weierstrass
    8855,       // 0xf0c4 circlemultiply
    8853,       // 0xf0c5 circleplus
    8709,       // 0xf0c6 emptyset
    8745,       // 0xf0c7 intersection
    8746,       // 0xf0c8 union
    8835,       // 0xf0c9 propersuperset
    8839,       // 0xf0ca reflexsuperset
    8836,       // 0xf0cb notsubset
    8834,       // 0xf0cc propersubset
    8838,       // 0xf0cd reflexsubset
    8712,       // 0xf0ce element
    8713,       // 0xf0cf notelement
    8736,       // 0xf0d0 angle
    8711,       // 0xf0d1 gradient
    174,        // 0xf0d2 registerserif
    169,        // 0xf0d3 copyrightserif
    8482,       // 0xf0d4 trademarkserif
    8719,       // 0xf0d5 product
    8730,       // 0xf0d6 radical
    8901,       // 0xf0d7 dotmath
    172,        // 0xf0d8 logicalnot
    8743,       // 0xf0d9 logicaland
    8744,       // 0xf0da logicalor
    8660,       // 0xf0db arrowdblboth
    8656,       // 0xf0dc arrowdblleft
    8657,       // 0xf0dd arrowdblup
    8658,       // 0xf0de arrowdblright
    8659,       // 0xf0df arrowdbldown
    9674,       // 0xf0e0 lozenge
    9001,       // 0xf0e1 angleleft
    174,        // 0xf0e2 registersans
    169,        // 0xf0e3 copyrightsans
    8482,       // 0xf0e4 trademarksans
    8721,       // 0xf0e5 summation
    9115,       // 0xf0e6 parenlefttp
    9116,       // 0xf0e7 parenleftex
    9117,       // 0xf0e8 parenleftbt
    9121,       // 0xf0e9 bracketlefttp
    9122,       // 0xf0ea bracketleftex
    9123,       // 0xf0eb bracketleftbt
    9127,       // 0xf0ec bracelefttp
    9128,       // 0xf0ed braceleftmid
    9129,       // 0xf0ee braceleftbt
    9130,       // 0xf0ef braceex
    ' ' as u32, // 0xf0f0 not defined
    9002,       // 0xf0f1 angleright
    8747,       // 0xf0f2 integral
    8992,       // 0xf0f3 integraltp
    9134,       // 0xf0f4 integralex
    8993,       // 0xf0f5 integralbt
    9118,       // 0xf0f6 parenrighttp
    9119,       // 0xf0f7 parenrightex
    9120,       // 0xf0f8 parenrightbt
    9124,       // 0xf0f9 bracketrighttp
    9125,       // 0xf0fa bracketrightex
    9126,       // 0xf0fb bracketrightbt
    9131,       // 0xf0fc bracerighttp
    9132,       // 0xf0fd bracerightmid
    9133,       // 0xf0fe bracerightbt
    ' ' as u32, // 0xf0ff not defined
];
