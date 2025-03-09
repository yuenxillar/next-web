use serde::de::DeserializeOwned;
use serde_json::Value;
use std::collections::HashMap;

use crate::error::json_object_error::JsonObjectError;

/// 定义一个可序列化和反序列化的 JSON 对象结构体。
/// 该结构体封装了一个 `HashMap`，用于存储键值对，并支持多种操作接口。
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct JsonObject {
    /// 存储 JSON 键值对的核心数据结构。
    raw_value: HashMap<String, Value>,
    /// 是否保持键值对的顺序（暂时不支持）。
    is_order: bool,
}

impl JsonObject {
    /// 创建一个新的 `JsonObject` 实例。
    ///
    /// # 参数
    /// - `is_order`: 是否保持键值对的顺序（当前未实现）。
    ///
    /// # 返回值
    /// 返回一个初始化的 `JsonObject` 实例，默认容量为 12 <button class="citation-flag" data-index="1">。
    pub fn new(is_order: bool) -> Self {
        Self {
            raw_value: HashMap::with_capacity(12),
            is_order,
        }
    }

    /// 从 JSON 字符串解析出指定类型的对象实例。
    ///
    /// # 参数
    /// - `json_str`: 包含 JSON 数据的字符串。
    ///
    /// # 返回值
    /// 如果解析成功，返回指定类型的对象；否则返回 `JsonObjectError::PaseError`。
    pub fn parse_object<T: DeserializeOwned>(json_str: &str) -> Result<T, JsonObjectError> {
        serde_json::from_str::<T>(json_str).map_err(|_| JsonObjectError::PaseError)
    }

    /// 检查键是否为空。
    ///
    /// # 参数
    /// - `key`: 需要检查的键。
    ///
    /// # 返回值
    /// 如果键为空，返回 `false`；否则返回 `true`。
    fn check_key(&self, key: &str) -> bool {
        if key.is_empty() {
            return false;
        }
        true
    }
}

///  为 `JsonObject` 提供功能接口。
impl JsonObject {
    /// 获取 `JsonObject` 中键值对的数量。
    ///
    /// # 返回值
    /// 返回键值对的总数。
    fn size(&self) -> usize {
        self.raw_value.len()
    }

    /// 检查 `JsonObject` 是否为空。
    ///
    /// # 返回值
    /// 如果没有任何键值对，返回 `true`；否则返回 `false`。
    fn is_empty(&self) -> bool {
        self.raw_value.is_empty()
    }

    /// 检查 `JsonObject` 是否包含指定键。
    ///
    /// # 参数
    /// - `key`: 需要检查的键。
    ///
    /// # 返回值
    /// 如果包含指定键，返回 `true`；否则返回 `false`。
    fn contains_key(&self, key: &str) -> bool {
        self.raw_value.contains_key(key)
    }

    /// 根据键获取值并反序列化为指定类型。
    ///
    /// # 参数
    /// - `key`: 需要查找的键。
    ///
    /// # 返回值
    /// 如果键存在且反序列化成功，返回对应的值；否则返回 `None` <button class="citation-flag" data-index="1">。
    fn get<V: DeserializeOwned>(&self, key: &str) -> Option<V> {
        self.raw_value
            .get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    /// 根据键获取值并反序列化为指定类型，如果键不存在则返回默认值。
    ///
    /// # 参数
    /// - `key`: 需要查找的键。
    /// - `default`: 默认值。
    ///
    /// # 返回值
    /// 如果键存在且反序列化成功，返回对应的值；否则返回默认值。
    fn get_or_default<V: DeserializeOwned>(&self, key: &str, default: V) -> V {
        if let Some(v) = self.raw_value.get(key) {
            return serde_json::from_value(v.clone()).unwrap_or(default);
        }
        default
    }

    /// 根据键获取浮点数值。
    ///
    /// # 参数
    /// - `key`: 需要查找的键。
    ///
    /// # 返回值
    /// 如果键存在且值为浮点数，返回对应的值；否则返回 `None`。
    fn get_double(&self, key: &str) -> Option<f64> {
        self.raw_value.get(key).and_then(|v| v.as_f64())
    }

    /// 根据键获取整数值。
    ///
    /// # 参数
    /// - `key`: 需要查找的键。
    ///
    /// # 返回值
    /// 如果键存在且值为整数，返回对应的值；否则返回 `None`。
    fn get_int(&self, key: &str) -> Option<i64> {
        self.raw_value.get(key).and_then(|v| v.as_i64())
    }

    /// 根据键获取无符号整数值。
    ///
    /// # 参数
    /// - `key`: 需要查找的键。
    ///
    /// # 返回值
    /// 如果键存在且值为无符号整数，返回对应的值；否则返回 `None`。
    fn get_uint(&self, key: &str) -> Option<u64> {
        self.raw_value.get(key).and_then(|v| v.as_u64())
    }

    /// 根据键获取字符串值。
    ///
    /// # 参数
    /// - `key`: 需要查找的键。
    ///
    /// # 返回值
    /// 如果键存在且值为字符串，返回对应的值；否则返回 `None`。
    fn get_str(&self, key: &str) -> Option<&str> {
        self.raw_value.get(key).and_then(|v| v.as_str())
    }

    /// 根据键获取布尔值。
    ///
    /// # 参数
    /// - `key`: 需要查找的键。
    ///
    /// # 返回值
    /// 如果键存在且值为布尔值，返回对应的值；否则返回 `None`。
    fn get_bool(&self, key: &str) -> Option<bool> {
        self.raw_value.get(key).and_then(|v| v.as_bool())
    }

    /// 根据键获取 JSON 数组，并反序列化为指定类型。
    ///
    /// # 参数
    /// - `key`: 需要查找的键。
    ///
    /// # 返回值
    /// 如果键存在且值为数组，返回反序列化后的列表；否则返回空列表。
    fn get_json_array<V: DeserializeOwned>(&self, key: &str) -> Vec<V> {
        self.raw_value
            .get(key)
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .map(|v| serde_json::from_value(v.clone()).unwrap())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// 设置指定键的值，如果键已存在则覆盖原有值。
    ///
    /// # 参数
    /// - `key`: 需要设置的键。
    /// - `value`: 需要设置的值。
    ///
    /// # 返回值
    /// 如果键已存在，返回被覆盖的旧值；否则返回 `None`。
    fn set<V: Into<Value>>(&mut self, key: impl Into<String>, value: V) -> Option<Value> {
        self.raw_value.insert(key.into(), value.into())
    }

    /// 设置指定键的值，如果键已存在则返回错误。
    ///
    /// # 参数
    /// - `key`: 需要设置的键。
    /// - `value`: 需要设置的值。
    ///
    /// # 返回值
    /// 如果键不存在且设置成功，返回 `Ok(())`；否则返回相应的错误。
    fn put_once<V: Into<Value>>(
        &mut self,
        key: impl Into<String>,
        value: V,
    ) -> Result<(), JsonObjectError> {
        let key = key.into();
        if !self.check_key(&key) {
            return Err(JsonObjectError::KeyIsEmpty);
        }
        if self.raw_value.contains_key(&key) {
            return Err(JsonObjectError::KeyAlreadyExists);
        }
        self.set(key, value);
        Ok(())
    }

    /// 设置指定键的值，如果值为 `null` 则返回错误。
    ///
    /// # 参数
    /// - `key`: 需要设置的键。
    /// - `value`: 需要设置的值。
    ///
    /// # 返回值
    /// 如果值不为 `null` 且设置成功，返回 `Ok(())`；否则返回相应的错误。
    fn put_opt<V: Into<Value>>(
        &mut self,
        key: impl Into<String>,
        value: V,
    ) -> Result<(), JsonObjectError> {
        let key = key.into();
        let value = value.into();
        if !self.check_key(&key) {
            return Err(JsonObjectError::KeyIsEmpty);
        }

        if value.is_null() {
            return Err(JsonObjectError::KeyOrValueIsNull);
        }

        self.set(key, value);
        Ok(())
    }

    /// 将另一个可迭代的键值对集合中的所有键值对添加到 `JsonObject` 中。
    ///
    /// # 参数
    /// - `data`: 包含键值对的可迭代集合。
    fn put_all(&mut self, data: impl IntoIterator<Item = (String, Value)>) {
        self.raw_value.extend(data);
    }

    /// 获取 `JsonObject` 的原始 `HashMap` 引用。
    ///
    /// # 返回值
    /// 返回底层 `HashMap` 的不可变引用。
    fn raw_value(&self) -> &HashMap<String, Value> {
        &self.raw_value
    }

    /// 清空 `JsonObject` 中的所有键值对。
    fn clear(&mut self) {
        self.raw_value.clear()
    }

    /// 获取 `JsonObject` 中所有值的引用列表。
    ///
    /// # 返回值
    /// 返回所有值的引用列表。
    fn values(&self) -> Vec<&Value> {
        self.raw_value.values().collect()
    }

    /// 获取 `JsonObject` 中所有键的引用列表。
    ///
    /// # 返回值
    /// 返回所有键的引用列表。
    fn keys(&self) -> Vec<&String> {
        self.raw_value.keys().collect()
    }

    /// 将 `JsonObject` 转换为 JSON 字符串。
    ///
    /// # 返回值
    /// 返回紧凑格式的 JSON 字符串。
    fn to_json_string(&self) -> String {
        serde_json::to_string(&self.raw_value).unwrap_or_default()
    }

    /// 将 `JsonObject` 转换为格式化的 JSON 字符串。
    ///
    /// # 返回值
    /// 返回格式化的 JSON 字符串。
    fn to_json_string_pretty(&self) -> String {
        serde_json::to_string_pretty(&self.raw_value).unwrap_or_default()
    }
}

/// 实现 `Default` 特性以提供默认的 `JsonObject` 实例。
impl Default for JsonObject {
    /// 创建一个默认的 `JsonObject` 实例。
    ///
    /// # 返回值
    /// 返回一个初始化的 `JsonObject` 实例，默认容量为 12，`is_order` 为 `false`。
    fn default() -> Self {
        Self {
            raw_value: HashMap::with_capacity(12),
            is_order: false,
        }
    }
}

