# Java → Rust 转换规则
## 1. 映射规则
- 泛型 Optional<T> → Option<T>
- 继承 → Deref 和 DerefMut + 组合
- 异常 → Result<T, Error>
- 日志 → tracing
- 抽象类 → Deref 和 DerefMut + 组合 + 拓展trait实现
- 泛型类 → 泛型 Struct + Trait Bounds  使用 `where` 子句 

## 2. 代码规范
- 源代码中有注释则必须要添加注释，必须是英文注释，注释转化时需要转化为符合rust的doc格式
- 无需添加java版本的 Copyright
- 代码风格保持大概 无需基本一致
- 将注释中的 关于java 或者 spring 相关的内容删除
- 将java版本一切的Exception 定义为 Error
- 当发现spring的Exception 是多继承的class 同时需要特定捕捉时，需要考虑转换rust错误结构体 含有kind, message等信息 kind方便进行对比某个错误
- 当结构体包含引用时，必须明确标注生命周期
- 使用 async_trait 宏处理异步 trait
- 实现 From trait 简化错误转换
- 多功能复用的函数需要写上测试用例
- Java无参构造函数对应的是rust Default函数
- 在函数运行时不能使用panic等中断程序的操作，在应用启动配置时允许可能需要中断程序的函数

## 3. 模块管理
- 当定义多个模块时可使用 `mod access_denied_error;  pub use access_denied_error::AccessDeniedError;` 这种方式可以避免 用户导入时候冗余的导入

## 4. 禁止事项
- 禁止使用 unsafe（除非明确要求）
- 保持原有命名风格：Java 用驼峰，Rust 用 snake_case
- 禁止在阻塞锁在异步环境
- .unwrap() 在生产代码
- println! 在生产日志 
- 在异步中使用 block_on
