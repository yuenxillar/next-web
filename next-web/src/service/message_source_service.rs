use std::sync::Arc;
use std::{collections::HashMap, str::FromStr};

use next_web_core::autoconfigure::context::messages_properties::MessagesProperties;
use next_web_core::constants::application_constants::I18N;
use next_web_core::constants::common_constants::{MESSAGES, PROPERTIES};
use next_web_core::context::application_resources::{ApplicationResources, ResourceLoader};
use next_web_core::util::locale::Locale;

#[derive(Clone, Debug)]
pub struct MessageSourceService {
    properties: MessagesProperties,
    source: Arc<HashMap<Locale, HashMap<Box<str>, Message>>>,
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
struct Message {
    indexs: Option<Vec<usize>>,
    content: String,
}

impl MessageSourceService {
    pub fn from_resouces(properties: MessagesProperties, resources: &ApplicationResources) -> Self {
        let locales = Self::build(&properties, resources);

        // println!("messages: {:?}", locales);
        Self {
            properties,
            source: Arc::new(locales),
        }
    }

    fn build(
        properties: &MessagesProperties,
        resources: &ApplicationResources,
    ) -> HashMap<Locale, HashMap<Box<str>, Message>> {
        let mut map: HashMap<Locale, HashMap<Box<str>, Message>> = HashMap::new();

        let base_name = properties.base_name().unwrap_or(MESSAGES);

        let iters = resources.load_dir(I18N);
        iters
            .into_iter()
            .filter(|s| s.starts_with(& format!("{}/{}", I18N, base_name)) && s.ends_with(PROPERTIES))
            .for_each(|path| {
                // default
                let locale: Option<Locale> =
                    if path.eq(&format!("{}/{}.{}", I18N, base_name, PROPERTIES)) {
                        Some(Locale::locale())
                    } else {
                        let mut s1 = path
                            .replace(& format!("{}/{}", I18N, base_name), "")
                            .replace(PROPERTIES, "")
                            .replace(".", "");
                        s1.remove(0);
                        Locale::from_str(s1.as_str()).ok()
                    };

                // println!("path: {}", path);
                // println!("locale: {:?}", locale);

                locale.map(|val| {
                    resources.load(path).map(|data| {
                        if let Ok(messages) = String::from_utf8(data.to_vec()) {
                            let source = messages.as_ref();
                            if let Ok(messages) = Self::analysis(source) {
                                if map.contains_key(&val) {
                                    // merge messages
                                    if let Some(m) = map.get_mut(&val) {
                                        m.extend(messages);
                                    }
                                } else {
                                    map.insert(val, messages);
                                }
                            }
                        }
                    });
                });
            });

        map
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
        self.source
            .get(&locale)
            .map(|s| {
                s.get(code.as_ref())
                    .map(|s| self.fill_args_from_index(s, args))
            })
            .unwrap_or_default()
    }

    pub fn message_or_default(&self, code: impl AsRef<str>, locale: Locale) -> String {
        let code = code.as_ref();
        self.message(code, locale).or_else(|| {
            self.message(
                code,
                self.properties
                    .local()
                    .map(|s| Locale::from_str(s).unwrap_or(Locale::locale()))
                    .unwrap_or(Locale::locale()),
            )
        })
        .unwrap_or(code.into())
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
                let content = value[1..].to_string();

                let indexs: Option<Vec<usize>> = if content.contains("%s") {
                    let index = content
                        .match_indices("%s")
                        .map(|(i, _)| i)
                        .collect::<Vec<_>>();
                    if index.is_empty() {
                        None
                    } else {
                        Some(index)
                    }
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
            for index in (0..args.len()).rev() {
                 if let Some(&placeholder_pos) = indexes.get(index) {
                    let start = placeholder_pos;
                    let end = placeholder_pos + 2;
                    if end <= text.len() {
                        text.replace_range(start..end, args[index]);
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
