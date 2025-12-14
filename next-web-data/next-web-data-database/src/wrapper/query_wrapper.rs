use next_web_core::async_trait;
use rbatis::RBatis;
use serde::de::DeserializeOwned;
use std::{
    borrow::Cow,
    fmt::{Display, Formatter, Result as FmtResult},
    marker::PhantomData,
    result::Result as StdResult,
};

type Result<T> = StdResult<T, rbatis::Error>;

#[async_trait]
pub trait QueryWrapper {
    async fn select_list<T, V>(&self, wrapper: SelectWrapper<'static, T>) -> Result<Vec<T>>
    where
        T: DeserializeOwned + TableName + Send,
        V: DeserializeOwned,
    {
        self.execute_wrapper(wrapper).await
    }

    async fn select_one<T, V>(&self, wrapper: SelectWrapper<'static, T>) -> Result<T>
    where
        T: DeserializeOwned + TableName + Send,
        V: DeserializeOwned,
    {
        self.execute_wrapper(wrapper).await
    }

    async fn select_count<T, V>(&self, wrapper: SelectWrapper<'static, T>) -> Result<u64>
    where
        T: DeserializeOwned + TableName + Send,
        V: DeserializeOwned,
    {
        self.execute_wrapper(wrapper.select(vec!["COUNT(1) as count"]))
            .await
    }

    async fn select_by_id<T, V>(&self, id: V) -> Result<T>
    where
        T: DeserializeOwned + TableName + Send,
        V: Into<rbs::Value> + Send;

    async fn select_by_ids<T, V>(&self, ids: V) -> Result<T>
    where
        T: DeserializeOwned + TableName + Send,
        V: IntoIterator + Send,
        V::Item: Into<rbs::Value> + Send;

    async fn execute_wrapper<T, V>(&self, wrapper: SelectWrapper<'static, T>) -> Result<V>
    where
        T: DeserializeOwned + TableName + Send,
        V: DeserializeOwned;
}

#[async_trait]
impl QueryWrapper for RBatis {
    async fn execute_wrapper<T, V>(&self, wrapper: SelectWrapper<'static, T>) -> Result<V>
    where
        T: DeserializeOwned + TableName + Send,
        V: DeserializeOwned,
    {
        self.query_decode(&wrapper.generate_sql(), Vec::with_capacity(0))
            .await
    }

    async fn select_by_id<T, V>(&self, id: V) -> Result<T>
    where
        T: DeserializeOwned + TableName + Send,
        V: Into<rbs::Value> + Send,
    {
        self.query_decode(
            &format!("SELECT * FROM {} WHERE id = ?", T::table_name()),
            vec![id.into()],
        )
        .await
    }

    async fn select_by_ids<T, V>(&self, ids: V) -> Result<T>
    where
        T: DeserializeOwned + TableName + Send,
        V: IntoIterator + Send,
        V::Item: Into<rbs::Value> + Send,
    {
        let ids = ids
            .into_iter()
            .map(Into::<rbs::Value>::into)
            .collect::<Vec<_>>();
        self.query_decode(
            &format!(
                "SELECT * FROM {} WHERE id IN ({})",
                T::table_name(),
                vec!["?"; ids.len()].join(", ")
            ),
            ids,
        )
        .await
    }
}

/// 定义一个 trait，用于获取表名
pub trait TableName {
    fn table_name() -> &'static str;
}

/// 定义一个 Wrapper trait，用于生成 SQL 语句
pub trait Wrapper<T>
where
    T: DeserializeOwned,
    T: TableName,
{
    fn generate_sql(&self) -> String;
}

/// 比较操作符枚举
#[derive(Debug, Clone, PartialEq)]
pub enum CompareOperator {
    Eq,         // 等于
    Ne,         // 不等于
    Gt,         // 大于
    Ge,         // 大于等于
    Lt,         // 小于
    Le,         // 小于等于
    Like,       // 模糊匹配
    NotLike,    // 不匹配
    LikeLeft,   // 左模糊匹配
    LikeRight,  // 右模糊匹配
    IsNull,     // 为空
    IsNotNull,  // 不为空
    In,         // 在列表中
    NotIn,      // 不在列表中
    Between,    // 在范围内
    NotBetween, // 不在范围内
}

impl Display for CompareOperator {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            CompareOperator::Eq => write!(f, "="),
            CompareOperator::Ne => write!(f, "<>"),
            CompareOperator::Gt => write!(f, ">"),
            CompareOperator::Ge => write!(f, ">="),
            CompareOperator::Lt => write!(f, "<"),
            CompareOperator::Le => write!(f, "<="),
            CompareOperator::Like => write!(f, "LIKE"),
            CompareOperator::NotLike => write!(f, "NOT LIKE"),
            CompareOperator::LikeLeft => write!(f, "LIKE"),
            CompareOperator::LikeRight => write!(f, "LIKE"),
            CompareOperator::IsNull => write!(f, "IS NULL"),
            CompareOperator::IsNotNull => write!(f, "IS NOT NULL"),
            CompareOperator::In => write!(f, "IN"),
            CompareOperator::NotIn => write!(f, "NOT IN"),
            CompareOperator::Between => write!(f, "BETWEEN"),
            CompareOperator::NotBetween => write!(f, "NOT BETWEEN"),
        }
    }
}

/// 逻辑操作符枚举
#[derive(Debug, Clone, PartialEq)]
pub enum LogicalOperator {
    And,
    Or,
}

impl Display for LogicalOperator {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            LogicalOperator::And => write!(f, "AND"),
            LogicalOperator::Or => write!(f, "OR"),
        }
    }
}

/// 排序方向枚举
#[derive(Debug, Clone, PartialEq)]
pub enum OrderDirection {
    Asc,
    Desc,
}

impl Display for OrderDirection {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            OrderDirection::Asc => write!(f, "ASC"),
            OrderDirection::Desc => write!(f, "DESC"),
        }
    }
}

/// 查询条件值枚举，减少内存占用
#[derive(Debug, Clone)]
pub enum ConditionValue<'a> {
    None,
    Single(Cow<'a, str>),
    Multiple(Vec<Cow<'a, str>>),
    Range(Cow<'a, str>, Cow<'a, str>),
}

/// 查询条件结构
#[derive(Debug, Clone)]
pub struct Condition<'a> {
    pub column: Cow<'a, str>,
    pub operator: CompareOperator,
    pub value: ConditionValue<'a>,
}

/// 排序条件结构
#[derive(Debug, Clone)]
pub struct OrderBy<'a> {
    pub column: Cow<'a, str>,
    pub direction: OrderDirection,
}

/// 查询条件节点枚举，用于构建条件树
#[derive(Debug, Clone)]
pub enum ConditionNode<'a> {
    Leaf(Condition<'a>),
    Branch {
        left: Box<ConditionNode<'a>>,
        op: LogicalOperator,
        right: Box<ConditionNode<'a>>,
    },
    Group(Box<ConditionNode<'a>>),
    Empty,
}

impl<'a> Default for ConditionNode<'a> {
    fn default() -> Self {
        ConditionNode::Empty
    }
}

#[derive(Debug, Clone, Default)]
pub enum JoinMode {
    #[default]
    And,
    Or,
}

/// 优化后的QueryWrapper结构体
#[derive(Debug, Clone, Default)]
pub struct SelectWrapper<'a, T>
where
    T: DeserializeOwned,
    T: TableName,
{
    root_condition: ConditionNode<'a>,
    order_by: Vec<OrderBy<'a>>,
    group_by: Vec<Cow<'a, str>>,
    having: Option<Cow<'a, str>>,
    limit: Option<usize>,
    offset: Option<usize>,
    select_columns: Vec<Cow<'a, str>>,
    custom_sql: Option<Cow<'a, str>>,
    _marker: PhantomData<T>,
    join_mode: JoinMode,
}

impl<'a, T> SelectWrapper<'a, T>
where
    T: DeserializeOwned,
    T: TableName,
{
    /// 创建一个新的QueryWrapper实例
    pub fn new() -> Self {
        Self {
            root_condition: ConditionNode::Empty,
            order_by: Vec::new(),
            group_by: Vec::new(),
            having: None,
            limit: None,
            offset: None,
            select_columns: Vec::new(),
            custom_sql: None,
            _marker: PhantomData,
            join_mode: JoinMode::And,
        }
    }

    /// 添加条件节点
    fn add_condition(&mut self, condition: Condition<'a>) {
        let new_node = ConditionNode::Leaf(condition);

        match &self.root_condition {
            ConditionNode::Empty => {
                self.root_condition = new_node;
            }
            _ => {
                let op = match self.join_mode {
                    JoinMode::And => LogicalOperator::And,
                    JoinMode::Or => LogicalOperator::Or,
                };
                let old_root = std::mem::replace(&mut self.root_condition, ConditionNode::Empty);
                self.root_condition = ConditionNode::Branch {
                    left: Box::new(old_root),
                    op,
                    right: Box::new(new_node),
                };
                self.join_mode = JoinMode::And; // 重置为默认
            }
        }
    }

    /// 等于条件
    pub fn eq<F, V>(mut self, column: F, value: V) -> Self
    where
        F: FnOnce() -> V,
        V: Into<Cow<'a, str>>,
    {
        let column_str = column().into();
        self.add_condition(Condition {
            column: column_str,
            operator: CompareOperator::Eq,
            value: ConditionValue::Single(value.into()),
        });
        self
    }

    /// 不等于条件
    pub fn ne<F, V>(mut self, column: F, value: V) -> Self
    where
        F: FnOnce() -> V,
        V: Into<Cow<'a, str>>,
    {
        let column_str = column().into();
        self.add_condition(Condition {
            column: column_str,
            operator: CompareOperator::Ne,
            value: ConditionValue::Single(value.into()),
        });
        self
    }

    /// 大于条件
    pub fn gt<F, V>(mut self, column: F, value: V) -> Self
    where
        F: FnOnce() -> V,
        V: Into<Cow<'a, str>>,
    {
        let column_str = column().into();
        self.add_condition(Condition {
            column: column_str,
            operator: CompareOperator::Gt,
            value: ConditionValue::Single(value.into()),
        });
        self
    }

    /// 大于等于条件
    pub fn ge<F, V>(mut self, column: F, value: V) -> Self
    where
        F: FnOnce() -> V,
        V: Into<Cow<'a, str>>,
    {
        let column_str = column().into();
        self.add_condition(Condition {
            column: column_str,
            operator: CompareOperator::Ge,
            value: ConditionValue::Single(value.into()),
        });
        self
    }

    /// 小于条件
    pub fn lt<F, V>(mut self, column: F, value: V) -> Self
    where
        F: FnOnce() -> V,
        V: Into<Cow<'a, str>>,
    {
        let column_str = column().into();
        self.add_condition(Condition {
            column: column_str,
            operator: CompareOperator::Lt,
            value: ConditionValue::Single(value.into()),
        });
        self
    }

    /// 小于等于条件
    pub fn le<F, V>(mut self, column: F, value: V) -> Self
    where
        F: FnOnce() -> V,
        V: Into<Cow<'a, str>>,
    {
        let column_str = column().into();
        self.add_condition(Condition {
            column: column_str,
            operator: CompareOperator::Le,
            value: ConditionValue::Single(value.into()),
        });
        self
    }

    /// 模糊匹配条件
    pub fn like<F, V>(mut self, column: F, value: V) -> Self
    where
        F: FnOnce() -> V,
        V: Into<Cow<'a, str>>,
    {
        let column_str = column().into();
        let value_str = value.into();
        let formatted_value = format!("%{}%", value_str);

        self.add_condition(Condition {
            column: column_str,
            operator: CompareOperator::Like,
            value: ConditionValue::Single(Cow::Owned(formatted_value)),
        });
        self
    }

    /// 不匹配条件
    pub fn not_like<F, V>(mut self, column: F, value: V) -> Self
    where
        F: FnOnce() -> V,
        V: Into<Cow<'a, str>>,
    {
        let column_str = column().into();
        let value_str = value.into();
        let formatted_value = format!("%{}%", value_str);

        self.add_condition(Condition {
            column: column_str,
            operator: CompareOperator::NotLike,
            value: ConditionValue::Single(Cow::Owned(formatted_value)),
        });
        self
    }

    /// 左模糊匹配条件
    pub fn like_left<F, V>(mut self, column: F, value: V) -> Self
    where
        F: FnOnce() -> V,
        V: Into<Cow<'a, str>>,
    {
        let column_str = column().into();
        let value_str = value.into();
        let formatted_value = format!("%{}", value_str);

        self.add_condition(Condition {
            column: column_str,
            operator: CompareOperator::LikeLeft,
            value: ConditionValue::Single(Cow::Owned(formatted_value)),
        });
        self
    }

    /// 右模糊匹配条件
    pub fn like_right<F, V>(mut self, column: F, value: V) -> Self
    where
        F: FnOnce() -> V,
        V: Into<Cow<'a, str>>,
    {
        let column_str = column().into();
        let value_str = value.into();
        let formatted_value = format!("{}%", value_str);

        self.add_condition(Condition {
            column: column_str,
            operator: CompareOperator::LikeRight,
            value: ConditionValue::Single(Cow::Owned(formatted_value)),
        });
        self
    }

    /// 为空条件
    pub fn is_null<F, V>(mut self, column: F) -> Self
    where
        F: FnOnce() -> V,
        V: Into<Cow<'a, str>>,
    {
        let column_str = column().into();
        self.add_condition(Condition {
            column: column_str,
            operator: CompareOperator::IsNull,
            value: ConditionValue::None,
        });
        self
    }

    /// 不为空条件
    pub fn is_not_null<F, V>(mut self, column: F) -> Self
    where
        F: FnOnce() -> V,
        V: Into<Cow<'a, str>>,
    {
        let column_str = column().into();
        self.add_condition(Condition {
            column: column_str,
            operator: CompareOperator::IsNotNull,
            value: ConditionValue::None,
        });
        self
    }

    /// 在列表中条件
    pub fn r#in<F, V, I>(mut self, column: F, values: I) -> Self
    where
        F: FnOnce() -> V,
        V: Into<Cow<'a, str>>,
        I: IntoIterator<Item = V>,
    {
        let column_str = column().into();
        let values_vec: Vec<Cow<'a, str>> = values.into_iter().map(|v| v.into()).collect();

        self.add_condition(Condition {
            column: column_str,
            operator: CompareOperator::In,
            value: ConditionValue::Multiple(values_vec),
        });
        self
    }

    /// 不在列表中条件
    pub fn not_in<F, V, I>(mut self, column: F, values: I) -> Self
    where
        F: FnOnce() -> V,
        V: Into<Cow<'a, str>>,
        I: IntoIterator<Item = V>,
    {
        let column_str = column().into();
        let values_vec: Vec<Cow<'a, str>> = values.into_iter().map(|v| v.into()).collect();
        self.add_condition(Condition {
            column: column_str,
            operator: CompareOperator::NotIn,
            value: ConditionValue::Multiple(values_vec),
        });
        self
    }

    /// 在范围内条件
    pub fn between<F, V>(mut self, column: F, value1: V, value2: V) -> Self
    where
        F: FnOnce() -> V,
        V: Into<Cow<'a, str>>,
    {
        let column_str = column().into();
        self.add_condition(Condition {
            column: column_str,
            operator: CompareOperator::Between,
            value: ConditionValue::Range(value1.into(), value2.into()),
        });
        self
    }

    /// 不在范围内条件
    pub fn not_between<F, V>(mut self, column: F, value1: V, value2: V) -> Self
    where
        F: FnOnce() -> V,
        V: Into<Cow<'a, str>>,
    {
        let column_str = column().into();
        self.add_condition(Condition {
            column: column_str,
            operator: CompareOperator::NotBetween,
            value: ConditionValue::Range(value1.into(), value2.into()),
        });
        self
    }

    /// 添加OR逻辑操作符
    pub fn or(mut self) -> Self {
        self.join_mode = JoinMode::Or;
        self
    }

    /// 修改最后一个操作符
    #[allow(unused)]
    fn change_last_operator(&mut self, op: LogicalOperator) {
        fn find_last_branch<'b>(node: &'b mut ConditionNode) -> Option<&'b mut LogicalOperator> {
            match node {
                ConditionNode::Branch { op, right, .. } => {
                    if let Some(last_op) = find_last_branch(right) {
                        Some(last_op)
                    } else {
                        Some(op)
                    }
                }
                ConditionNode::Group(inner) => find_last_branch(inner),
                _ => None,
            }
        }

        if let Some(last_op) = find_last_branch(&mut self.root_condition) {
            *last_op = op;
        }
    }

    /// 添加嵌套条件
    pub fn nested<F>(mut self, f: F) -> Self
    where
        F: FnOnce(SelectWrapper<'a, T>) -> SelectWrapper<'a, T>,
    {
        let nested = f(SelectWrapper::new());

        if let ConditionNode::Empty = nested.root_condition {
            return self;
        }

        let nested_node = ConditionNode::Group(Box::new(nested.root_condition));

        match &self.root_condition {
            ConditionNode::Empty => {
                self.root_condition = nested_node;
            }
            _ => {
                let op = match self.join_mode {
                    JoinMode::And => LogicalOperator::And,
                    JoinMode::Or => LogicalOperator::Or,
                };
                let old_root = std::mem::replace(&mut self.root_condition, ConditionNode::Empty);
                self.root_condition = ConditionNode::Branch {
                    left: Box::new(old_root),
                    op,
                    right: Box::new(nested_node),
                };
                self.join_mode = JoinMode::And; // 重置为默认
            }
        }

        self
    }

    /// 添加自定义SQL
    pub fn apply<V: Into<Cow<'a, str>>>(mut self, sql: V) -> Self {
        self.custom_sql = Some(sql.into());
        self
    }

    /// 设置排序条件
    pub fn order_by<F, V>(mut self, column: F, direction: OrderDirection) -> Self
    where
        F: FnOnce() -> V,
        V: Into<Cow<'a, str>>,
    {
        let column_str = column().into();
        self.order_by.push(OrderBy {
            column: column_str,
            direction,
        });
        self
    }

    /// 设置升序排序
    pub fn order_by_asc<F, V>(self, column: F) -> Self
    where
        F: FnOnce() -> V,
        V: Into<Cow<'a, str>>,
    {
        self.order_by(column, OrderDirection::Asc)
    }

    /// 设置降序排序
    pub fn order_by_desc<F, V>(self, column: F) -> Self
    where
        F: FnOnce() -> V,
        V: Into<Cow<'a, str>>,
    {
        self.order_by(column, OrderDirection::Desc)
    }

    /// 设置分组条件
    pub fn group_by<V, I>(mut self, columns: I) -> Self
    where
        V: Into<Cow<'a, str>>,
        I: IntoIterator<Item = V>,
    {
        self.group_by = columns.into_iter().map(|c| c.into()).collect();
        self
    }

    /// 设置HAVING条件
    pub fn having<V: Into<Cow<'a, str>>>(mut self, condition: V) -> Self {
        self.having = Some(condition.into());
        self
    }

    /// 设置查询列
    pub fn select<V, I>(mut self, columns: I) -> Self
    where
        V: Into<Cow<'a, str>>,
        I: IntoIterator<Item = V>,
    {
        self.select_columns = columns.into_iter().map(|c| c.into()).collect();
        self
    }

    /// 设置分页
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// 设置偏移量
    pub fn offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }

    /// 生成WHERE子句
    pub fn build_where_clause(&self) -> String {
        fn build_condition_node(node: &ConditionNode) -> String {
            match node {
                ConditionNode::Empty => String::new(),
                ConditionNode::Leaf(condition) => match &condition.operator {
                    CompareOperator::IsNull | CompareOperator::IsNotNull => {
                        format!("{} {}", condition.column, condition.operator)
                    }
                    CompareOperator::In | CompareOperator::NotIn => {
                        if let ConditionValue::Multiple(values) = &condition.value {
                            let values_str = values
                                .iter()
                                .map(|v| format!("'{}'", v))
                                .collect::<Vec<_>>()
                                .join(", ");
                            format!(
                                "{} {} ({})",
                                condition.column, condition.operator, values_str
                            )
                        } else {
                            String::new()
                        }
                    }
                    CompareOperator::Between | CompareOperator::NotBetween => {
                        if let ConditionValue::Range(value1, value2) = &condition.value {
                            format!(
                                "{} {} '{}' AND '{}'",
                                condition.column, condition.operator, value1, value2
                            )
                        } else {
                            String::new()
                        }
                    }
                    _ => {
                        if let ConditionValue::Single(value) = &condition.value {
                            format!("{} {} '{}'", condition.column, condition.operator, value)
                        } else {
                            String::new()
                        }
                    }
                },
                ConditionNode::Branch { left, op, right } => {
                    let left_str = build_condition_node(left);
                    let right_str = build_condition_node(right);

                    if left_str.is_empty() {
                        right_str
                    } else if right_str.is_empty() {
                        left_str
                    } else {
                        format!("{} {} {}", left_str, op, right_str)
                    }
                }
                ConditionNode::Group(inner) => {
                    let inner_str = build_condition_node(inner);
                    if inner_str.is_empty() {
                        inner_str
                    } else {
                        format!("({})", inner_str)
                    }
                }
            }
        }

        let condition_str = build_condition_node(&self.root_condition);

        if condition_str.is_empty() && self.custom_sql.is_none() {
            return String::new();
        }

        let mut where_clause = String::from("WHERE ");

        if !condition_str.is_empty() {
            where_clause.push_str(&condition_str);
        }

        if let Some(sql) = &self.custom_sql {
            if !condition_str.is_empty() {
                where_clause.push_str(" AND ");
            }
            where_clause.push_str(sql);
        }

        where_clause
    }

    /// 生成ORDER BY子句
    pub fn build_order_by_clause(&self) -> String {
        if self.order_by.is_empty() {
            return String::new();
        }

        let order_by_str = self
            .order_by
            .iter()
            .map(|o| format!("{} {}", o.column, o.direction))
            .collect::<Vec<_>>()
            .join(", ");

        format!("ORDER BY {}", order_by_str)
    }

    /// 生成GROUP BY子句
    pub fn build_group_by_clause(&self) -> String {
        if self.group_by.is_empty() {
            return String::new();
        }

        format!("GROUP BY {}", self.group_by.join(", "))
    }

    /// 生成HAVING子句
    pub fn build_having_clause(&self) -> String {
        if let Some(having) = &self.having {
            format!("HAVING {}", having)
        } else {
            String::new()
        }
    }

    /// 生成LIMIT和OFFSET子句
    pub fn build_limit_offset_clause(&self) -> String {
        let mut clause = String::new();

        if let Some(limit) = self.limit {
            clause.push_str(&format!("LIMIT {}", limit));
        }

        if let Some(offset) = self.offset {
            if !clause.is_empty() {
                clause.push_str(" ");
            }
            clause.push_str(&format!("OFFSET {}", offset));
        }

        clause
    }

    /// 生成SELECT子句
    pub fn build_select_clause(&self) -> String {
        if self.select_columns.is_empty() {
            return String::from("*");
        }

        self.select_columns.join(", ")
    }

    /// 生成完整的SQL查询语句
    fn build_sql(&self, table_name: &str) -> String {
        let select_clause = self.build_select_clause();
        let where_clause = self.build_where_clause();
        let group_by_clause = self.build_group_by_clause();
        let having_clause = self.build_having_clause();
        let order_by_clause = self.build_order_by_clause();
        let limit_offset_clause = self.build_limit_offset_clause();

        let mut sql = format!("SELECT {} FROM {}", select_clause, table_name);

        if !where_clause.is_empty() {
            sql.push_str(&format!(" {}", where_clause));
        }

        if !group_by_clause.is_empty() {
            sql.push_str(&format!(" {}", group_by_clause));
        }

        if !having_clause.is_empty() {
            sql.push_str(&format!(" {}", having_clause));
        }

        if !order_by_clause.is_empty() {
            sql.push_str(&format!(" {}", order_by_clause));
        }

        if !limit_offset_clause.is_empty() {
            sql.push_str(&format!(" {}", limit_offset_clause));
        }

        sql
    }

    /// 将PascalCase转换为snake_case
    #[allow(unused)]
    fn to_snake_case(pascal_case: &str) -> Cow<'_, str> {
        if pascal_case.chars().all(|c| c.is_lowercase() || c == '_') {
            return Cow::Borrowed(pascal_case);
        }

        let mut name = String::with_capacity(pascal_case.len() + 4);
        let mut chars = pascal_case.chars().peekable();

        while let Some(c) = chars.next() {
            if c.is_uppercase() {
                if !name.is_empty() && chars.peek().map_or(false, |next| next.is_lowercase()) {
                    name.push('_');
                }
                name.push(c.to_lowercase().next().unwrap());
            } else {
                name.push(c);
            }
        }
        Cow::Owned(name)
    }
}

impl<'a, T> Wrapper<T> for SelectWrapper<'_, T>
where
    T: DeserializeOwned,
    T: TableName,
{
    fn generate_sql(&self) -> String {
        self.build_sql(T::table_name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Clone, Default, Deserialize)]
    struct User;

    impl User {
        pub fn name() -> &'static str {
            "name"
        }
    }

    impl TableName for User {
        fn table_name() -> &'static str {
            "user"
        }
    }

    #[test]
    fn test_simple_query() {
        let query = SelectWrapper::<User>::new()
            .eq(|| "name", "张三")
            .gt(|| "age", "18")
            .generate_sql();

        assert_eq!(
            query,
            "SELECT * FROM user WHERE name = '张三' AND age > '18'"
        );
    }

    #[test]
    fn test_complex_query() {
        let query = SelectWrapper::<User>::new()
            .eq(|| "status", "active")
            .or()
            .nested(|q| q.eq(|| "role", "admin").gt(|| "level", "5"))
            .order_by_desc(|| "created_at")
            .limit(10)
            .offset(20)
            .generate_sql();

        assert_eq!(
            query,
            "SELECT * FROM user WHERE status = 'active' OR (role = 'admin' AND level > '5') ORDER BY created_at DESC LIMIT 10 OFFSET 20"
        );
    }

    #[test]
    fn test_in_condition() {
        let query = SelectWrapper::<User>::new()
            .r#in(|| "id", vec!["1", "2", "3"])
            .generate_sql();

        assert_eq!(query, "SELECT * FROM user WHERE id IN ('1', '2', '3')");
    }

    #[test]
    fn test_between_condition() {
        let query = SelectWrapper::<User>::new()
            .between(|| "age", "18", "30")
            .generate_sql();

        assert_eq!(query, "SELECT * FROM user WHERE age BETWEEN '18' AND '30'");
    }

    #[test]
    fn test_like_condition() {
        let query = SelectWrapper::<User>::new()
            .like(User::name, "张")
            .generate_sql();

        assert_eq!(query, "SELECT * FROM user WHERE name LIKE '%张%'");
    }

    #[test]
    fn test_group_by_having() {
        let query = SelectWrapper::<User>::new()
            .select(vec!["department", "COUNT(*) as count"])
            .group_by(vec!["department"])
            .having("COUNT(*) > 5")
            .build_sql("employees");

        assert_eq!(
            query,
            "SELECT department, COUNT(*) as count FROM employees GROUP BY department HAVING COUNT(*) > 5"
        );
    }

    #[test]
    fn test_count() {
        let query = SelectWrapper::<User>::new()
            .select(vec!["COUNT(1) as count"])
            .eq(|| "name", "zhangsan")
            .generate_sql();
        assert_eq!(
            query,
            "SELECT COUNT(1) as count FROM user WHERE name = 'zhangsan'"
        );
    }
}
