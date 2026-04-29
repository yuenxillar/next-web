use std::collections::BTreeMap;

pub struct PageUtil;

impl PageUtil {
    /// 构建一个自动提交的 HTML 表单
    pub fn build_form(action_url: impl AsRef<str>, parameters: &BTreeMap<&'static str, String>) -> String {
        format!(
            r#"<form name="punchout_form" method="post" action="{}">
{}
<input type="submit" value="立即支付" style="display:none">
</form>
<script>document.forms[0].submit();</script>"#,
            action_url.as_ref(),
            Self::build_hidden_fields(parameters)
        )
    }

    /// 构建所有隐藏字段的 HTML
    fn build_hidden_fields(parameters: &BTreeMap<&'static str, String>) -> String {
        if parameters.is_empty() {
            return String::new();
        }

        let mut builder = String::new();
        for (key, value) in parameters {
            if !key.is_empty() && !value.is_empty() {
                builder.push_str(&Self::build_hidden_field(key, value));
            }
        }
        builder
    }

    /// 构建单个隐藏字段的 HTML
    fn build_hidden_field(key: &str, value: &str) -> String {
        // 将双引号转义为 &quot;
        let escaped_value = value.replace('"', "&quot;");
        format!(
            r#"<input type="hidden" name="{}" value="{}">
"#,
            key, escaped_value
        )
    }
}

// 使用示例
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_form() {
        let mut params = BTreeMap::new();
        params.insert("order_id", "12345".to_string());
        params.insert("amount", "100.00".to_string());
        params.insert("desc", "测试商品\"特别版\"".to_string());

        let form_html = PageUtil::build_form("https://example.com/pay", &params);

        println!("生成的表单 HTML:\n{}", form_html);

        // 验证包含必要元素
        assert!(form_html.contains(r#"action="https://example.com/pay""#));
        assert!(form_html.contains(r#"name="order_id""#));
        assert!(form_html.contains(r#"value="12345""#));
        assert!(form_html.contains(r#"&quot;特别版&quot;"#)); // 验证引号转义
        assert!(form_html.contains("document.forms[0].submit()"));
    }

    #[test]
    fn test_empty_parameters() {
        let params = BTreeMap::new();
        let form_html = PageUtil::build_form("https://example.com/pay", &params);

        // 没有隐藏字段，但仍然有表单结构
        assert!(form_html.contains("<form"));
        assert!(!form_html.contains(r#"<input type="hidden""#));
    }
}
