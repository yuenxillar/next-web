use std::{cell::UnsafeCell, fmt};

/// URL 路由匹配器
#[derive(Debug)]
pub struct AnyMatcher<T> {
    root: Node<T>,
}

impl<T> Default for AnyMatcher<T> {
    fn default() -> Self {
        Self {
            root: Node::new("/", NodeType::Root),
        }
    }
}

impl<T> AnyMatcher<T> {
    /// 创建新的路由匹配器
    pub fn new() -> Self {
        Self::default()
    }

    /// 添加路由规则
    pub fn insert(&mut self, pattern: impl Into<String>, value: T) -> Result<(), RouteError> {
        let pattern = pattern.into();
        self.root.insert(&pattern, value)
    }

    /// 匹配路径（不可变引用）
    pub fn at(&self, path: &str) -> Result<&T, MatchError> {
        self.root.find(path).map(|cell| unsafe { &*cell.get() })
    }

    /// 匹配路径（可变引用）
    pub fn at_mut(&mut self, path: &str) -> Result<&mut T, MatchError> {
        self.root.find_mut(path).map(|cell| unsafe { &mut *cell.get() })
    }
}

/// 路由节点
#[derive(Debug)]
struct Node<T> {
    prefix: String,
    node_type: NodeType,
    priority: u32,
    children: Vec<Node<T>>,
    value: Option<UnsafeCell<T>>,
}

impl<T> Node<T> {
    fn new(prefix: &str, node_type: NodeType) -> Self {
        Self {
            prefix: prefix.to_string(),
            node_type,
            priority: 0,
            children: Vec::new(),
            value: None,
        }
    }

    fn determine_node_type(segment: &str) -> NodeType {
        match segment {
            "**" => NodeType::CatchAll,
            "*" => NodeType::CatchSingle,
            s if s.starts_with("*.") => NodeType::CatchExtension(s[2..].to_string()),
            s if s.ends_with(".*") => NodeType::PrefixExtension(s[..s.len()-2].to_string()),
            _ => NodeType::Static,
        }
    }

    /// 插入路由
    fn insert(&mut self, pattern: &str, value: T) -> Result<(), RouteError> {
        if !pattern.starts_with('/') {
            return Err(RouteError::InvalidFormat);
        }

        let segments: Vec<&str> = pattern.split('/').filter(|s| !s.is_empty()).collect();
        if segments.is_empty() {
            return Err(RouteError::InvalidFormat);
        }

        self.insert_segments(&segments, value)
    }

    fn insert_segments(&mut self, segments: &[&str], value: T) -> Result<(), RouteError> {
        if segments.is_empty() {
            if self.value.is_some() {
                return Err(RouteError::Conflict {
                    with: self.prefix.clone(),
                });
            }
            self.value = Some(UnsafeCell::new(value));
            return Ok(());
        }

        let current_segment = segments[0];
        let node_type = Self::determine_node_type(current_segment);
        let remaining_segments = &segments[1..];

        // 查找匹配的子节点
        for child in &mut self.children {
            if child.prefix == current_segment && child.node_type == node_type {
                return child.insert_segments(remaining_segments, value);
            }
        }

        // 没有找到匹配的子节点，创建新节点
        let mut new_node = Node::new(current_segment, node_type);
        new_node.insert_segments(remaining_segments, value)?;
        self.children.push(new_node);
        self.priority += 1;
        
        // 保持子节点按优先级排序
        self.children.sort_by(|a, b| b.priority.cmp(&a.priority));
        
        Ok(())
    }

    /// 查找匹配的路由（不可变）
    fn find(&self, path: &str) -> Result<&UnsafeCell<T>, MatchError> {
        let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        self.find_internal(&segments)
    }

    /// 查找匹配的路由（可变）
    fn find_mut(&mut self, path: &str) -> Result<&mut UnsafeCell<T>, MatchError> {
        let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        self.find_mut_internal(&segments)
    }

    fn find_internal(&self, segments: &[&str]) -> Result<&UnsafeCell<T>, MatchError> {
        if segments.is_empty() {
            return self.value.as_ref().ok_or(MatchError::NotFound);
        }

        let current_segment = segments[0];
        let remaining_segments = &segments[1..];

        // 检查所有可能的子节点
        for child in &self.children {
            match &child.node_type {
                NodeType::Static if child.prefix == current_segment => {
                    return child.find_internal(remaining_segments);
                }
                NodeType::CatchSingle => {
                    return child.find_internal(remaining_segments);
                }
                NodeType::CatchExtension(ext) if current_segment.ends_with(ext) => {
                    return child.find_internal(remaining_segments);
                }
                NodeType::PrefixExtension(prefix) if current_segment.starts_with(prefix) => {
                    return child.find_internal(remaining_segments);
                }
                NodeType::CatchAll => {
                    return child.value.as_ref().ok_or(MatchError::NotFound);
                }
                _ => {}
            }
        }

        Err(MatchError::NotFound)
    }

    fn find_mut_internal(&mut self, segments: &[&str]) -> Result<&mut UnsafeCell<T>, MatchError> {
        if segments.is_empty() {
            return self.value.as_mut().ok_or(MatchError::NotFound);
        }

        let current_segment = segments[0];
        let remaining_segments = &segments[1..];

        for child in &mut self.children {
            match &child.node_type {
                NodeType::Static if child.prefix == current_segment => {
                    return child.find_mut_internal(remaining_segments);
                }
                NodeType::CatchSingle => {
                    return child.find_mut_internal(remaining_segments);
                }
                NodeType::CatchExtension(ext) if current_segment.ends_with(ext) => {
                    return child.find_mut_internal(remaining_segments);
                }
                NodeType::PrefixExtension(prefix) if current_segment.starts_with(prefix) => {
                    return child.find_mut_internal(remaining_segments);
                }
                NodeType::CatchAll => {
                    return child.value.as_mut().ok_or(MatchError::NotFound);
                }
                _ => {}
            }
        }

        Err(MatchError::NotFound)
    }
}

/// 节点类型
#[derive(Debug, Clone, PartialEq)]
enum NodeType {
    Root,
    Static,
    CatchSingle,
    CatchExtension(String),      // 例如 *.ext
    PrefixExtension(String),     // 例如 prefix.*
    CatchAll,
}

/// 路由错误
#[derive(Debug, Clone)]
pub enum RouteError {
    Conflict { with: String },
    InvalidFormat,
}

impl fmt::Display for RouteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Conflict { with } => write!(f, "Route conflicts with existing route: {}", with),
            Self::InvalidFormat => write!(f, "Route must start with '/'"),
        }
    }
}

impl std::error::Error for RouteError {}

/// 匹配错误
#[derive(Debug, Clone)]
pub enum MatchError {
    NotFound,
}

impl fmt::Display for MatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => write!(f, "No matching route found"),
        }
    }
}

impl std::error::Error for MatchError {}

// 实现 Clone 需要确保 UnsafeCell 的安全使用
impl<T> Clone for Node<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Self {
            prefix: self.prefix.clone(),
            node_type: self.node_type.clone(),
            priority: self.priority,
            children: self.children.clone(),
            value: self.value.as_ref().map(|v| UnsafeCell::new(unsafe { &*v.get() }.clone())),
        }
    }
}

// 安全实现，因为 UnsafeCell 的使用是受控的
unsafe impl<T: Send> Send for Node<T> {}
unsafe impl<T: Sync> Sync for Node<T> {}