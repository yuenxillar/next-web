use std::collections::HashMap;

/// 摩尔斯电码工具类
pub struct MorseCodeUtil;

impl MorseCodeUtil {
    /// 获取摩尔斯电码映射表
    fn get_morse_map() -> HashMap<char, &'static str> {
        let mut map = HashMap::new();
        // 字母
        map.insert('A', ".-");
        map.insert('B', "-...");
        map.insert('C', "-.-.");
        map.insert('D', "-..");
        map.insert('E', ".");
        map.insert('F', "..-.");
        map.insert('G', "--.");
        map.insert('H', "....");
        map.insert('I', "..");
        map.insert('J', ".---");
        map.insert('K', "-.-");
        map.insert('L', ".-..");
        map.insert('M', "--");
        map.insert('N', "-.");
        map.insert('O', "---");
        map.insert('P', ".--.");
        map.insert('Q', "--.-");
        map.insert('R', ".-.");
        map.insert('S', "...");
        map.insert('T', "-");
        map.insert('U', "..-");
        map.insert('V', "...-");
        map.insert('W', ".--");
        map.insert('X', "-..-");
        map.insert('Y', "-.--");
        map.insert('Z', "--..");
        // 数字
        map.insert('0', "-----");
        map.insert('1', ".----");
        map.insert('2', "..---");
        map.insert('3', "...--");
        map.insert('4', "....-");
        map.insert('5', ".....");
        map.insert('6', "-....");
        map.insert('7', "--...");
        map.insert('8', "---..");
        map.insert('9', "----.");
        // 标点符号
        map.insert('.', ".-.-.-");
        map.insert(',', "--..--");
        map.insert('?', "..--..");
        map.insert('\'', ".----.");
        map.insert('!', "-.-.--");
        map.insert('/', "-..-.");
        map.insert('(', "-.--.");
        map.insert(')', "-.--.-");
        map.insert('&', ".-...");
        map.insert(':', "---...");
        map.insert(';', "-.-.-.");
        map.insert('=', "-...-");
        map.insert('+', ".-.-.");
        map.insert('-', "-....-");
        map.insert('_', "..--.-");
        map.insert('"', ".-..-.");
        map.insert('$', "...-..-");
        map.insert('@', ".--.-.");
        map.insert(' ', "/");
        map
    }

    /// 获取反向摩尔斯电码映射表
    fn get_reverse_morse_map() -> HashMap<&'static str, char> {
        let mut map = HashMap::new();
        for (key, value) in Self::get_morse_map() {
            map.insert(value, key);
        }
        map
    }

    /// 将文本编码为摩尔斯电码
    pub fn encode(text: &str) -> String {
        let morse_map = Self::get_morse_map();
        let mut result = String::new();
        
        for c in text.to_uppercase().chars() {
            if let Some(code) = morse_map.get(&c) {
                result.push_str(code);
                result.push(' ');
            }
        }
        
        result.trim().to_string()
    }

    /// 将摩尔斯电码解码为文本
    pub fn decode(code: &str) -> String {
        let reverse_map = Self::get_reverse_morse_map();
        let mut result = String::new();
        
        for word in code.split('/') {
            for symbol in word.trim().split_whitespace() {
                if let Some(c) = reverse_map.get(symbol) {
                    result.push(*c);
                }
            }
            result.push(' ');
        }
        
        result.trim().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode() {
        assert_eq!(MorseCodeUtil::encode("HELLO"), ".... . .-.. .-.. ---");
        assert_eq!(MorseCodeUtil::encode("123"), ".---- ..--- ...--");
        assert_eq!(MorseCodeUtil::encode("HELLO WORLD"), ".... . .-.. .-.. --- / .-- --- .-. .-.. -..");
    }

    #[test]
    fn test_decode() {
        assert_eq!(MorseCodeUtil::decode(".... . .-.. .-.. ---"), "HELLO");
        assert_eq!(MorseCodeUtil::decode(".---- ..--- ...--"), "123");
        assert_eq!(MorseCodeUtil::decode(".... . .-.. .-.. --- / .-- --- .-. .-.. -.."), "HELLO WORLD");
    }

    #[test]
    fn test_encode_decode() {
        let text = "HELLO WORLD 123!";
        let encoded = MorseCodeUtil::encode(text);
        let decoded = MorseCodeUtil::decode(&encoded);
        assert_eq!(decoded, text);
    }
}
