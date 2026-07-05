use std::sync::Arc;

use crate::anys::any_value::AnyValue;

pub trait HttpSession {
    /// 返回此 session 创建的时间，以毫秒表示
    /// 自 1970 年 1 月 1 日 GMT 以来的毫秒数
    fn creation_time(&self) -> u64;

    /// 返回分配给此 session 的唯一标识符
    fn id(&self) -> &str;

    /// 返回客户端最后一次与此 session 关联的请求时间
    fn get_last_accessed_time(&self) -> u64;

    /// 指定在 servlet 容器使此 session 失效之前
    /// 客户端请求之间的时间（以秒为单位）
    fn set_max_inactive_interval(&mut self, interval: u64);

    /// 返回 servlet 容器在客户端访问之间保持此 session 打开的最大时间间隔
    fn max_inactive_interval(&self) -> u64;

    /// 返回绑定到此 session 的指定名称的对象
    /// 如果没有绑定对象，则返回 None
    fn attribute(&self, name: &str) -> Option<&AnyValue>;

    /// 返回绑定到此 session 的所有对象名称的集合
    fn attribute_names(&self) -> Vec<&str>;

    /// 使用指定的名称将对象绑定到此 session
    fn set_attribute(&self, name: &str, value: AnyValue);

    /// 从此 session 中移除指定名称绑定的对象
    fn remove_attribute(&self, name: &str);

    /// 使此 session 无效，然后解绑绑定到它的任何对象
    fn invalidate(&mut self);

    /// 如果客户端还不知道该 session，或者客户端选择不加入该 session，则返回 true
    fn is_new(&self) -> bool;

    /// 获取 Accessor（默认实现返回 None）
    fn accessor(&self) -> Option<Arc<dyn HttpSessionAccessor>> {
        None
    }
}

pub trait HttpSessionAccessor {
    fn access(&self, var1: &dyn FnMut(&mut dyn HttpSession));
}
