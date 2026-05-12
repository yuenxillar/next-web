use std::collections::HashSet;

pub struct HtmlUtil;

impl HtmlUtil {
    /// Removes empty HTML tag pairs such as `<p></p>` and `<span> </span>`.
    #[must_use]
    pub fn clean_empty_tag(content: &str) -> String {
        let mut current = content.to_string();

        loop {
            let next = remove_one_empty_tag_pass(&current);
            if next == current {
                return next;
            }
            current = next;
        }
    }

    /// Removes all HTML tags but keeps their text content.
    #[must_use]
    pub fn clean_html_tag(content: &str) -> String {
        strip_tags(content, false)
    }

    /// Escapes text for safe HTML output.
    #[must_use]
    pub fn escape(text: &str) -> String {
        let mut escaped = String::with_capacity(text.len());
        for ch in text.chars() {
            match ch {
                '\'' => escaped.push_str("&#039;"),
                '"' => escaped.push_str("&quot;"),
                '&' => escaped.push_str("&amp;"),
                '<' => escaped.push_str("&lt;"),
                '>' => escaped.push_str("&gt;"),
                _ => escaped.push(ch),
            }
        }
        escaped
    }

    /// Unescapes common HTML entities.
    #[must_use]
    pub fn unescape(html_str: &str) -> String {
        let mut result = html_str.to_string();
        let entities = [
            ("&#039;", "'"),
            ("&apos;", "'"),
            ("&quot;", "\""),
            ("&lt;", "<"),
            ("&gt;", ">"),
            ("&nbsp;", " "),
            ("&amp;", "&"),
        ];

        for (from, to) in entities {
            result = result.replace(from, to);
        }

        result
    }

    /// Filters HTML content to reduce XSS risk.
    ///
    /// Dangerous elements are removed with their content. Event attributes,
    /// style attributes, and `javascript:`/`data:` URLs are removed from allowed
    /// tags.
    #[must_use]
    pub fn filter(html_content: &str) -> String {
        let blocked_tags = tag_set(&[
            "script", "style", "iframe", "frame", "frameset", "object", "embed", "applet", "meta",
            "link", "base", "form",
        ]);
        let allowed_tags = tag_set(&[
            "a",
            "abbr",
            "b",
            "blockquote",
            "br",
            "caption",
            "cite",
            "code",
            "col",
            "colgroup",
            "dd",
            "del",
            "div",
            "dl",
            "dt",
            "em",
            "h1",
            "h2",
            "h3",
            "h4",
            "h5",
            "h6",
            "hr",
            "i",
            "img",
            "ins",
            "li",
            "ol",
            "p",
            "pre",
            "q",
            "s",
            "small",
            "span",
            "strong",
            "sub",
            "sup",
            "table",
            "tbody",
            "td",
            "tfoot",
            "th",
            "thead",
            "tr",
            "u",
            "ul",
        ]);
        let global_attrs = attr_set(&[
            "class", "id", "title", "alt", "width", "height", "src", "href",
        ]);

        let without_blocked = remove_html_tag_internal(html_content, true, &blocked_tags);
        sanitize_tags(&without_blocked, &allowed_tags, &global_attrs)
    }

    /// Removes all attributes from the specified tags.
    #[must_use]
    pub fn remove_all_html_attr(content: &str, tag_names: &[&str]) -> String {
        let tags = tag_set(tag_names);
        transform_tags(content, |tag| {
            if !tag.is_end && tags.contains(tag.name.as_str()) {
                tag.render_without_attrs()
            } else {
                tag.original
            }
        })
    }

    /// Removes attributes by name from all tags.
    #[must_use]
    pub fn remove_html_attr(content: &str, attrs: &[&str]) -> String {
        let attrs = attr_set(attrs);
        transform_tags(content, |mut tag| {
            if tag.is_end {
                return tag.original;
            }
            tag.attrs.retain(|attr| !attrs.contains(attr.name.as_str()));
            tag.render()
        })
    }

    /// Removes the specified HTML tags and, optionally, their wrapped content.
    #[must_use]
    pub fn remove_html_tag_with_content(
        content: &str,
        with_tag_content: bool,
        tag_names: &[&str],
    ) -> String {
        let tags = tag_set(tag_names);
        remove_html_tag_internal(content, with_tag_content, &tags)
    }

    /// Removes the specified HTML tags and their wrapped content.
    #[must_use]
    pub fn remove_html_tag(content: &str, tag_names: &[&str]) -> String {
        Self::remove_html_tag_with_content(content, true, tag_names)
    }

    /// Removes the specified HTML tags but keeps their wrapped content.
    #[must_use]
    pub fn unwrap_html_tag(content: &str, tag_names: &[&str]) -> String {
        Self::remove_html_tag_with_content(content, false, tag_names)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct HtmlTag {
    original: String,
    name: String,
    attrs: Vec<HtmlAttr>,
    is_end: bool,
    is_self_closing: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct HtmlAttr {
    name: String,
    value: Option<String>,
    quote: Option<char>,
}

impl HtmlTag {
    fn render(self) -> String {
        if self.is_end {
            return format!("</{}>", self.name);
        }

        let mut output = String::new();
        output.push('<');
        output.push_str(&self.name);
        for attr in self.attrs {
            output.push(' ');
            output.push_str(&attr.name);
            if let Some(value) = attr.value {
                output.push('=');
                let quote = attr.quote.unwrap_or('"');
                output.push(quote);
                output.push_str(&value);
                output.push(quote);
            }
        }
        if self.is_self_closing {
            output.push_str(" /");
        }
        output.push('>');
        output
    }

    fn render_without_attrs(self) -> String {
        if self.is_end {
            format!("</{}>", self.name)
        } else if self.is_self_closing {
            format!("<{} />", self.name)
        } else {
            format!("<{}>", self.name)
        }
    }
}

fn transform_tags<F>(content: &str, mut transform: F) -> String
where
    F: FnMut(HtmlTag) -> String,
{
    let mut output = String::with_capacity(content.len());
    let mut index = 0;

    while let Some((start, end, raw_tag)) = next_tag(content, index) {
        output.push_str(&content[index..start]);
        if let Some(tag) = parse_tag(raw_tag) {
            output.push_str(&transform(tag));
        } else {
            output.push_str(raw_tag);
        }
        index = end;
    }

    output.push_str(&content[index..]);
    output
}

fn sanitize_tags(
    content: &str,
    allowed_tags: &HashSet<String>,
    allowed_attrs: &HashSet<String>,
) -> String {
    transform_tags(content, |mut tag| {
        if !allowed_tags.contains(tag.name.as_str()) {
            return String::new();
        }

        if tag.is_end {
            return tag.render();
        }

        tag.attrs.retain(|attr| {
            if attr.name.starts_with("on") || attr.name == "style" {
                return false;
            }
            if !allowed_attrs.contains(attr.name.as_str()) {
                return false;
            }
            if matches!(attr.name.as_str(), "href" | "src") {
                return attr.value.as_deref().is_none_or(|value| is_safe_url(value));
            }
            true
        });

        tag.render()
    })
}

fn remove_html_tag_internal(
    content: &str,
    with_tag_content: bool,
    tag_names: &HashSet<String>,
) -> String {
    if tag_names.is_empty() {
        return content.to_string();
    }

    let mut output = String::with_capacity(content.len());
    let mut index = 0;
    let mut skip_stack: Vec<String> = Vec::new();

    while let Some((start, end, raw_tag)) = next_tag(content, index) {
        if skip_stack.is_empty() {
            output.push_str(&content[index..start]);
        }

        let tag = parse_tag(raw_tag);
        let is_target = tag
            .as_ref()
            .is_some_and(|tag| tag_names.contains(tag.name.as_str()));

        if let Some(tag) = tag {
            if is_target {
                if with_tag_content {
                    if !tag.is_end && !tag.is_self_closing {
                        skip_stack.push(tag.name);
                    } else if tag.is_end {
                        pop_matching_tag(&mut skip_stack, &tag.name);
                    }
                }
            } else if with_tag_content {
                if tag.is_end {
                    pop_matching_tag(&mut skip_stack, &tag.name);
                } else if !skip_stack.is_empty() && !tag.is_self_closing {
                    skip_stack.push(tag.name);
                }

                if skip_stack.is_empty() {
                    output.push_str(raw_tag);
                }
            } else if skip_stack.is_empty() {
                output.push_str(raw_tag);
            }
        } else if skip_stack.is_empty() {
            output.push_str(raw_tag);
        }

        index = end;
    }

    if skip_stack.is_empty() {
        output.push_str(&content[index..]);
    }

    output
}

fn strip_tags(content: &str, drop_block_content: bool) -> String {
    if drop_block_content {
        let blocked = tag_set(&["script", "style"]);
        return SelfLike::remove_tag_content(content, &blocked);
    }

    let mut output = String::with_capacity(content.len());
    let mut index = 0;
    while let Some((start, end, _)) = next_tag(content, index) {
        output.push_str(&content[index..start]);
        index = end;
    }
    output.push_str(&content[index..]);
    output
}

struct SelfLike;

impl SelfLike {
    fn remove_tag_content(content: &str, blocked: &HashSet<String>) -> String {
        remove_html_tag_internal(content, true, blocked)
    }
}

fn remove_one_empty_tag_pass(content: &str) -> String {
    let mut output = String::with_capacity(content.len());
    let mut index = 0;

    while let Some((start, open_end, raw_open)) = next_tag(content, index) {
        output.push_str(&content[index..start]);

        let Some(open_tag) = parse_tag(raw_open) else {
            output.push_str(raw_open);
            index = open_end;
            continue;
        };

        if open_tag.is_end || open_tag.is_self_closing {
            output.push_str(raw_open);
            index = open_end;
            continue;
        }

        let body_start = open_end;
        let Some((close_start, close_end, raw_close)) = next_tag(content, body_start) else {
            output.push_str(raw_open);
            index = open_end;
            continue;
        };

        let body = &content[body_start..close_start];
        let close_tag = parse_tag(raw_close);
        if body.trim().is_empty()
            && close_tag
                .as_ref()
                .is_some_and(|tag| tag.is_end && tag.name == open_tag.name)
        {
            index = close_end;
        } else {
            output.push_str(raw_open);
            index = open_end;
        }
    }

    output.push_str(&content[index..]);
    output
}

fn next_tag(content: &str, from: usize) -> Option<(usize, usize, &str)> {
    let relative_start = content[from..].find('<')?;
    let start = from + relative_start;
    let mut quote = None;

    for (offset, ch) in content[start..].char_indices() {
        if offset == 0 {
            continue;
        }

        match (quote, ch) {
            (Some(q), c) if q == c => quote = None,
            (None, '\'' | '"') => quote = Some(ch),
            (None, '>') => {
                let end = start + offset + ch.len_utf8();
                return Some((start, end, &content[start..end]));
            }
            _ => {}
        }
    }

    None
}

fn parse_tag(raw: &str) -> Option<HtmlTag> {
    if !raw.starts_with('<') || !raw.ends_with('>') || raw.starts_with("<!--") {
        return None;
    }

    let mut inner = raw[1..raw.len() - 1].trim();
    if inner.is_empty() || inner.starts_with('!') || inner.starts_with('?') {
        return None;
    }

    let is_end = inner.starts_with('/');
    if is_end {
        inner = inner[1..].trim_start();
    }

    let is_self_closing = !is_end && inner.ends_with('/');
    if is_self_closing {
        inner = inner[..inner.len() - 1].trim_end();
    }

    let name_end = inner
        .find(|ch: char| ch.is_whitespace() || ch == '/')
        .unwrap_or(inner.len());
    let name = inner[..name_end].to_ascii_lowercase();
    if name.is_empty()
        || !name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-')
    {
        return None;
    }

    let attrs = if is_end {
        Vec::new()
    } else {
        parse_attrs(&inner[name_end..])
    };

    Some(HtmlTag {
        original: raw.to_string(),
        name,
        attrs,
        is_end,
        is_self_closing,
    })
}

fn parse_attrs(input: &str) -> Vec<HtmlAttr> {
    let bytes = input.as_bytes();
    let mut attrs = Vec::new();
    let mut index = 0;

    while index < bytes.len() {
        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        if index >= bytes.len() {
            break;
        }

        let name_start = index;
        while index < bytes.len()
            && !bytes[index].is_ascii_whitespace()
            && bytes[index] != b'='
            && bytes[index] != b'/'
        {
            index += 1;
        }

        if name_start == index {
            index += 1;
            continue;
        }

        let name = input[name_start..index].to_ascii_lowercase();

        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }

        let mut value = None;
        let mut quote = None;
        if index < bytes.len() && bytes[index] == b'=' {
            index += 1;
            while index < bytes.len() && bytes[index].is_ascii_whitespace() {
                index += 1;
            }

            if index < bytes.len() && matches!(bytes[index], b'\'' | b'"') {
                let quote_char = bytes[index] as char;
                quote = Some(quote_char);
                index += 1;
                let value_start = index;
                while index < bytes.len() && bytes[index] as char != quote_char {
                    index += 1;
                }
                value = Some(input[value_start..index].to_string());
                if index < bytes.len() {
                    index += 1;
                }
            } else {
                let value_start = index;
                while index < bytes.len() && !bytes[index].is_ascii_whitespace() {
                    index += 1;
                }
                value = Some(input[value_start..index].to_string());
            }
        }

        attrs.push(HtmlAttr { name, value, quote });
    }

    attrs
}

fn is_safe_url(value: &str) -> bool {
    let lower = value
        .trim()
        .chars()
        .filter(|ch| !ch.is_ascii_control() && !ch.is_whitespace())
        .collect::<String>()
        .to_ascii_lowercase();

    !(lower.starts_with("javascript:")
        || lower.starts_with("vbscript:")
        || lower.starts_with("data:"))
}

fn pop_matching_tag(stack: &mut Vec<String>, tag_name: &str) {
    if let Some(position) = stack.iter().rposition(|name| name == tag_name) {
        stack.truncate(position);
    }
}

fn tag_set(tags: &[&str]) -> HashSet<String> {
    tags.iter()
        .map(|tag| tag.trim().trim_start_matches('/').to_ascii_lowercase())
        .filter(|tag| !tag.is_empty())
        .collect()
}

fn attr_set(attrs: &[&str]) -> HashSet<String> {
    attrs
        .iter()
        .map(|attr| attr.trim().to_ascii_lowercase())
        .filter(|attr| !attr.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::HtmlUtil;

    #[test]
    fn clean_empty_tag_removes_nested_empty_tags() {
        assert_eq!(
            HtmlUtil::clean_empty_tag("<div><p></p><span> </span><b>x</b></div>"),
            "<div><b>x</b></div>"
        );
    }

    #[test]
    fn clean_html_tag_keeps_text() {
        let html = "pre<div class=\"test_div\">\r\n\t\ttext\r\n</div><div>BBBB</div>";
        assert_eq!(HtmlUtil::clean_html_tag(html), "pre\r\n\t\ttext\r\nBBBB");
    }

    #[test]
    fn escape_and_unescape_work() {
        let escaped = HtmlUtil::escape("'\"&<>");
        assert_eq!(escaped, "&#039;&quot;&amp;&lt;&gt;");
        assert_eq!(HtmlUtil::unescape(&escaped), "'\"&<>");
    }

    #[test]
    fn filter_removes_xss_vectors() {
        let html = "<script>alert(1)</script><p onclick=\"x()\">safe</p><a href=\"javascript:evil()\">bad</a><img src=\"https://example.com/a.png\">";
        assert_eq!(
            HtmlUtil::filter(html),
            "<p>safe</p><a>bad</a><img src=\"https://example.com/a.png\">"
        );
    }

    #[test]
    fn remove_all_attrs_from_named_tags() {
        let html = "<div class=\"a\" id=\"b\"><span class=\"c\">x</span></div>";
        assert_eq!(
            HtmlUtil::remove_all_html_attr(html, &["DIV"]),
            "<div><span class=\"c\">x</span></div>"
        );
    }

    #[test]
    fn remove_named_attrs_from_all_tags() {
        let html = "<div class=\"a\" id=\"b\"></div><span CLASS=\"c\"></span>";
        assert_eq!(
            HtmlUtil::remove_html_attr(html, &["class"]),
            "<div id=\"b\"></div><span></span>"
        );
    }

    #[test]
    fn remove_html_tag_can_drop_content() {
        let html = "pre<div>content</div><p>keep</p>";
        assert_eq!(HtmlUtil::remove_html_tag(html, &["div"]), "pre<p>keep</p>");
    }

    #[test]
    fn remove_html_tag_can_keep_content() {
        let html = "pre<div>content</div><p>keep</p>";
        assert_eq!(
            HtmlUtil::remove_html_tag_with_content(html, false, &["div"]),
            "precontent<p>keep</p>"
        );
        assert_eq!(
            HtmlUtil::unwrap_html_tag(html, &["div", "p"]),
            "precontentkeep"
        );
    }
}
