//! 生成器核心 Trait。

use crate::error::CodeGenError;
use crate::model::ApiDefinition;

/// 生成器元信息（语言标识 + 目标 SDK 展示名）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LanguageInfo {
    /// 语言标识：注册中心的检索键（如 "java"、"go"）。
    pub name: &'static str,
    /// 目标 SDK 展示名（如 "Java (OkHttp)"、"Go (net/http)"）。
    pub sdk: &'static str,
}

/// 代码生成器：每种语言/SDK 一个实现，经注册中心动态接入引擎。
///
/// 实现必须满足：
/// - 对象安全（`&self`、返回 `&'static str`），可装箱为 `Box<dyn CodeGenerator>`；
/// - 线程安全（`Send + Sync`），注册中心为全局共享容器；
/// - `language_name` 返回值需稳定唯一，作为注册键。
pub trait CodeGenerator: Send + Sync {
    /// 语言标识（注册键），须保持唯一且稳定。
    fn language_name(&self) -> &'static str;

    /// 目标 SDK 展示名，如 "Java (OkHttp)"、"Go (net/http)"。
    fn target_sdk(&self) -> &'static str;

    /// 将统一 API 定义渲染为目标语言代码片段。
    fn generate(&self, api: &ApiDefinition) -> Result<String, CodeGenError>;

    /// 便捷方法：聚合元信息。
    fn metadata(&self) -> LanguageInfo {
        LanguageInfo {
            name: self.language_name(),
            sdk: self.target_sdk(),
        }
    }
}
