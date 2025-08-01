use std::{cell::UnsafeCell, fmt, ops::Range};

/// URL 路由匹配器
#[derive(Debug, Clone)]
pub struct RouteMatcher<T> {
    root: Node<T>,
}

impl<T> Default for RouteMatcher<T> {
    fn default() -> Self {
        Self {
            root: Node::default(),
        }
    }
}

impl<T> RouteMatcher<T> {
    /// 创建新的路由匹配器
    pub fn new() -> Self {
        Self::default()
    }

    /// 添加路由规则
    pub fn insert(&mut self, pattern: impl Into<String>, value: T) -> Result<(), RouteError> {
        let pattern = pattern.into();
        self.root.insert(pattern, value)
    }

    /// 匹配路径（不可变引用）
    pub fn at(&self, path: &str) -> Result<&T, MatchError> {
        self.root
            .find(path.as_bytes())
            .map(|cell| unsafe { &*cell.get() })
    }

    /// 匹配路径（可变引用）
    pub fn at_mut(&mut self, path: &str) -> Result<&mut T, MatchError> {
        self.root
            .find(path.as_bytes())
            .map(|cell| unsafe { &mut *cell.get() })
    }
}

/// 路由节点
#[derive(Debug)]
struct Node<T> {
    prefix: String,
    node_type: NodeType,
    priority: u32,
    wild_child: bool,
    children: Vec<Node<T>>,
    value: Option<UnsafeCell<T>>,
}

impl<T> Default for Node<T> {
    fn default() -> Self {
        Self {
            prefix: "/".to_string(),
            node_type: NodeType::Root,
            priority: 0,
            wild_child: true,
            children: Vec::new(),
            value: None,
        }
    }
}

impl<T> Node<T> {
    fn get_route_type(path: &str) -> NodeType {
        if path.is_empty() || !path.contains("*") {
            return NodeType::Static;
        }

        if path.eq("**") {
            return NodeType::CatchAll;
        }

        if path.eq("*") {
            return NodeType::CatchSingle;
        }

        //    let element = path.split("*").filter(|s| !s.is_empty()).collect::<Vec<_>>();

        //     if element.len() > 1 {
        //         return NodeType::CatchSingleExtension;
        //     }

        NodeType::CatchSingleExtension
    }
    /// 插入路由
    fn insert(&mut self, pattern: String, value: T) -> Result<(), RouteError> {
        if !pattern.starts_with('/') {
            return Err(RouteError::InvalidFormat);
        }

        if pattern == "/**" {
            return Err(RouteError::InvalidFormat);
        }

        self.priority += 1;
        let mut remaining = pattern;
        let mut current = self;

        let mut routes = remaining.split('/').filter(|s| !s.is_empty()).peekable();

        'weak: loop {
            if let Some(prefix) = routes.next() {
                let node_type = Self::get_route_type(prefix);
                let priority: u32 = node_type.clone().into();
                let prefix = prefix.to_string();

                let has_children = current.children.is_empty();
                if has_children {
                    current.children.push(Node {
                        prefix,
                        node_type,
                        priority,
                        wild_child: false,
                        children: Vec::new(),
                        value: if routes.peek().is_none() {
                            Some(UnsafeCell::new(value))
                        } else {
                            None
                        },
                    });
                    current.wild_child = true;
                    break;
                } else {
                    // 在循环外部定义一个变量来存储匹配的索引
                    let mut found_index = None;

                    // 使用不可变借用或索引进行查找
                    for (index, child) in current.children.iter().enumerate() {
                        if child.prefix.eq(&prefix) {
                            found_index = Some(index);
                            break; // 找到就跳出
                        }
                    }

                    // 在循环外部，借用已经结束，可以安全地进行可变借用
                    if let Some(index) = found_index {
                        current = &mut current.children[index];
                        continue 'weak;
                    } else {
                        current.children.push(Node {
                            prefix,
                            node_type,
                            priority,
                            wild_child: false,
                            children: Vec::new(),
                            value: if routes.peek().is_none() {
                            Some(UnsafeCell::new(value))
                        } else {
                            None
                        },
                        });
                        current.wild_child = true;
                        break;
                    }
                }
            }
        }

        // 'insert: loop {
        //     break;

        // // 查找共同前缀
        // let common_len = current
        //     .prefix
        //     .chars()
        //     .zip(remaining.chars())
        //     .take_while(|(a, b)| a == b)
        //     .count();

        // // 如果当前节点前缀比共同前缀长，需要分割节点
        // if current.prefix.len() > common_len {
        //     let suffix = current.prefix.split_off(common_len);
        //     let child = Node {
        //         prefix: suffix,
        //         value: current.value.take(),
        //         children: std::mem::take(&mut current.children),
        //         priority: current.priority - 1,
        //         wild_child: current.wild_child,
        //         node_type: current.node_type.clone(),
        //     };

        //     current.children.push(child);
        //     current.node_type = NodeType::Static;
        //     current.wild_child = false;
        // }

        // // 如果剩余部分完全匹配
        // if remaining.len() == common_len {
        //     if current.value.is_some() {
        //         return Err(RouteError::Conflict {
        //             with: remaining.clone(),
        //         });
        //     }
        //     current.value = Some(UnsafeCell::new(value));
        //     return Ok(());
        // }

        // // 处理剩余部分
        // remaining = remaining[common_len..].to_string();
        // let next_char = remaining.chars().next().unwrap();

        // // 检查通配符
        // if let Some(wildcard) = Self::extract_wildcard(&remaining) {
        //     // 处理通配符节点
        //     if current.wild_child {
        //         // 已有通配符子节点，检查是否冲突
        //         let last_child = current.children.last_mut().unwrap();
        //         // if last_child.prefix != wildcard.pattern {
        //         //     return Err(RouteError::Conflict {
        //         //         with: remaining.clone(),
        //         //     });
        //         // }

        //         if last_child.node_type == NodeType::CatchAll {
        //             return Err(RouteError::InvalidCatchAll);
        //         }

        //         last_child.priority += 1;
        //         current = last_child;
        //         remaining = remaining[wildcard.range.len()..].to_string();
        //         continue 'insert;
        //     }

        //     // 创建新的通配符节点
        //     let mut wildcard_node = Node {
        //         prefix: wildcard.pattern.clone(),
        //         node_type: wildcard.node_type,
        //         priority: 1,
        //         wild_child: false,
        //         children: Vec::new(),
        //         value: None,
        //     };

        //     // 如果通配符后还有路径，需要继续处理
        //     if wildcard.range.end < remaining.len() {
        //         let suffix = remaining[wildcard.range.end..].to_string();
        //         wildcard_node.insert(suffix, value)?;
        //     } else {
        //         wildcard_node.value = Some(UnsafeCell::new(value));
        //     }

        //     current.children.push(wildcard_node);
        //     current.wild_child = true;
        //     current.update_child_priority(current.children.len() - 1);
        //     return Ok(());
        // }

        // // 处理静态子节点
        // if let Some(child_idx) = current
        //     .children
        //     .iter()
        //     .position(|c| c.prefix.starts_with(next_char))
        // {
        //     current.update_child_priority(child_idx);
        //     current = &mut current.children[child_idx];
        //     continue 'insert;
        // }

        // // 添加新的静态子节点
        // let static_node = Node {
        //     prefix: remaining.clone(),
        //     node_type: NodeType::Static,
        //     priority: 1,
        //     wild_child: false,
        //     children: Vec::new(),
        //     value: Some(UnsafeCell::new(value)),
        // };

        // current.children.push(static_node);
        // current.update_child_priority(current.children.len() - 1);
        return Ok(());
        // }
    }

    /// 查找匹配的路由（不可变）
    fn find<'a>(&'a self, path: &'a [u8]) -> Result<&'a UnsafeCell<T>, MatchError> {
        self.find_internal(path, &mut Vec::new())
    }

    /// 内部查找实现
    fn find_internal<'a>(
        &'a self,
        mut path: &'a [u8],
        backtrack: &mut Vec<(&'a Node<T>, &'a [u8])>,
    ) -> Result<&'a UnsafeCell<T>, MatchError> {
        let mut current = self;

        loop {
            // 检查前缀匹配
            if !path.starts_with(current.prefix.as_bytes()) {
                if let Some((node, remaining)) = backtrack.pop() {
                    current = node;
                    path = remaining;

                    continue;
                }
                return Err(MatchError::NotFound);
            }

            path = &path[current.prefix.len()..];

            // 如果完全匹配且有值，返回
            if path.is_empty() {
                if let Some(value) = &current.value {
                    return Ok(value);
                }
            }

            // 尝试静态子节点（按优先级排序）
            if let Some(next_char) = path.first() {
                let mut found = None;

                // 优先尝试静态节点
                for (i, child) in current.children.iter().enumerate() {
                    if child.is_static() && child.prefix.as_bytes()[0] == *next_char {
                        found = Some((i, child));
                        break;
                    }
                }

                if let Some((i, child)) = found {
                    // 如果有通配符子节点，保存回溯点
                    if current.wild_child {
                        backtrack.push((current, path));
                    }
                    current = child;
                    continue;
                }
            }

            // 尝试通配符子节点（如果有）
            if current.wild_child {
                if let Some(child) = current.children.last() {
                    match child.node_type {
                        NodeType::CatchSingle => {
                            // 匹配直到下一个斜杠或结束
                            let pos = path.iter().position(|&c| c == b'/');
                            let segment = if let Some(pos) = pos {
                                &path[..pos]
                            } else {
                                path
                            };

                            if segment.is_empty() {
                                // 空段不匹配
                                if let Some((node, remaining)) = backtrack.pop() {
                                    current = node;
                                    path = remaining;
                                    continue;
                                }
                                return Err(MatchError::NotFound);
                            }

                            // 消耗匹配的段
                            path = if let Some(pos) = pos {
                                &path[pos..]
                            } else {
                                b""
                            };

                            // 如果有值且路径结束，返回
                            if path.is_empty() {
                                if let Some(value) = &child.value {
                                    return Ok(value);
                                }
                            }

                            // 继续匹配子节点
                            current = child;
                            continue;
                        }
                        NodeType::CatchAll => {
                            // 匹配所有剩余路径
                            if let Some(value) = &child.value {
                                return Ok(value);
                            }
                        }
                        _ => {}
                    }
                }
            }

            // 没有匹配的子节点，尝试回溯
            if let Some((node, remaining)) = backtrack.pop() {
                current = node;
                path = remaining;
                continue;
            }

            return Err(MatchError::NotFound);
        }
    }

    /// 更新子节点优先级并排序
    fn update_child_priority(&mut self, child_idx: usize) {
        self.children[child_idx].priority += 1;
        let priority = self.children[child_idx].priority;

        // 保持子节点按优先级排序（高优先级在前）
        let mut i = child_idx;
        while i > 0 && self.children[i - 1].priority < priority {
            self.children.swap(i - 1, i);
            i -= 1;
        }
    }

    /// 检查是否是静态节点
    fn is_static(&self) -> bool {
        matches!(self.node_type, NodeType::Static)
    }

    /// 提取通配符信息
    fn extract_wildcard(pattern: &str) -> Option<WildcardInfo> {
        let start = pattern.find('*')?;

        // 处理 /** 模式
        if pattern[start..].starts_with("**") {
            return Some(WildcardInfo {
                range: start..start + 2,
                pattern: "**".to_string(),
                node_type: NodeType::CatchAll,
            });
        }

        // 处理 *.ext 模式
        if pattern[start..].contains('.') {
            let end = pattern[start..]
                .find('/')
                .map(|i| start + i)
                .unwrap_or(pattern.len());
            return Some(WildcardInfo {
                range: start..end,
                pattern: pattern[start..end].to_string(),
                node_type: NodeType::CatchSingleExtension,
            });
        }

        // 处理 * 模式
        Some(WildcardInfo {
            range: start..start + 1,
            pattern: "*".to_string(),
            node_type: NodeType::CatchSingle,
        })
    }
}

/// 通配符信息
struct WildcardInfo {
    range: Range<usize>,
    pattern: String,
    node_type: NodeType,
}

/// 节点类型
#[derive(Debug, Clone, PartialEq)]
enum NodeType {
    Root,
    Static,
    CatchSingle,
    CatchSingleExtension,
    CatchAll,
}

impl Into<u32> for NodeType {
    fn into(self) -> u32 {
        match self {
            NodeType::Root => 0,
            NodeType::Static => 1,
            NodeType::CatchSingleExtension => 2,
            NodeType::CatchSingle => 3,
            NodeType::CatchAll => 4,
        }
    }
}

/// 路由错误
#[derive(Debug, Clone)]
pub enum RouteError {
    Conflict { with: String },
    InvalidFormat,
    InvalidCatchAll,
}

impl fmt::Display for RouteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Conflict { with } => write!(f, "Route conflicts with existing route: {}", with),
            Self::InvalidFormat => write!(f, "Route must start with '/'"),
            Self::InvalidCatchAll => write!(f, "Catch-all must be at the end of route"),
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
        write!(f, "No matching route found")
    }
}

impl std::error::Error for MatchError {}

impl<T> Clone for Node<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        let value = self.value.as_ref().map(|value| {
            // Safety: We only expose `&mut T` through `&mut self`.
            let value = unsafe { &*value.get() };
            UnsafeCell::new(value.clone())
        });

        Self {
            value,
            prefix: self.prefix.clone(),
            wild_child: self.wild_child,
            node_type: self.node_type.clone(),
            children: self.children.clone(),
            priority: self.priority,
        }
    }
}

// 安全实现，因为 UnsafeCell 的使用是受控的
unsafe impl<T: Send> Send for Node<T> {}
unsafe impl<T: Sync> Sync for Node<T> {}
