//! 内置生成器：
//! - [`curl::CurlGenerator`]：cURL 命令行
//! - [`go::GoGenerator`]：Go (net/http)，含 Context 超时与错误处理
//! - [`java::JavaGenerator`]：Java (OkHttp)，单例客户端 + 构建模式
//! - [`python::PythonGenerator`]：Python (requests)，Pythonic 字面量
//! - [`mock::MockGenerator`]：测试/插件开发参考

pub mod curl;
pub mod go;
pub mod java;
pub mod mock;
pub mod python;

pub use curl::CurlGenerator;
pub use go::GoGenerator;
pub use java::JavaGenerator;
pub use mock::MockGenerator;
pub use python::PythonGenerator;
