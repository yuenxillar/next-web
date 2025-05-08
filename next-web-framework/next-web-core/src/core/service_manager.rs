use std::fmt;

use hashbrown::HashMap;

// 引入 Service trait，用于约束泛型 T 必须实现 Service 接口
use super::service::Service;

/// Service manager 用于管理实现了 `Service` trait 的服务实例。
/// 
/// 它通过 `HashMap<String, T>` 来根据服务名称快速查找和管理服务。
pub struct ServiceManager<T>
where
    T: Service,
{
    services: HashMap<String, T>,
}

/// 自定义错误类型
#[derive(Debug)]
pub enum ServiceManagerError {
    ServiceAlreadyExists(String),
}

impl fmt::Display for ServiceManagerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ServiceManagerError::ServiceAlreadyExists(name) => {
                write!(f, "Service already exists: {}", name)
            }
        }
    }
}


impl<T> ServiceManager<T>
where
    T: Service,
{
    /// 创建一个新的空服务管理器。
    ///
    /// # 示例
    /// ```rust
    /// let manager = ServiceManager::<MyService>::new();
    /// assert!(manager.is_empty());
    /// ```
    pub fn new() -> Self {
        Self {
            services: HashMap::new(),
        }
    }

    /// 添加一个服务到管理器中。
    ///
    /// 服务的名称由其 `service_name()` 方法提供作为键。
    ///
    /// # 参数
    /// - `service`: 要添加的服务实例。
    ///
    pub fn add_service(&mut self, service: T) -> Result<(), ServiceManagerError> {
        let name = service.service_name();
        if self.services.contains_key(&name) {
            return Err(ServiceManagerError::ServiceAlreadyExists(name));
        }
        self.services.insert(name, service);
        Ok(())
    }

    /// 检查是否已经注册了指定名称的服务。
    ///
    /// # 参数
    /// - `name`: 要检查的服务名称。
    ///
    /// # 返回值
    /// 如果存在该名称的服务返回 `true`，否则返回 `false`。
    pub fn has_service(&self, name: &str) -> bool {
        self.services.contains_key(name)
    }

    /// 获取指定名称的服务的不可变引用。
    ///
    /// # 参数
    /// - `name`: 要获取的服务名称。
    ///
    /// # 返回值
    /// 如果存在该名称的服务则返回 `Some(&T)`，否则返回 `None`。
    pub fn get_service(&self, name: &str) -> Option<&T> {
        self.services.get(name)
    }

    /// 获取指定名称的服务的可变引用。
    ///
    /// # 参数
    /// - `name`: 要获取的服务名称。
    ///
    /// # 返回值
    /// 如果存在该名称的服务则返回 `Some(&mut T)`，否则返回 `None`。
    pub fn get_service_mut(&mut self, name: &str) -> Option<&mut T> {
        self.services.get_mut(name)
    }

    /// 获取当前管理器中注册的服务数量。
    ///
    /// # 返回值
    /// 返回服务数量（usize 类型）。
    pub fn len(&self) -> usize {
        self.services.len()
    }

    /// 获取所有服务的名称列表。
    ///
    /// # 返回值
    /// 返回一个包含所有服务名称的向量（Vec<String>）。
    pub fn service_names(&self) -> Vec<String> {
        self.services.keys().cloned().collect()
    }

    /// 判断服务管理器是否为空。
    ///
    /// # 返回值
    /// 如果没有注册任何服务则返回 `true`，否则返回 `false`。
    pub fn is_empty(&self) -> bool {
        self.services.is_empty()
    }
}