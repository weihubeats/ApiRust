//! Mock 生成器：确定性输出，用于测试与插件开发参考。

use crate::engine::CodeGenerator;
use crate::error::CodeGenError;
use crate::model::ApiDefinition;

/// Mock 生成器：`generate` 始终返回固定格式字符串，不产出真实代码。
///
/// 用途：单元测试验证注册中心分发链路、UI 联调时的假数据源。
#[derive(Debug, Clone, Copy)]
pub struct MockGenerator {
    name: &'static str,
    sdk: &'static str,
}

impl MockGenerator {
    pub fn new(name: &'static str, sdk: &'static str) -> Self {
        MockGenerator { name, sdk }
    }
}

impl CodeGenerator for MockGenerator {
    fn language_name(&self) -> &'static str {
        self.name
    }

    fn target_sdk(&self) -> &'static str {
        self.sdk
    }

    fn generate(&self, api: &ApiDefinition) -> Result<String, CodeGenError> {
        Ok(format!(
            "mock:{}:{} {}?{}",
            self.name,
            api.method,
            api.url,
            api.query_params
                .iter()
                .map(|(key, value)| format!("{key}={value}"))
                .collect::<Vec<_>>()
                .join("&")
        ))
    }
}
