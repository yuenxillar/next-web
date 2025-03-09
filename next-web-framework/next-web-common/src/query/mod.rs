pub mod query_wrapper;

/// 这个模块提供了QueryWrapper功能，用于构建SQL查询条件
/// 
/// # 示例
/// 
/// ```rust
/// use next_web_common::query::query_wrapper::{QueryWrapper, OrderDirection};
/// 
/// // 创建一个简单的查询
/// let query = QueryWrapper::new()
///     .eq("name", "张三")
///     .gt("age", "18")
///     .build_sql("users");
/// 
/// // 创建一个复杂的查询
/// let complex_query = QueryWrapper::new()
///     .eq("status", "active")
///     .or()
///     .nested(|q| q.eq("role", "admin").gt("level", "5"))
///     .order_by_desc("created_at")
///     .limit(10)
///     .offset(20)
///     .build_sql("users");
/// 
/// // 使用IN条件
/// let in_query = QueryWrapper::new()
///     .r#in("id", vec!["1", "2", "3"])
///     .build_sql("users");
/// 
/// // 使用BETWEEN条件
/// let between_query = QueryWrapper::new()
///     .between("age", "18", "30")
///     .build_sql("users");
/// 
/// // 使用LIKE条件
/// let like_query = QueryWrapper::new()
///     .like("name", "张")
///     .build_sql("users");
/// 
/// // 使用GROUP BY和HAVING
/// let group_query = QueryWrapper::new()
///     .select(vec!["department", "COUNT(*) as count"])
///     .group_by(vec!["department"])
///     .having("COUNT(*) > 5")
///     .build_sql("employees");
/// ```
pub struct QueryExample;
