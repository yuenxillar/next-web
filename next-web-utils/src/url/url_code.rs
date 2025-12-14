pub struct UrlCode;

/// URL的组成部分
#[derive(Debug, Clone, Default)]
pub struct UrlComponents {
    /// 协议，如 "http", "https"
    pub scheme: String,
    /// 用户名
    pub username: String,
    /// 密码
    pub password: String,
    /// 主机名，如 "example.com"
    pub host: String,
    /// 端口号
    pub port: Option<u16>,
    /// 路径，如 "/path/to/resource"
    pub path: String,
    /// 查询参数
    pub query: Vec<(String, String)>,
    /// 片段标识符
    pub fragment: String,
}

impl UrlCode {
    /// URL编码
    /// 将字符串转换为URL编码格式
    pub fn encode(s: &str) -> String {
        let mut result = String::new();
        for b in s.bytes() {
            match b {
                // 不需要编码的字符: 字母、数字、'-'、'.'、'_'、'~'
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                    result.push(b as char);
                }
                // 空格转换为加号
                b' ' => result.push('+'),
                // 其他字符转换为%HH格式，HH为十六进制值
                _ => {
                    result.push('%');
                    result.push_str(&format!("{:02X}", b));
                }
            }
        }
        result
    }

    /// URL解码
    /// 将URL编码的字符串转换回普通字符串
    pub fn decode(s: &str) -> Result<String, String> {
        let mut result = Vec::new(); // 存储解码后的字节
        let mut i = 0;
        let bytes = s.as_bytes();
        
        while i < bytes.len() {
            match bytes[i] {
                // 加号转换为空格
                b'+' => {
                    result.push(b' ');
                    i += 1;
                }
                // %HH格式
                b'%' => {
                    if i + 2 >= bytes.len() {
                        return Err("URL解码错误: %后没有足够的字符".to_string());
                    }
                    
                    // 解析十六进制值
                    let hex = match (Self::from_hex(bytes[i + 1]), Self::from_hex(bytes[i + 2])) {
                        (Some(h1), Some(h2)) => h1 * 16 + h2,
                        _ => return Err(format!("URL解码错误: 无效的十六进制值 %{}{}", 
                            bytes[i + 1] as char, bytes[i + 2] as char))
                    };
                    
                    // 将解码后的字节添加到结果
                    result.push(hex);
                    i += 3;
                }
                // 其他字符保持不变
                _ => {
                    result.push(bytes[i]);
                    i += 1;
                }
            }
        }
        
        // 将字节向量转换为UTF-8字符串
        match String::from_utf8(result) {
            Ok(s) => Ok(s),
            Err(e) => Err(format!("URL解码错误: 无效的UTF-8序列: {}", e))
        }
    }
    
    /// 将十六进制字符转换为数值
    fn from_hex(c: u8) -> Option<u8> {
        match c {
            b'0'..=b'9' => Some(c - b'0'),
            b'A'..=b'F' => Some(c - b'A' + 10),
            b'a'..=b'f' => Some(c - b'a' + 10),
            _ => None,
        }
    }
    
    /// 编码表单数据中的字符串
    pub fn encode_form_value(s: &str) -> String {
        Self::encode(s)
    }
    
    /// 编码URL路径部分
    pub fn encode_path_segment(s: &str) -> String {
        let mut result = String::new();
        for b in s.bytes() {
            match b {
                // 路径段中允许的字符
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'!' |
                b'$' | b'&' | b'\'' | b'(' | b')' | b'*' | b'+' | b',' | b';' | b'=' => {
                    result.push(b as char);
                }
                // 其他字符编码为%HH
                _ => {
                    result.push('%');
                    result.push_str(&format!("{:02X}", b));
                }
            }
        }
        result
    }
    
    /// 编码URL参数名或值
    pub fn encode_query_param(s: &str) -> String {
        let mut result = String::new();
        for b in s.bytes() {
            match b {
                // 查询参数中允许的字符
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                    result.push(b as char);
                }
                // 空格转换为加号
                b' ' => result.push('+'),
                // 其他字符编码为%HH
                _ => {
                    result.push('%');
                    result.push_str(&format!("{:02X}", b));
                }
            }
        }
        result
    }
    
    /// 判断字符是否需要URL编码
    pub fn needs_encoding(c: char) -> bool {
        let b = c as u8;
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => false,
            _ => true
        }
    }
    
    /// 解析URL中的查询参数，返回键值对
    pub fn parse_query_string(query: &str) -> Result<Vec<(String, String)>, String> {
        let mut params = Vec::new();
        
        if query.is_empty() {
            return Ok(params);
        }
        
        for param in query.split('&') {
            if param.is_empty() {
                continue;
            }
            
            let parts: Vec<&str> = param.split('=').collect();
            match parts.len() {
                1 => {
                    // 没有值的参数，如 "param"
                    let key = Self::decode(parts[0])?;
                    params.push((key, String::new()));
                },
                2 => {
                    // 正常的键值对，如 "param=value"
                    let key = Self::decode(parts[0])?;
                    let value = Self::decode(parts[1])?;
                    params.push((key, value));
                },
                _ => {
                    // 处理 "param=value=something" 的情况，把第一个等号后面的全部作为值
                    let key = Self::decode(parts[0])?;
                    let value = Self::decode(&param[parts[0].len()+1..])?;
                    params.push((key, value));
                }
            }
        }
        
        Ok(params)
    }
    
    /// 构建查询字符串
    pub fn build_query_string(params: &[(String, String)]) -> String {
        let mut query = String::new();
        
        for (i, (key, value)) in params.iter().enumerate() {
            if i > 0 {
                query.push('&');
            }
            
            query.push_str(&Self::encode_query_param(key));
            
            if !value.is_empty() {
                query.push('=');
                query.push_str(&Self::encode_query_param(value));
            }
        }
        
        query
    }
    
    /// 解析完整URL，拆分为各个组成部分
    pub fn parse_url(url: &str) -> Result<UrlComponents, String> {
        let mut components = UrlComponents::default();
        let mut remaining = url;
        
        // 解析协议 (scheme)
        if let Some(pos) = remaining.find("://") {
            components.scheme = remaining[..pos].to_lowercase();
            remaining = &remaining[pos + 3..];
        } else {
            return Err("URL格式错误: 缺少协议部分".to_string());
        }
        
        // 解析认证信息 (用户名和密码)
        if let Some(pos) = remaining.find('@') {
            let auth = &remaining[..pos];
            remaining = &remaining[pos + 1..];
            
            if let Some(pos_colon) = auth.find(':') {
                components.username = auth[..pos_colon].to_string();
                match Self::decode(&auth[pos_colon + 1..]) {
                    Ok(pwd) => components.password = pwd,
                    Err(e) => return Err(format!("URL解析错误: 密码解码失败: {}", e)),
                }
            } else {
                components.username = auth.to_string();
            }
        }
        
        // 解析主机和端口
        let path_start = remaining.find('/').unwrap_or(remaining.len());
        let query_start = remaining.find('?').unwrap_or(remaining.len());
        let fragment_start = remaining.find('#').unwrap_or(remaining.len());
        
        let authority_end = path_start.min(query_start).min(fragment_start);
        let authority = &remaining[..authority_end];
        
        if let Some(pos) = authority.rfind(':') {
            components.host = authority[..pos].to_string();
            match authority[pos + 1..].parse::<u16>() {
                Ok(port) => components.port = Some(port),
                Err(_) => return Err("URL格式错误: 端口号无效".to_string()),
            }
        } else {
            components.host = authority.to_string();
            // 根据协议设置默认端口
            components.port = match components.scheme.as_str() {
                "http" => Some(80),
                "https" => Some(443),
                "ftp" => Some(21),
                _ => None,
            };
        }
        
        remaining = &remaining[authority_end..];
        
        // 解析路径
        if path_start < query_start && path_start < fragment_start {
            let path_end = query_start.min(fragment_start);
            components.path = remaining[..path_end].to_string();
            remaining = &remaining[path_end..];
        } else {
            components.path = "/".to_string();
        }
        
        // 解析查询参数
        if query_start < fragment_start && query_start < remaining.len() {
            let query_end = fragment_start;
            let query_str = &remaining[1..query_end]; // 跳过开头的 '?'
            match Self::parse_query_string(query_str) {
                Ok(params) => components.query = params,
                Err(e) => return Err(format!("URL解析错误: 查询参数解析失败: {}", e)),
            }
            remaining = &remaining[query_end..];
        }
        
        // 解析片段标识符
        if fragment_start < remaining.len() {
            components.fragment = remaining[1..].to_string(); // 跳过开头的 '#'
        }
        
        Ok(components)
    }
    
    /// 根据各组成部分构建完整URL
    pub fn build_url(components: &UrlComponents) -> String {
        let mut url = String::new();
        
        // 添加协议
        if !components.scheme.is_empty() {
            url.push_str(&components.scheme);
            url.push_str("://");
        }
        
        // 添加认证信息
        if !components.username.is_empty() {
            url.push_str(&Self::encode(&components.username));
            
            if !components.password.is_empty() {
                url.push(':');
                url.push_str(&Self::encode(&components.password));
            }
            
            url.push('@');
        }
        
        // 添加主机
        url.push_str(&components.host);
        
        // 添加端口
        if let Some(port) = components.port {
            // 只有当端口不是协议的默认端口时才添加
            let is_default_port = match components.scheme.as_str() {
                "http" => port == 80,
                "https" => port == 443,
                "ftp" => port == 21,
                _ => false,
            };
            
            if !is_default_port {
                url.push(':');
                url.push_str(&port.to_string());
            }
        }
        
        // 添加路径
        if components.path.is_empty() {
            url.push('/');
        } else if !components.path.starts_with('/') {
            url.push('/');
            url.push_str(&components.path);
        } else {
            url.push_str(&components.path);
        }
        
        // 添加查询参数
        if !components.query.is_empty() {
            url.push('?');
            url.push_str(&Self::build_query_string(&components.query));
        }
        
        // 添加片段标识符
        if !components.fragment.is_empty() {
            url.push('#');
            url.push_str(&components.fragment);
        }
        
        url
    }
    
    /// 提取URL的域名部分
    pub fn get_domain(url: &str) -> Result<String, String> {
        let components = Self::parse_url(url)?;
        Ok(components.host)
    }
    
    /// 判断URL是否为绝对URL（包含协议部分）
    pub fn is_absolute_url(url: &str) -> bool {
        url.contains("://")
    }
    
    /// 提取URL的path部分
    pub fn get_path(url: &str) -> Result<String, String> {
        let components = Self::parse_url(url)?;
        Ok(components.path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode() {
        println!("{}", UrlCode::encode("http://www.baidu.com/a/b/c?d=e&f=g"));
        assert_eq!(UrlCode::encode("Hello World"), "Hello+World");
        assert_eq!(UrlCode::encode("hello@example.com"), "hello%40example.com");
        assert_eq!(UrlCode::encode("你好"), "%E4%BD%A0%E5%A5%BD");
    }

    #[test]
    fn test_decode() {
        assert_eq!(UrlCode::decode("Hello+World").unwrap(), "Hello World");
        assert_eq!(UrlCode::decode("hello%40example.com").unwrap(), "hello@example.com");
        assert_eq!(UrlCode::decode("%E4%BD%A0%E5%A5%BD").unwrap(), "你好");
    }
    
    #[test]
    fn test_decode_error() {
        assert!(UrlCode::decode("%").is_err());
        assert!(UrlCode::decode("%1").is_err());
        assert!(UrlCode::decode("%XY").is_err());
        assert!(UrlCode::decode("%FF%FF").is_err()); // 无效的UTF-8序列
    }
    
    #[test]
    fn test_parse_query_string() {
        let query = "name=%E5%BC%A0%E4%B8%89&age=25&hobby=coding&hobby=reading";
        let params = UrlCode::parse_query_string(query).unwrap();
        
        assert_eq!(params.len(), 4);
        assert_eq!(params[0], ("name".to_string(), "张三".to_string()));
        assert_eq!(params[1], ("age".to_string(), "25".to_string()));
        assert_eq!(params[2], ("hobby".to_string(), "coding".to_string()));
        assert_eq!(params[3], ("hobby".to_string(), "reading".to_string()));
        
        // 测试空查询字符串
        assert_eq!(UrlCode::parse_query_string("").unwrap().len(), 0);
        
        // 测试没有值的参数
        let params = UrlCode::parse_query_string("param1&param2=value").unwrap();
        assert_eq!(params.len(), 2);
        assert_eq!(params[0], ("param1".to_string(), "".to_string()));
        assert_eq!(params[1], ("param2".to_string(), "value".to_string()));
        
        // 测试多个等号的情况
        let params = UrlCode::parse_query_string("key=value=something").unwrap();
        assert_eq!(params.len(), 1);
        assert_eq!(params[0], ("key".to_string(), "value=something".to_string()));
    }
    
    #[test]
    fn test_build_query_string() {
        let params = vec![
            ("name".to_string(), "张三".to_string()),
            ("age".to_string(), "25".to_string()),
            ("hobby".to_string(), "coding".to_string()),
            ("hobby".to_string(), "reading".to_string()),
        ];
        
        let query = UrlCode::build_query_string(&params);
        assert_eq!(query, "name=%E5%BC%A0%E4%B8%89&age=25&hobby=coding&hobby=reading");
        
        // 测试没有值的参数
        let params = vec![
            ("param1".to_string(), "".to_string()),
            ("param2".to_string(), "value".to_string()),
        ];
        let query = UrlCode::build_query_string(&params);
        assert_eq!(query, "param1&param2=value");
    }
    
    #[test]
    fn test_parse_url() {
        let url = "https://user:pass@example.com:8080/path/to/resource?name=value&foo=bar#section";
        let components = UrlCode::parse_url(url).unwrap();
        
        assert_eq!(components.scheme, "https");
        assert_eq!(components.username, "user");
        assert_eq!(components.password, "pass");
        assert_eq!(components.host, "example.com");
        assert_eq!(components.port, Some(8080));
        assert_eq!(components.path, "/path/to/resource");
        assert_eq!(components.query.len(), 2);
        assert_eq!(components.query[0], ("name".to_string(), "value".to_string()));
        assert_eq!(components.query[1], ("foo".to_string(), "bar".to_string()));
        assert_eq!(components.fragment, "section");
        
        // 测试没有用户名密码的URL
        let url = "http://example.com/path";
        let components = UrlCode::parse_url(url).unwrap();
        assert_eq!(components.username, "");
        assert_eq!(components.password, "");
        
        // 测试默认端口
        let url = "https://example.com/path";
        let components = UrlCode::parse_url(url).unwrap();
        assert_eq!(components.port, Some(443));
    }
    
    #[test]
    fn test_build_url() {
        let mut components = UrlComponents::default();
        components.scheme = "https".to_string();
        components.username = "user".to_string();
        components.password = "pass".to_string();
        components.host = "example.com".to_string();
        components.port = Some(8080);
        components.path = "/path/to/resource".to_string();
        components.query = vec![
            ("name".to_string(), "value".to_string()),
            ("foo".to_string(), "bar".to_string()),
        ];
        components.fragment = "section".to_string();
        
        let url = UrlCode::build_url(&components);
        assert_eq!(url, "https://user:pass@example.com:8080/path/to/resource?name=value&foo=bar#section");
        
        // 测试默认端口不会被添加到URL中
        components.port = Some(443); // HTTPS的默认端口
        let url = UrlCode::build_url(&components);
        assert_eq!(url, "https://user:pass@example.com/path/to/resource?name=value&foo=bar#section");
    }
}