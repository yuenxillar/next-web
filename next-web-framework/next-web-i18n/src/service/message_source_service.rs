use std::{collections::HashMap, str::FromStr};

use crate::{properties::messages_properties::MessagesProperties, util::locale::Locale};

#[derive(Clone)]
pub struct MessageSourceService {
    properties: MessagesProperties,
    source: HashMap<Locale, HashMap<Box<str>, Message>>,
}

#[derive(Clone, Hash, Eq, PartialEq)]
struct Message {
    indexs: Option<Vec<usize>>,
    content: String,
}

impl MessageSourceService {
    pub fn new(properties: MessagesProperties) -> Self {
        Self {
            properties,
            source: HashMap::new(),
        }
    }

    pub fn message(&self, code: impl AsRef<str>, locale: Locale) -> Option<String> {
        self.source
            .get(&locale)
            .map(|s| s.get(code.as_ref()).map(|s| s.content.clone()))
            .unwrap_or_default()
    }

    pub fn message_with_args(
        &self,
        code: impl AsRef<str>,
        args: &[&str],
        locale: Locale,
    ) -> Option<String> {
        // TODO ARGS
        self.source
            .get(&locale)
            .map(|s| {
                s.get(code.as_ref())
                    .map(|s| self.fill_args_from_index(s, args))
            })
            .unwrap_or_default()
    }

    pub fn message_or_default(&self, code: impl AsRef<str>, locale: Locale) -> Option<String> {
        self.message(code.as_ref(), locale).or_else(|| {
            self.message(
                code,
                self.properties
                    .local()
                    .map(|s| Locale::from_str(s).unwrap_or(Locale::locale()))
                    .unwrap_or(Locale::locale()),
            )
        })
    }

    pub fn add_message_source(&mut self, locale: Locale, str: impl AsRef<str>) {
        let source = str.as_ref();
        if let Ok(messages) = Self::analysis(source) {
            self.source.insert(locale, messages);
        }
    }

    fn analysis(str: &str) -> Result<HashMap<Box<str>, Message>, &'static str> {
        if str.is_empty() {
            return Err("MessageSourceService analysis error: empty string");
        }

        let mut messages = HashMap::new();

        // properties file format
        // key=value
        // example:
        // hello=你好
        // world=世界
        str.lines().for_each(|s| {
            if let Some(index) = s.find('=') {
                let (key, value) = s.split_at(index);
                let key = key.trim().into();
                let content = value.to_string();

                let indexs: Option<Vec<usize>> = if content.contains("%s") {
                    let index = content.match_indices("%s").map(|(i, _)| i).collect::<Vec<_>>();
                    if index.is_empty() { None } else { Some(index) }
                } else {
                    None
                };

                let message = Message { indexs, content };
                messages.insert(key, message);
            }
        });

        Ok(messages)
    }

    // I am you %s, you are %s!
    fn fill_args_from_index(&self, message: &Message, args: &[&str]) -> String {
        let mut text = message.content.clone();

        if let Some(indexes) = &message.indexs {
            for (arg_idx, &arg) in args.iter().enumerate() {
                if let Some(&placeholder_pos) = indexes.get(arg_idx) {
                    let start = placeholder_pos;
                    let end = placeholder_pos + 2;
                    if end <= text.len() {
                        text.replace_range(start..end, arg);
                    }
                }
            }
        }

        text
    }

    pub fn fill_args(message: &str, args: &[&str]) -> String {
        // i am %s, you are %s!
        let mut result =
            String::with_capacity(message.len() + args.iter().map(|s| s.len()).sum::<usize>());
        let mut arg_index = 0;

        let mut start = 0;
        let bytes = message.as_bytes();
        let len = bytes.len();

        while start < len {
            // 查找下一个 '%' 的位置
            if let Some(percent_pos) = &bytes[start..].iter().position(|&b| b == b'%') {
                let actual_pos = start + percent_pos;
                result.push_str(&message[start..actual_pos]); // 复制 '%' 之前的内容

                // 检查是否是 "%s"
                if actual_pos + 1 < len && bytes[actual_pos + 1] == b's' {
                    if arg_index < args.len() {
                        result.push_str(args[arg_index]);
                        arg_index += 1;
                        start = actual_pos + 2; // 跳过 "%s"
                    } else {
                        return message.into();
                    }
                } else {
                    // 不是 "%s"，比如 "%%" 或其他，原样输出 '%' 并移动一位
                    result.push('%');
                    start = actual_pos + 1;
                }
            } else {
                // 没有更多 '%' 了，复制剩余部分
                result.push_str(&message[start..]);
                break;
            }
        }

        if arg_index < args.len() {
            return message.into();
        }

        result
    }
}
