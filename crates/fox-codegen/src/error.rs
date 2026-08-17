//! 代码生成引擎错误。

use thiserror::Error;

/// 代码生成引擎统一错误。
#[derive(Debug, Error, PartialEq, Eq)]
pub enum CodeGenError {
    /// 生成器不支持该 HTTP 方法。
    #[error("生成器不支持请求方法: {0}")]
    UnsupportedMethod(String),
    /// 生成器不支持该请求体类型。
    #[error("生成器不支持请求体: {0}")]
    UnsupportedBody(&'static str),
    /// 生成器不支持该认证方式。
    #[error("生成器不支持认证方式: {0}")]
    UnsupportedAuth(&'static str),
    /// 渲染目标代码过程中失败。
    #[error("渲染失败: {0}")]
    Render(String),
    /// 注册中心中不存在该语言的生成器。
    #[error("未注册代码生成器: {0}")]
    GeneratorNotFound(String),
    /// 向注册中心重复注册同一语言的生成器。
    #[error("代码生成器已注册: {0}")]
    GeneratorAlreadyRegistered(String),
    /// 输入 JSON 无法解析。
    #[error("JSON 解析失败: {0}")]
    JsonParse(String),
    /// 从 JSON 推断类型的形状不满足要求（如根节点非对象/对象数组）。
    #[error("类型推断失败: {0}")]
    TypeInference(String),
}
