use std::cmp::min;
use std::str;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::{char, io};

use byteorder::{ReadBytesExt, WriteBytesExt};
use encoding::all::{ISO_8859_1, UTF_8, UTF_16LE};
use encoding::{DecoderTrap, EncoderTrap, Encoding};
use next_web_core::error::BoxError;
use once_cell::sync::Lazy;
use std::io::{Read, Write};

use crate::core::poi::util::little_endian_input::LittleEndianInput;
use crate::core::poi::util::little_endian_output::LittleEndianOutput;

static MAX_RECORD_LENGTH: Lazy<Arc<AtomicUsize>> =
    Lazy::new(|| Arc::new(AtomicUsize::new(StringUtil::DEFAULT_MAX_RECORD_LENGTH)));

/// Collection of string handling utilities
pub struct StringUtil;

impl StringUtil {
    /// Arbitrarily selected; may need to increase
    pub const DEFAULT_MAX_RECORD_LENGTH: usize = 10_000_000;

    /// Sets the maximum record length allowed for StringUtil
    pub fn set_max_record_length(length: usize) {
        MAX_RECORD_LENGTH.store(length, Ordering::Relaxed);
    }

    /// Gets the maximum record length allowed for StringUtil
    pub fn get_max_record_length() -> usize {
        MAX_RECORD_LENGTH.load(Ordering::Relaxed)
    }

    /// Given a byte array of 16-bit unicode characters in Little Endian
    /// of it.
    ///
    /// # Arguments
    /// * `string` - the byte array to be converted
    /// * `offset` - the initial offset into the byte array
    /// * `len` - the length of the final string
    ///
    /// # Returns
    /// The converted string, never empty.
    pub fn get_from_unicode_le(string: &[u8], offset: usize, len: usize) -> String {
        if len == 0 {
            return String::new();
        }

        // ：使用 UTF_16LE 解码器
        match UTF_16LE.decode(&string[offset..offset + len * 2], DecoderTrap::Replace) {
            Ok(result) => result,
            Err(_) => {
                // 回退到手动解码
                Self::get_from_unicode_le_fallback(string, offset, len)
            }
        }
    }

    /// 回退方法：手动 UTF-16LE 解码
    fn get_from_unicode_le_fallback(string: &[u8], offset: usize, len: usize) -> String {
        let mut result = String::with_capacity(len);
        let mut i = offset;

        for _ in 0..len {
            if i + 1 >= string.len() {
                break;
            }
            let low = string[i] as u16;
            let high = string[i + 1] as u16;
            let code_point = (high << 8) | low;

            match char::from_u32(code_point as u32) {
                Some(c) => result.push(c),
                None => result.push('\u{FFFD}'), // 替换字符
            }

            i += 2;
        }

        result
    }

    /// Given a byte array of 16-bit unicode characters in little endian
    pub fn get_from_unicode_le_bytes(string: &[u8]) -> String {
        if string.is_empty() {
            return String::new();
        }
        Self::get_from_unicode_le(string, 0, string.len() / 2)
    }

    /// Convert String to 16-bit unicode characters in little endian format
    /// ：使用 Replace 策略处理无效字符
    pub fn get_to_unicode_le(input: &str) -> io::Result<Vec<u8>> {
        UTF_16LE.encode(input, EncoderTrap::Replace).map_err(|e| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("UTF_16LE encoding error: {:?}", e),
            )
        })
    }

    /// Read 8 bit data (in ISO-8859-1 codepage) into a (unicode)
    /// String and return.
    /// ：使用 ISO_8859_1 解码器
    pub fn get_from_compressed_unicode(string: &[u8], offset: usize, len: usize) -> String {
        let len_to_use = min(len, string.len() - offset);

        if len_to_use == 0 {
            return String::new();
        }

        // ：使用 ISO_8859_1 解码器
        match ISO_8859_1.decode(&string[offset..offset + len_to_use], DecoderTrap::Replace) {
            Ok(result) => result,
            Err(_) => {
                // 回退到手动解码
                let mut result = String::with_capacity(len_to_use);
                for i in offset..offset + len_to_use {
                    result.push(string[i] as char); // ISO-8859-1 直接映射
                }
                result
            }
        }
    }

    /// Read 8 bit data (in UTF-8 codepage) into a (unicode)
    /// String and return.
    /// ：使用 UTF_8 解码器
    pub fn get_from_compressed_utf8(string: &[u8], offset: usize, len: usize) -> String {
        let len_to_use = min(len, string.len() - offset);

        if len_to_use == 0 {
            return String::new();
        }

        // ：优先使用 UTF_8 解码器
        match UTF_8.decode(&string[offset..offset + len_to_use], DecoderTrap::Replace) {
            Ok(result) => result,
            Err(_) => {
                // 回退到手动解码（与之前逻辑保持一致）
                Self::decode_utf8_manually(string, offset, len_to_use)
            }
        }
    }

    /// 手动 UTF-8 解码（回退方法）
    fn decode_utf8_manually(string: &[u8], offset: usize, len: usize) -> String {
        let mut result = String::with_capacity(len);
        let mut i = offset;
        let end = offset + len;

        while i < end {
            let byte = string[i];

            if byte < 0x80 {
                result.push(byte as char);
                i += 1;
            } else if byte < 0xE0 && i + 1 < end {
                // 2字节 UTF-8
                let b1 = (byte & 0x1F) as u32;
                let b2 = (string[i + 1] & 0x3F) as u32;
                let code_point = (b1 << 6) | b2;
                if let Some(c) = char::from_u32(code_point) {
                    result.push(c);
                } else {
                    result.push('\u{FFFD}'); // 替换字符
                }
                i += 2;
            } else if byte < 0xF0 && i + 2 < end {
                // 3字节 UTF-8
                let b1 = (byte & 0x0F) as u32;
                let b2 = (string[i + 1] & 0x3F) as u32;
                let b3 = (string[i + 2] & 0x3F) as u32;
                let code_point = (b1 << 12) | (b2 << 6) | b3;
                if let Some(c) = char::from_u32(code_point) {
                    result.push(c);
                } else {
                    result.push('\u{FFFD}'); // 替换字符
                }
                i += 3;
            } else if i + 3 < end {
                // 4字节 UTF-8
                let b1 = (byte & 0x07) as u32;
                let b2 = (string[i + 1] & 0x3F) as u32;
                let b3 = (string[i + 2] & 0x3F) as u32;
                let b4 = (string[i + 3] & 0x3F) as u32;
                let code_point = (b1 << 18) | (b2 << 12) | (b3 << 6) | b4;
                if let Some(c) = char::from_u32(code_point) {
                    result.push(c);
                } else {
                    result.push('\u{FFFD}'); // 替换字符
                }
                i += 4;
            } else {
                // 无效序列，跳过并添加替换字符
                result.push('\u{FFFD}');
                i += 1;
            }
        }

        result
    }

    /// Read compressed Unicode from a LittleEndianInput stream
    /// ：使用 ISO_8859_1 解码
    pub fn read_compressed_unicode(
        in_: &mut dyn LittleEndianInput,
        n_chars: u16,
    ) -> Result<String, BoxError> {
        let max_len = Self::get_max_record_length();
        let buf_len = min(n_chars as usize, max_len);
        let mut buf = vec![0u8; buf_len];

        in_.read_fully(&mut buf);

        // ：使用 Replace 策略
        Ok(ISO_8859_1.decode(&buf, DecoderTrap::Replace)?)
    }

    /// Read Unicode string from a LittleEndianInput stream
    pub fn read_unicode_string(in_: &mut dyn LittleEndianInput) -> io::Result<String> {
        let n_chars = in_.read_u_short();
        let flag = in_.read_byte();

        if (flag & 0x01) == 0 {
            Self::read_compressed_unicode(in_, n_chars)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))
        } else {
            Self::read_unicode_le(in_, n_chars as usize)
        }
    }

    /// Read Unicode string with known character count
    pub fn read_unicode_string_with_count<R: Read + ReadBytesExt>(
        in_: &mut R,
        n_chars: usize,
    ) -> io::Result<String> {
        let is_16bit = in_.read_u8()?;

        if (is_16bit & 0x01) == 0 {
            // 读取压缩 Unicode
            let mut buf = vec![0u8; n_chars];
            in_.read_exact(&mut buf)?;
            Ok(ISO_8859_1
                .decode(&buf, DecoderTrap::Replace)
                .unwrap_or_else(|_| String::new()))
        } else {
            // 读取 Unicode LE
            let mut buf = vec![0u8; n_chars * 2];
            in_.read_exact(&mut buf)?;
            Ok(UTF_16LE
                .decode(&buf, DecoderTrap::Replace)
                .unwrap_or_else(|_| String::new()))
        }
    }

    /// Write Unicode string to a LittleEndianOutput stream
    pub fn write_unicode_string(out: &mut dyn LittleEndianOutput, value: &str) -> io::Result<()> {
        let n_chars = value.len();
        out.write_short(n_chars as u16)?;

        let is_16bit = Self::has_multibyte(value);
        out.write_byte(if is_16bit { 0x01 } else { 0x00 })?;

        if is_16bit {
            Self::put_unicode_le(value, out)
        } else {
            Self::put_compressed_unicode(value, out)
        }
    }

    /// Write Unicode string flag and data only
    pub fn write_unicode_string_flag_and_data<W: Write + WriteBytesExt>(
        out: &mut W,
        value: &str,
    ) -> io::Result<()> {
        let is_16bit = Self::has_multibyte(value);
        out.write_u8(if is_16bit { 0x01 } else { 0x00 })?;

        if is_16bit {
            let bytes = Self::get_to_unicode_le(value)?;
            out.write_all(&bytes)
        } else {
            // 使用正确的 ISO-8859-1 编码
            let bytes = ISO_8859_1
                .encode(value, EncoderTrap::Replace)
                .map_err(|_| {
                    io::Error::new(io::ErrorKind::InvalidData, "Invalid ISO-8859-1 data")
                })?;
            out.write_all(&bytes)
        }
    }

    /// Get the encoded size of a Unicode string
    pub fn get_encoded_size(value: &str) -> usize {
        let mut result = 2 + 1; // n_chars (2 bytes) + flag (1 byte)
        result += value.len() * (if Self::has_multibyte(value) { 2 } else { 1 });
        result
    }

    /// Put compressed Unicode into byte array
    /// ：使用 ISO-8859-1 编码
    pub fn put_compressed_unicode_bytes(
        input: &str,
        output: &mut [u8],
        offset: usize,
    ) -> io::Result<()> {
        let encoded = ISO_8859_1
            .encode(input, EncoderTrap::Replace)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid ISO-8859-1 data"))?;

        output[offset..offset + encoded.len()].copy_from_slice(&encoded);
        Ok(())
    }

    /// Put compressed Unicode into output stream
    pub fn put_compressed_unicode(input: &str, out: &mut dyn LittleEndianOutput) -> io::Result<()> {
        // ISO-8859-1 编码
        let bytes = ISO_8859_1
            .encode(input, EncoderTrap::Replace)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid ISO-8859-1 data"))?;
        out.write_bytes(&bytes)
    }

    /// Put Unicode LE into byte array
    pub fn put_unicode_le_bytes(input: &str, output: &mut [u8], offset: usize) -> io::Result<()> {
        let bytes = Self::get_to_unicode_le(input)?;
        let len = min(bytes.len(), output.len() - offset);
        output[offset..offset + len].copy_from_slice(&bytes[..len]);
        Ok(())
    }

    /// Put Unicode LE into output stream
    pub fn put_unicode_le(input: &str, out: &mut dyn LittleEndianOutput) -> io::Result<()> {
        let bytes = Self::get_to_unicode_le(input)?;
        out.write_bytes(&bytes)
    }

    /// Read Unicode LE from input stream
    /// ：使用 UTF_16LE 解码器
    pub fn read_unicode_le(in_: &mut dyn LittleEndianInput, n_chars: usize) -> io::Result<String> {
        let max_len = Self::get_max_record_length();
        let buf_len = min(n_chars * 2, max_len);
        let mut buf = vec![0u8; buf_len];

        in_.read_fully(&mut buf);

        // ：使用 Replace 策略
        UTF_16LE.decode(&buf, DecoderTrap::Replace).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Failed to decode UTF-16LE: {:?}", e),
            )
        })
    }

    /// Get preferred encoding
    pub fn get_preferred_encoding() -> &'static str {
        "ISO-8859-1"
    }

    /// Check if the string has multibyte character

    pub fn has_multibyte(value: &str) -> bool {
        value.chars().any(|c| c as u32 > 0xFF)
    }

    /// Tests if the string starts with the specified prefix, ignoring case consideration.
    pub fn starts_with_ignore_case(haystack: &str, prefix: &str) -> bool {
        if haystack.len() < prefix.len() {
            return false;
        }
        haystack[..prefix.len()].eq_ignore_ascii_case(prefix)
    }

    /// Tests if the string ends with the specified suffix, ignoring case consideration.
    pub fn ends_with_ignore_case(haystack: &str, suffix: &str) -> bool {
        if haystack.len() < suffix.len() {
            return false;
        }
        let start = haystack.len() - suffix.len();
        haystack[start..].eq_ignore_ascii_case(suffix)
    }

    /// Convert character to lowercase
    pub fn to_lowercase_char(c: char) -> String {
        c.to_lowercase().collect()
    }

    /// Convert character to uppercase
    pub fn to_uppercase_char(c: char) -> String {
        c.to_uppercase().collect()
    }

    /// Check if character is uppercase
    pub fn is_uppercase_char(c: char) -> bool {
        c.to_uppercase().next() == Some(c)
    }

    /// Map Microsoft codepoint string to Unicode

    pub fn map_ms_codepoint_string(string: &str) -> String {
        if string.is_empty() {
            return String::new();
        }

        let mut result = String::with_capacity(string.len());

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
    pub fn join<T: ToString>(array: &[T], separator: &str) -> String {
        if array.is_empty() {
            return String::new();
        }

        let mut result = array[0].to_string();

        for i in 1..array.len() {
            result.push_str(separator);
            result.push_str(&array[i].to_string());
        }

        result
    }

    /// Join array without separator
    pub fn join_simple<T: ToString>(array: &[T]) -> String {
        let mut result = String::new();

        for item in array {
            result.push_str(&item.to_string());
        }

        result
    }

    /// Count number of occurrences of needle in haystack
    pub fn count_matches(haystack: &str, needle: char) -> usize {
        haystack.chars().filter(|&c| c == needle).count()
    }

    /// Get from Unicode LE with null termination
    pub fn get_from_unicode_le_0_terminated(string: &[u8], offset: usize, len: usize) -> String {
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

            // 检查下一个字符是否有效
            let cp = if len > 1 && offset + 3 < string.len() {
                let low = string[offset + 2] as u16;
                let high = string[offset + 3] as u16;
                (high << 8) | low
            } else {
                0
            };

            let new_max_len = if Self::is_identifier_part(cp as u32) {
                len - 1
            } else {
                0
            };

            (new_offset, prefix, new_max_len)
        } else {
            (offset, "", len)
        };

        let mut new_len = 0;

        // 查找空终止符
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

        let mut result = String::from(prefix);
        if new_len > 0 {
            result.push_str(&Self::get_from_unicode_le(string, new_offset, new_len));
        }

        result
    }

    /// Get length of CharSequence or 0 if null
    pub fn length(cs: Option<&str>) -> usize {
        cs.map_or(0, |s| s.len())
    }

    /// Check if CharSequence is blank (empty, null, or whitespace only)
    pub fn is_blank(cs: Option<&str>) -> bool {
        match cs {
            Some(s) => s.chars().all(|c| c.is_whitespace()),
            None => true,
        }
    }

    /// Check if CharSequence is not blank
    pub fn is_not_blank(cs: Option<&str>) -> bool {
        !Self::is_blank(cs)
    }

    /// Repeat character n times
    pub fn repeat(ch: char, repeat: usize) -> String {
        if repeat == 0 {
            return String::new();
        }
        String::from_iter(std::iter::repeat(ch).take(repeat))
    }

    // Helper method
    fn is_identifier_part(cp: u32) -> bool {
        match cp {
            0..=0x7F => {
                // ASCII range
                let c = cp as u8 as char;
                c.is_alphanumeric() || c == '_' || c == '$'
            }
            _ => {
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
    ' ' as u32, '!' as u32, 8704, '#' as u32, 8707, '%' as u32, '&' as u32, 8717, '(' as u32,
    ')' as u32, 8727, '+' as u32, ',' as u32, 8722, '.' as u32, '/' as u32, '0' as u32, '1' as u32,
    '2' as u32, '3' as u32, '4' as u32, '5' as u32, '6' as u32, '7' as u32, '8' as u32, '9' as u32,
    ':' as u32, ';' as u32, '<' as u32, '=' as u32, '>' as u32, '?' as u32, 8773, 913, 914, 935,
    916, 917, 934, 915, 919, 921, 977, 922, 923, 924, 925, 927, 928, 920, 929, 931, 932, 933, 962,
    937, 926, 936, 918, '[' as u32, 8765, ']' as u32, 8869, '_' as u32, ' ' as u32, 945, 946, 967,
    948, 949, 966, 947, 951, 953, 981, 954, 955, 956, 957, 959, 960, 952, 961, 963, 964, 965, 982,
    969, 958, 968, 950, '{' as u32, '|' as u32, '}' as u32, 8764, ' ' as u32,
];

static SYMBOL_MAP_F0A0: [u32; 96] = [
    8364, 978, 8242, 8804, 8260, 8734, 402, 9827, 9830, 9829, 9824, 8596, 8591, 8593, 8594, 8595,
    176, 177, 8243, 8805, 215, 181, 8706, 8729, 247, 8800, 8801, 8776, 8230, 9168, 9135, 8629,
    8501, 8475, 8476, 8472, 8855, 8853, 8709, 8745, 8746, 8835, 8839, 8836, 8834, 8838, 8712, 8713,
    8736, 8711, 174, 169, 8482, 8719, 8730, 8901, 172, 8743, 8744, 8660, 8656, 8657, 8658, 8659,
    9674, 9001, 174, 169, 8482, 8721, 9115, 9116, 9117, 9121, 9122, 9123, 9127, 9128, 9129, 9130,
    ' ' as u32, 9002, 8747, 8992, 9134, 8993, 9118, 9119, 9120, 9124, 9125, 9126, 9131, 9132, 9133,
    ' ' as u32,
];
