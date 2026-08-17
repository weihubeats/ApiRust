//! 生成器注册中心：全局单例 + 可独立实例。
//!
//! 引擎本身不硬编码任何语言：生成器全部由外部注册（`register`），
//! 注册键 = 生成器自报的 `language_name`。

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use crate::engine::{CodeGenerator, LanguageInfo};
use crate::error::CodeGenError;
use crate::model::ApiDefinition;

/// 生成器注册中心。
///
/// 线程安全；既可独立创建（测试/隔离场景），也提供进程级全局单例
/// [`GeneratorRegistry::global`]，应用启动时向单例注册内置生成器即可。
pub struct GeneratorRegistry {
    inner: Mutex<HashMap<&'static str, Arc<dyn CodeGenerator>>>,
}

impl GeneratorRegistry {
    /// 创建空注册中心。
    pub fn new() -> Self {
        GeneratorRegistry {
            inner: Mutex::new(HashMap::new()),
        }
    }

    /// 进程级全局单例注册中心。
    ///
    /// 首次调用时惰性初始化；应用应在启动阶段向其中注册生成器。
    pub fn global() -> &'static Self {
        static GLOBAL: OnceLock<GeneratorRegistry> = OnceLock::new();
        GLOBAL.get_or_init(GeneratorRegistry::new)
    }

    /// 注册生成器，注册键取其 `language_name`。
    ///
    /// # Errors
    /// 同键重复注册返回 [`CodeGenError::GeneratorAlreadyRegistered`]。
    pub fn register(&self, generator: impl CodeGenerator + 'static) -> Result<(), CodeGenError> {
        let key = generator.language_name();
        let mut inner = self.inner.lock().expect("registry lock poisoned");
        if inner.contains_key(key) {
            return Err(CodeGenError::GeneratorAlreadyRegistered(key.to_string()));
        }
        inner.insert(key, Arc::new(generator));
        Ok(())
    }

    /// 按语言标识提取生成器（返回共享句柄，可跨线程持有）。
    pub fn get(&self, language: &str) -> Option<Arc<dyn CodeGenerator>> {
        let inner = self.inner.lock().expect("registry lock poisoned");
        inner.get(language).cloned()
    }

    /// 是否已注册该语言。
    pub fn has(&self, language: &str) -> bool {
        self.get(language).is_some()
    }

    /// 按语言标识渲染。
    ///
    /// # Errors
    /// 未注册该语言时返回 [`CodeGenError::GeneratorNotFound`]，
    /// 其余错误透传生成器自身错误。
    pub fn generate(&self, language: &str, api: &ApiDefinition) -> Result<String, CodeGenError> {
        let inner = self.inner.lock().expect("registry lock poisoned");
        let generator = inner
            .get(language)
            .ok_or_else(|| CodeGenError::GeneratorNotFound(language.to_string()))?;
        generator.generate(api)
    }

    /// 已注册的全部生成器元信息（按语言标识排序，供 UI 选项列表使用）。
    pub fn languages(&self) -> Vec<LanguageInfo> {
        let inner = self.inner.lock().expect("registry lock poisoned");
        let mut infos: Vec<LanguageInfo> = inner.values().map(|gen| gen.metadata()).collect();
        infos.sort_by(|a, b| a.name.cmp(b.name));
        infos
    }

    /// 已注册的生成器数量。
    pub fn len(&self) -> usize {
        self.inner.lock().expect("registry lock poisoned").len()
    }

    /// 是否为空。
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for GeneratorRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use fox_core::model::HttpMethod;

    use super::*;
    use crate::model::{ApiDefinition, AuthInfo};

    /// Mock 生成器：不产出真实代码，仅验证注册/查找/分发链路。
    struct MockGenerator {
        name: &'static str,
        sdk: &'static str,
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
                "mock({}) {} {}#{}",
                self.name,
                api.method,
                api.url,
                api.headers.len()
            ))
        }
    }

    fn mock(name: &'static str) -> MockGenerator {
        MockGenerator {
            name,
            sdk: "Mock SDK",
        }
    }

    fn demo_api() -> ApiDefinition {
        ApiDefinition::new("https://api.example.com/users/1", HttpMethod::GET)
            .query("page", "1")
            .header("Accept", "application/json")
            .auth(AuthInfo::Bearer {
                token: "tok-1".into(),
            })
    }

    #[test]
    fn register_then_get_by_language() {
        let registry = GeneratorRegistry::new();
        registry.register(mock("java")).unwrap();

        assert!(registry.has("java"));
        let generator = registry.get("java").unwrap();
        assert_eq!(generator.language_name(), "java");
        assert_eq!(generator.target_sdk(), "Mock SDK");
        assert_eq!(
            generator.metadata(),
            LanguageInfo {
                name: "java",
                sdk: "Mock SDK",
            }
        );
        assert!(registry.get("go").is_none());
    }

    #[test]
    fn generate_dispatches_to_registered_generator() {
        let registry = GeneratorRegistry::new();
        registry.register(mock("curl")).unwrap();

        let code = registry.generate("curl", &demo_api()).unwrap();
        assert_eq!(code, "mock(curl) GET https://api.example.com/users/1#1");
    }

    #[test]
    fn unknown_language_returns_not_found() {
        let registry = GeneratorRegistry::new();
        registry.register(mock("go")).unwrap();

        let err = registry.generate("java", &demo_api()).unwrap_err();
        assert_eq!(err, CodeGenError::GeneratorNotFound("java".to_string()));
    }

    #[test]
    fn duplicate_register_rejected() {
        let registry = GeneratorRegistry::new();
        registry.register(mock("php")).unwrap();
        let err = registry.register(mock("php")).unwrap_err();

        assert_eq!(
            err,
            CodeGenError::GeneratorAlreadyRegistered("php".to_string())
        );
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn languages_lists_all_registered_sorted() {
        let registry = GeneratorRegistry::new();
        registry.register(mock("java")).unwrap();
        registry.register(mock("curl")).unwrap();
        registry.register(mock("go")).unwrap();

        let names: Vec<&str> = registry.languages().iter().map(|i| i.name).collect();
        assert_eq!(names, vec!["curl", "go", "java"]);
        assert!(!registry.is_empty());
    }

    #[test]
    fn global_singleton_is_shared() {
        let registry = GeneratorRegistry::global();
        registry.register(mock("mock-global-test")).unwrap();

        assert_eq!(
            GeneratorRegistry::global()
                .get("mock-global-test")
                .unwrap()
                .language_name(),
            "mock-global-test"
        );
    }

    #[test]
    fn generator_error_propagates() {
        struct FailingGenerator;

        impl CodeGenerator for FailingGenerator {
            fn language_name(&self) -> &'static str {
                "failing"
            }

            fn target_sdk(&self) -> &'static str {
                "Failing SDK"
            }

            fn generate(&self, _api: &ApiDefinition) -> Result<String, CodeGenError> {
                Err(CodeGenError::Render("boom".to_string()))
            }
        }

        let registry = GeneratorRegistry::new();
        registry.register(FailingGenerator).unwrap();
        let err = registry.generate("failing", &demo_api()).unwrap_err();
        assert_eq!(err, CodeGenError::Render("boom".to_string()));
    }
}
