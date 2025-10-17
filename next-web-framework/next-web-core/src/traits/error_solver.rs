use std::fmt::Debug;

use axum::response::IntoResponse;

/// 解决错误并将其转化为适当响应类型的特性
///
/// 此特性提供了一种将错误字符串转换为结构化响应类型的机制
/// 可以从HTTP处理程序返回。它允许自定义错误格式
/// 以及报头管理。
///
/// # 类型参数
///
/// *'T'-错误将转换为的响应类型。必须执行：
/// -`serde:：Serialize`用于序列化
/// -`Debug`            用于打印结构体
/// -`Send`和`Sync`     用于线程安全
/// -`Clone`            用于潜在的克隆
/// -`IntoRespons`      用于生成最终的返回响应
///
/// # 示例
///
/// ```
/// use axum::response::IntoResponse;
/// use your_crate::ErrorSolver;
///
/// // 使用默认字符串实现
/// let error = "Something went wrong".to_string();
/// let response = <() as ErrorSolver>::solve_error(error);
/// ```
///
/// A trait for solving and transforming errors into appropriate response types
///
/// This trait provides a mechanism to convert error strings into structured response types
/// that can be returned from HTTP handlers. It allows for customization of error formatting
/// and header management.
///
/// # Type Parameters
///
/// * `T` - The response type that the error will be transformed into. Must implement:
///   - `serde::Serialize`      for serialization
///   - `Debug`                 for logging and display purposes
///   - `Send` and `Sync`       for thread safety
///   - `Clone`                 for potential clones
///   - `IntoResponse`          Used to generate the final return response
///
/// # Examples
///
/// ```
/// use axum::response::IntoResponse;
/// use your_crate::ErrorSolver;
///
/// // Using the default string implementation
/// let error = "Something went wrong".to_string();
/// let response = <() as ErrorSolver>::solve_error(error);
/// ```
pub trait ErrorSolver<T = String>
where
    T: serde::Serialize,
    T: Debug + Clone,
    T: Send + Sync,
    T: IntoResponse,
{
    /// 将错误字符串转换为适当的响应类型
    ///
    /// 此方法接收原始错误字符串并将其转换为所需的响应类型`T`
    ///
    /// # 参数
    ///
    /// * `error`   - 要转换的错误消息
    ///
    /// # 返回
    ///
    /// 将转换后的错误作为类型 `T` 返回，准备用作HTTP响应
    ///
    /// Transforms an error string into an appropriate response type
    ///
    /// This method takes a raw error string and converts it into the desired response type `T`.
    ///
    /// # Parameters
    ///
    /// * `error`   - The error message to be transformed
    ///
    /// # Returns
    ///
    /// Returns the transformed error as type `T`, ready to be used as an HTTP response
    ///
    fn solve_error(error: String) -> T;
}

/// `()` 的 `ErrorSolver` 的默认实现
///
/// 此实现提供了返回纯文本响应的基本错误处理
/// 具有适当的内容类型标题。
///
/// Default implementation of `ErrorSolver` for the unit type `()`
///
/// This implementation provides basic error handling that returns plain text responses
/// with appropriate content type headers.
impl ErrorSolver for () {
    /// 将错误字符串转换为纯文本响应
    ///
    /// 本次实施：
    /// - 返回原始错误字符串作为响应体
    ///
    /// # 参数
    /// * `error`   -以纯文本形式返回的错误消息
    ///
    /// # 返回
    /// 返回原始错误字符串
    ///
    /// # 示例
    ///
    /// ```
    /// use your_crate:：ErrorSolver；
    ///
    /// let error_message = "Database connection failed".to_t_string（）；
    /// let response = <() as ErrorSolver>::solve_error(error_message);
    ///
    /// ```
    ///
    /// Transforms an error string into a plain text response
    ///
    /// This implementation:
    /// - Returns the original error string as the response body
    ///
    /// # Parameters
    ///
    /// * `error` - The error message to be returned as plain text
    ///
    /// # Returns
    ///
    /// Returns the original error string unchanged
    ///
    /// # Examples
    ///
    /// ```
    /// use your_crate::ErrorSolver;
    ///
    /// let error_message = "Database connection failed".to_string();
    /// let response = <() as ErrorSolver>::solve_error(error_message);
    ///
    /// println!("{:?}", response);
    /// ```
    fn solve_error(error: String) -> String {
        error
    }
}
