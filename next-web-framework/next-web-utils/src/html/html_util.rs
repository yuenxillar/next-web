use std::collections::HashSet;

pub struct HtmlUtil;

impl HtmlUtil {
    /// 清除指定HTML标签和被标签包围的内容
    pub fn remove_html_tag(html: &str, tag: &str) -> String {
        let mut result = String::new();
        let mut in_tag = false;
        let mut buffer = String::new();
        let tag_start = format!("<{}", tag);
        let tag_end = format!("</{}>", tag);
        let mut tag_depth = 0;

        let mut chars = html.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '<' {
                if let Some(next) = chars.peek() {
                    if *next == '/' {
                        // 检查是否是结束标签
                        let mut potential_end = String::from("</");
                        let mut temp_chars = chars.clone();
                        temp_chars.next(); // 跳过 '/'
                        
                        for _ in 0..tag.len() {
                            if let Some(tc) = temp_chars.next() {
                                potential_end.push(tc);
                            }
                        }
                        
                        if potential_end == tag_end {
                            tag_depth -= 1;
                            if tag_depth == 0 {
                                in_tag = false;
                                // 跳过整个结束标签
                                for _ in 0..(tag.len() + 3) { // </tag>
                                    chars.next();
                                }
                                buffer.clear();
                                continue;
                            }
                        }
                    } else {
                        // 检查是否是开始标签
                        let mut potential_start = String::from("<");
                        let mut temp_chars = chars.clone();
                        
                        for _ in 0..tag.len() {
                            if let Some(tc) = temp_chars.next() {
                                potential_start.push(tc);
                            }
                        }
                        
                        if potential_start == tag_start {
                            // 检查是否是自闭合标签
                            let mut is_self_closing = false;
                            let mut temp_chars = chars.clone();
                            let mut tag_content = String::new();
                            
                            while let Some(tc) = temp_chars.next() {
                                tag_content.push(tc);
                                if tc == '>' {
                                    break;
                                }
                            }
                            
                            if tag_content.ends_with("/>") {
                                is_self_closing = true;
                            }
                            
                            if is_self_closing {
                                // 跳过自闭合标签
                                for _ in 0..tag_content.len() {
                                    chars.next();
                                }
                                continue;
                            } else {
                                tag_depth += 1;
                                if tag_depth == 1 {
                                    in_tag = true;
                                    buffer.clear();
                                }
                            }
                        }
                    }
                }
            }

            if !in_tag {
                result.push(c);
            } else {
                buffer.push(c);
            }
        }

        result
    }

    /// 清除所有HTML标签，但是保留标签内的内容
    pub fn clean_html_tag(html: &str) -> String {
        let mut result = String::new();
        let mut in_tag = false;

        for c in html.chars() {
            if c == '<' {
                in_tag = true;
                continue;
            }
            if c == '>' {
                in_tag = false;
                continue;
            }
            if !in_tag {
                result.push(c);
            }
        }

        result
    }

    pub fn unwrap_html_tag(html: &str, tag: &str) -> String {
        let mut result = String::new();
        let mut buffer = String::new();
        let mut in_tag = false;
        let mut in_target_tag = false;
        let mut tag_depth = 0;
        
        let open_tag = format!("<{}", tag);
        let close_tag = format!("</{}>", tag);
    
        let mut chars = html.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '<' {
                if in_tag {
                    buffer.push(c);
                    continue;
                }
                
                in_tag = true;
                buffer.clear();
                buffer.push(c);
                continue;
            }
            
            if in_tag {
                buffer.push(c);
                
                if c == '>' {
                    in_tag = false;
                    
                    if buffer.starts_with(&open_tag) {
                        if buffer.ends_with("/>") {
                            // 自闭合标签，直接跳过
                            continue;
                        } else {
                            // 开始标签
                            tag_depth += 1;
                            in_target_tag = true;
                        }
                    } else if buffer.starts_with(&close_tag) {
                        // 结束标签
                        tag_depth -= 1;
                        if tag_depth == 0 {
                            in_target_tag = false;
                        }
                    } else if in_target_tag {
                        // 在目标标签内的其他标签，保留内容
                        result.push_str(&buffer);
                    }
                    
                    buffer.clear();
                    continue;
                }
            } else {
                if in_target_tag || !in_tag {
                    result.push(c);
                }
            }
        }
    
        result
    }

    
    /// 去除HTML标签中的指定属性
    pub fn remove_html_attr(html: &str, attr: &str) -> String {
        let mut result = String::new();
        let mut in_tag = false;
        let mut in_attr = false;
        let mut current_attr = String::new();
        let mut skip_until = None;

        for c in html.chars() {
            if skip_until == Some(c) {
                skip_until = None;
                continue;
            }

            if c == '<' && !in_tag {
                in_tag = true;
                result.push(c);
                continue;
            }

            if c == '>' && in_tag {
                in_tag = false;
                result.push(c);
                continue;
            }

            if in_tag {
                if c == ' ' && !in_attr {
                    result.push(c);
                    continue;
                }

                if c == '=' && in_attr {
                    if current_attr == attr {
                        // 跳过属性值
                        skip_until = Some(' ');
                        in_attr = false;
                        current_attr.clear();
                        continue;
                    } else {
                        result.push_str(&current_attr);
                        result.push('=');
                        current_attr.clear();
                        in_attr = false;
                        continue;
                    }
                }

                if in_attr {
                    current_attr.push(c);
                } else {
                    if c == ' ' {
                        if !current_attr.is_empty() {
                            if current_attr != attr {
                                result.push_str(&current_attr);
                            }
                            current_attr.clear();
                        }
                        result.push(' ');
                    } else {
                        in_attr = true;
                        current_attr.push(c);
                    }
                }
            } else {
                result.push(c);
            }
        }

        if !current_attr.is_empty() && current_attr != attr {
            result.push_str(&current_attr);
        }

        result
    }

    /// 去除指定标签的所有属性
    pub fn remove_all_html_attr(html: &str, tag: &str) -> String {
        let mut result = String::new();
        let tag_start = format!("<{}", tag);
        let mut in_target_tag = false;
        let mut in_tag = false;
        let mut in_attr = false;

        let mut chars = html.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '<' {
                // 检查是否是目标标签
                let mut is_target = false;
                let mut temp_chars = chars.clone();
                let mut potential_tag = String::from("<");
                
                for _ in 0..tag.len() {
                    if let Some(tc) = temp_chars.next() {
                        potential_tag.push(tc);
                    }
                }
                
                if potential_tag == tag_start {
                    is_target = true;
                }
                
                in_tag = true;
                in_target_tag = is_target;
                result.push(c);
                
                if is_target {
                    // 跳过标签名
                    for _ in 0..tag.len() {
                        if let Some(tc) = chars.next() {
                            result.push(tc);
                        }
                    }
                    
                    // 跳过所有属性直到 '>'
                    while let Some(tc) = chars.peek() {
                        if *tc == '>' {
                            break;
                        }
                        chars.next();
                    }
                }
                continue;
            }

            if c == '>' && in_tag {
                in_tag = false;
                in_target_tag = false;
                result.push(c);
                continue;
            }

            if !in_target_tag {
                result.push(c);
            }
        }

        result
    }

    /// 过滤HTML文本，防止XSS攻击
    pub fn filter(html: &str) -> String {
        let allowed_tags: HashSet<&str> = [
            "a", "abbr", "acronym", "address", "area", "b", "big", "blockquote", "br", "button",
            "caption", "center", "cite", "code", "col", "colgroup", "dd", "del", "dfn", "dir",
            "div", "dl", "dt", "em", "fieldset", "font", "form", "h1", "h2", "h3", "h4", "h5", "h6",
            "hr", "i", "img", "input", "ins", "kbd", "label", "legend", "li", "map", "menu", "ol",
            "optgroup", "option", "p", "pre", "q", "s", "samp", "select", "small", "span", "strike",
            "strong", "sub", "sup", "table", "tbody", "td", "textarea", "tfoot", "th", "thead", "tr",
            "tt", "u", "ul", "var",
        ].iter().cloned().collect();

        let mut result = String::new();
        let mut current_tag = String::new();
        let mut in_tag = false;

        for c in html.chars() {
            if c == '<' {
                in_tag = true;
                current_tag.clear();
                continue;
            }

            if c == '>' && in_tag {
                in_tag = false;
                let tag_name = current_tag.split_whitespace().next().unwrap_or("");
                if allowed_tags.contains(tag_name) {
                    result.push('<');
                    result.push_str(&current_tag);
                    result.push('>');
                }
                current_tag.clear();
                continue;
            }

            if in_tag {
                current_tag.push(c);
            } else {
                result.push(c);
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remove_html_tag() {
        let html = "pre<img src=\"xxx/dfdsfds/test.jpg\">";
        assert_eq!(HtmlUtil::remove_html_tag(html, "img"), "pre");

        let html = "pre<div>content</div>";
        assert_eq!(HtmlUtil::remove_html_tag(html, "div"), "pre");

        let html = "pre<div><div>nested</div></div>";
        assert_eq!(HtmlUtil::remove_html_tag(html, "div"), "pre");
    }

    #[test]
    fn test_clean_html_tag() {
        let html = "pre<div class=\"test_div\">\r\n\t\tdfdsfdsfdsf\r\n</div><div class=\"test_div\">BBBB</div>";
        assert_eq!(HtmlUtil::clean_html_tag(html), "pre\r\n\t\tdfdsfdsfdsf\r\nBBBB");
    }

    #[test]
    fn test_unwrap_html_tag() {
        let html = "pre<div class=\"test_div\">abc</div>";
        assert_eq!(HtmlUtil::unwrap_html_tag(html, "div"), "preabc");
        
        let html_nested = "pre<div>outer<div>inner</div></div>";
        assert_eq!(HtmlUtil::unwrap_html_tag(html_nested, "div"), "preouterinner");
        
        let html_self_closing = "pre<img src=\"test.jpg\"/>";
        assert_eq!(HtmlUtil::unwrap_html_tag(html_self_closing, "img"), "pre");
    }

    #[test]
    fn test_remove_html_attr() {
        let html = "<div class=\"test_div\"></div><span class=\"test_div\"></span>";
        assert_eq!(HtmlUtil::remove_html_attr(html, "class"), "<div></div><span></span>");
    }

    #[test]
    fn test_remove_all_html_attr() {
        let html = "<div class=\"test_div\" width=\"120\"></div>";
        assert_eq!(HtmlUtil::remove_all_html_attr(html, "div"), "<div></div>");
    }

    #[test]
    fn test_filter() {
        let html = "<alert></alert><script>malicious()</script><p>safe</p>";
        assert_eq!(HtmlUtil::filter(html), "<p>safe</p>");
    }
}