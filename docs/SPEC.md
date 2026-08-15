# RustFox：基于 Rust 的 API 管理工具可执行开发文档

> 本项目参考 Apifox 的产品形态，但不复制其名称、Logo、UI 资产、私有格式。
> 目标是实现一个本地优先、单机可用的 API 管理工具，覆盖 API 设计、调试、Mock、测试、文档管理等核心能力。
> 第一阶段目标是可交付的 MVP，后续再扩展协作与云端能力。

> **架构迁移说明(2026-08)**:UI 层已从 Dioxus 桌面(`crates/fox-desktop`,已删除)迁移至
> Tauri 2(`frontend/src-tauri` + `frontend/` Vue 3)。本文下方目录结构为历史设计蓝图,
> 当前实际结构以 [TAURI_MIGRATION.md](TAURI_MIGRATION.md) 与仓库为准,`fox-core/fox-storage/fox-http/
> fox-openapi/fox-mock/fox-test/fox-oauth/fox-backup/fox-codegen/fox-smoke` 核心 crate 全部保留。

---

## 0. 给 AI Coder 的强制执行指令

你现在是一个 Rust 全栈工程师与架构师，需要按照本文档实现一个名为 `RustFox` 的 API 管理工具。

### 0.1 执行原则

1. 必须严格按照本文档的里程碑顺序实现。
2. 不允许引入 Node.js 前端框架，例如 React、Vue、Svelte、Angular。
3. 不允许使用 TypeScript / JavaScript 编写业务逻辑。
4. 所有核心业务逻辑必须使用 Rust 实现。
5. UI 使用 Dioxus 的 Rust 组件实现。
6. 数据库使用本地 SQLite。
7. 所有用户可见错误必须友好提示，不允许直接 panic。
8. 每个里程碑完成后必须执行：

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

9. 任何一个阶段没有通过测试，不允许进入下一阶段。
10. 必须在仓库中维护以下进度文件：

```text
docs/PROGRESS.md
```

每完成一个里程碑，追加一条记录。

---

## 1. 产品定义

### 1.1 产品名称

```text
RustFox
```

名称仅为临时名称，后续可替换。

### 1.2 产品定位

RustFox 是一个本地优先的 API 管理工具，提供：

1. API 接口管理
2. API 文档管理
3. HTTP 请求调试
4. 环境变量管理
5. Mock Server
6. 自动化测试
7. OpenAPI 导入导出
8. 请求历史记录
9. 本地备份与恢复

### 1.3 第一阶段目标

第一阶段只实现单机本地版，不实现云端协作。

第一阶段必须实现：

1. 多项目管理
2. 接口目录树
3. 接口创建、编辑、删除
4. HTTP 请求发送
5. 环境变量与变量替换
6. OpenAPI 3.x 导入导出
7. 本地 Mock Server
8. 简单自动化测试
9. API 文档导出
10. 请求历史

### 1.4 第一阶段不实现

以下功能第一阶段不做：

1. 团队云端协作
2. 用户登录注册
3. 权限系统
4. 云端同步
5. WebSocket 调试
6. GraphQL 调试
7. gRPC 调试
8. 性能压测
9. Postman 格式导入
10. JavaScript 脚本系统

---

## 2. 技术栈

### 2.1 总体技术栈

| 模块 | 技术 |
|---|---|
| 语言 | Rust |
| UI | Dioxus Desktop |
| 异步运行时 | Tokio |
| 本地数据库 | SQLite |
| 数据库访问 | SQLx |
| HTTP Client | reqwest |
| Mock Server | axum |
| OpenAPI | openapiv3 |
| JSONPath | jsonpath-rust |
| 日志 | tracing + tracing-subscriber |
| 文件对话框 | rfd |
| 脚本能力 | 第一阶段不做，后续可用 Rhai |

### 2.2 Rust 版本

使用 stable：

```toml
# rust-toolchain.toml
[toolchain]
channel = "stable"
```

最低要求：

```text
Rust 1.79+
```

### 2.3 UI 技术约束

UI 使用 Dioxus Desktop。

禁止使用：

```text
React
Vue
Svelte
Angular
TypeScript
JavaScript 业务代码
```

允许 Dioxus 内部通过 webview 渲染 HTML/CSS，但所有业务逻辑必须由 Rust 控制。

---

## 3. 功能范围

## 3.1 MVP 功能列表

| 模块 | 功能 | 是否必须 |
|---|---|---|
| 项目 | 创建项目 | 必须 |
| 项目 | 删除项目 | 必须 |
| 项目 | 切换项目 | 必须 |
| 目录 | 创建文件夹 | 必须 |
| 目录 | 树形展示 | 必须 |
| 接口 | 新建接口 | 必须 |
| 接口 | 编辑接口 | 必须 |
| 接口 | 删除接口 | 必须 |
| 接口 | 复制接口 | 必须 |
| 调试 | GET/POST/PUT/DELETE/PATCH | 必须 |
| 调试 | Query 参数 | 必须 |
| 调试 | Header 参数 | 必须 |
| 调试 | Path 参数 | 必须 |
| 调试 | Cookie 参数 | 第一阶段可选 |
| 调试 | JSON Body | 必须 |
| 调试 | Text Body | 必须 |
| 调试 | x-www-form-urlencoded | 必须 |
| 调试 | multipart/form-data | 可选 |
| 环境 | 环境管理 | 必须 |
| 环境 | 环境变量 | 必须 |
| 环境 | 变量替换 | 必须 |
| Mock | 启动 Mock Server | 必须 |
| Mock | 根据接口返回 Mock | 必须 |
| Mock | 自定义 Mock 规则 | 必须 |
| 测试 | 单接口测试 | 必须 |
| 测试 | 集合测试 | 必须 |
| 测试 | 断言 | 必须 |
| 测试 | 变量提取 | 必须 |
| 文档 | Markdown 导出 | 必须 |
| 文档 | OpenAPI JSON 导出 | 必须 |
| 历史 | 请求历史 | 必须 |
| 备份 | 本地 JSON 备份 | 必须 |
| 备份 | JSON 恢复 | 必须 |

---

## 4. 系统架构

### 4.1 总体架构

```text
┌────────────────────────────────────────────┐
│                 Dioxus UI                  │
│                                            │
│  ProjectTree / Editor / Response / Mock    │
└──────────────────────┬─────────────────────┘
                       │
┌──────────────────────▼─────────────────────┐
│              Application Services          │
│                                            │
│ ProjectService / EndpointService           │
│ EnvironmentService / HttpService           │
│ MockService / TestService                  │
└──────┬──────────┬──────────┬───────────────┘
       │          │          │
┌──────▼───┐ ┌────▼────┐ ┌───▼────────┐
│ Storage  │ │ HTTP    │ │ OpenAPI    │
│ SQLite   │ │ Client  │ │ Import/Exp │
└──────────┘ └─────────┘ └────────────┘
```

### 4.2 Crate 划分

项目使用 Cargo workspace。

```text
rustfox/
├── Cargo.toml
├── rust-toolchain.toml
├── README.md
├── docs/
│   ├── SPEC.md
│   └── PROGRESS.md
├── crates/
│   ├── fox-core/
│   ├── fox-storage/
│   ├── fox-http/
│   ├── fox-openapi/
│   ├── fox-mock/
│   ├── fox-test/
│   └── fox-desktop/
```

### 4.3 Crate 职责

| Crate | 职责 |
|---|---|
| fox-core | 领域模型、错误、变量引擎、通用工具 |
| fox-storage | SQLite 存储、迁移、Repository |
| fox-http | HTTP 请求构建、发送、响应解析 |
| fox-openapi | OpenAPI 导入导出 |
| fox-mock | Mock Server |
| fox-test | 测试运行器、断言、变量提取 |
| fox-desktop | Dioxus 桌面 UI |

---

## 5. 目录结构

最终必须形成如下结构：

```text
rustfox/
├── Cargo.toml
├── rust-toolchain.toml
├── README.md
├── docs/
│   ├── SPEC.md
│   └── PROGRESS.md
├── crates/
│   ├── fox-core/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── error.rs
│   │       ├── model.rs
│   │       ├── variable.rs
│   │       └── util.rs
│   ├── fox-storage/
│   │   ├── Cargo.toml
│   │   ├── migrations/
│   │   │   └── 0001_init.sql
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── db.rs
│   │       └── repository.rs
│   ├── fox-http/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       └── client.rs
│   ├── fox-openapi/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── import.rs
│   │       └── export.rs
│   ├── fox-mock/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       └── server.rs
│   ├── fox-test/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       └── runner.rs
│   └── fox-desktop/
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs
│           ├── app.rs
│           ├── state.rs
│           ├── services.rs
│           ├── components/
│           ├── pages/
│           └── styles.rs
```

---

## 6. Cargo Workspace 配置

根目录 `Cargo.toml` 必须如下：

```toml
[workspace]
resolver = "2"
members = [
    "crates/fox-core",
    "crates/fox-storage",
    "crates/fox-http",
    "crates/fox-openapi",
    "crates/fox-mock",
    "crates/fox-test",
    "crates/fox-desktop",
]

[workspace.package]
version = "0.1.0"
edition = "2021"
rust-version = "1.79"

[workspace.dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
serde_yaml = "0.9"
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
thiserror = "1"
anyhow = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
tokio = { version = "1", features = ["full"] }
tokio-util = { version = "0.7", features = ["rt"] }
futures = "0.3"
async-trait = "1"
regex = "1"
url = "2"
mime = "0.3"
bytes = "1"
indexmap = { version = "2", features = ["serde"] }
dirs = "5"

sqlx = { version = "0.7", features = [
    "runtime-tokio-rustls",
    "sqlite",
    "uuid",
    "chrono",
    "json",
    "migrate",
] }

reqwest = { version = "0.12", default-features = false, features = [
    "json",
    "stream",
    "cookies",
    "multipart",
    "rustls-tls",
] }

axum = "0.7"
tower = "0.4"
tower-http = { version = "0.5", features = ["cors", "trace"] }

openapiv3 = "2.0"
jsonpath-rust = "0.5"
fake = "2.9"
rand = "0.8"
rfd = "0.14"

dioxus = { version = "0.5", features = ["desktop"] }
```

---

## 7. 数据库设计

数据库使用 SQLite。

数据库文件路径：

```text
{SystemDataDir}/RustFox/rustfox.db
```

示例：

```text
Linux:
~/.local/share/RustFox/rustfox.db

macOS:
~/Library/Application Support/RustFox/rustfox.db

Windows:
C:\Users\{User}\AppData\Roaming\RustFox\rustfox.db
```

---

## 7.1 初始化迁移文件

文件路径：

```text
crates/fox-storage/migrations/0001_init.sql
```

内容必须如下：

```sql
CREATE TABLE IF NOT EXISTS projects (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    variables_json TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS folders (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    parent_id TEXT NULL REFERENCES folders(id) ON DELETE SET NULL,
    name TEXT NOT NULL,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS endpoints (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    folder_id TEXT NULL REFERENCES folders(id) ON DELETE SET NULL,
    name TEXT NOT NULL,
    method TEXT NOT NULL,
    path TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'developing',
    sort_order INTEGER NOT NULL DEFAULT 0,
    request_json TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS response_examples (
    id TEXT PRIMARY KEY,
    endpoint_id TEXT NOT NULL REFERENCES endpoints(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    status INTEGER NOT NULL DEFAULT 200,
    headers_json TEXT NOT NULL DEFAULT '{}',
    body TEXT NOT NULL DEFAULT '',
    content_type TEXT NOT NULL DEFAULT 'application/json',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS environments (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    variables_json TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS mock_rules (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    endpoint_id TEXT NULL REFERENCES endpoints(id) ON DELETE SET NULL,
    name TEXT NOT NULL,
    method TEXT NOT NULL,
    path TEXT NOT NULL,
    match_query_json TEXT NOT NULL DEFAULT '[]',
    match_headers_json TEXT NOT NULL DEFAULT '[]',
    response_status INTEGER NOT NULL DEFAULT 200,
    response_headers_json TEXT NOT NULL DEFAULT '{}',
    response_body_template TEXT NOT NULL DEFAULT '',
    delay_ms INTEGER NOT NULL DEFAULT 0,
    enabled INTEGER NOT NULL DEFAULT 1,
    priority INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS test_runs (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    environment_id TEXT NULL REFERENCES environments(id) ON DELETE SET NULL,
    name TEXT NOT NULL,
    result_json TEXT NOT NULL DEFAULT '{}',
    started_at TEXT NOT NULL,
    finished_at TEXT NULL
);

CREATE TABLE IF NOT EXISTS request_histories (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    endpoint_id TEXT NULL REFERENCES endpoints(id) ON DELETE SET NULL,
    method TEXT NOT NULL,
    url TEXT NOT NULL,
    status INTEGER NULL,
    duration_ms INTEGER NULL,
    request_summary_json TEXT NOT NULL DEFAULT '{}',
    response_summary_json TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value_json TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_folders_project ON folders(project_id);
CREATE INDEX IF NOT EXISTS idx_endpoints_project ON endpoints(project_id);
CREATE INDEX IF NOT EXISTS idx_endpoints_folder ON endpoints(folder_id);
CREATE INDEX IF NOT EXISTS idx_endpoints_method_path ON endpoints(method, path);
CREATE INDEX IF NOT EXISTS idx_environments_project ON environments(project_id);
CREATE INDEX IF NOT EXISTS idx_mock_rules_project ON mock_rules(project_id);
CREATE INDEX IF NOT EXISTS idx_histories_project ON request_histories(project_id);
```

---

## 8. 领域模型

### 8.1 主键与时间

1. 所有 ID 使用 UUID v4，存储为 TEXT。
2. 所有时间使用 UTC RFC3339 字符串。
3. 所有 JSON 字段存储为 TEXT。

### 8.2 核心模型

```rust
pub struct Project {
    pub id: uuid::Uuid,
    pub name: String,
    pub description: String,
    pub variables: std::collections::HashMap<String, String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

pub struct Folder {
    pub id: uuid::Uuid,
    pub project_id: uuid::Uuid,
    pub parent_id: Option<uuid::Uuid>,
    pub name: String,
    pub sort_order: i64,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

pub struct Endpoint {
    pub id: uuid::Uuid,
    pub project_id: uuid::Uuid,
    pub folder_id: Option<uuid::Uuid>,
    pub name: String,
    pub method: HttpMethod,
    pub path: String,
    pub description: String,
    pub status: EndpointStatus,
    pub sort_order: i64,
    pub request: RequestSpec,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
```

### 8.3 HTTP 方法

```rust
pub enum HttpMethod {
    GET,
    POST,
    PUT,
    DELETE,
    PATCH,
    HEAD,
    OPTIONS,
}
```

### 8.4 接口状态

```rust
pub enum EndpointStatus {
    Designing,
    Developing,
    Testing,
    Released,
    Deprecated,
}
```

数据库存储字符串：

```text
designing
developing
testing
released
deprecated
```

---

## 9. 请求结构 RequestSpec

`endpoints.request_json` 使用统一结构。

### 9.1 JSON 结构

```json
{
  "params": [
    {
      "key": "page",
      "value": "1",
      "enabled": true,
      "description": "页码"
    }
  ],
  "headers": [
    {
      "key": "Authorization",
      "value": "Bearer {{token}}",
      "enabled": true,
      "description": ""
    }
  ],
  "path_variables": [
    {
      "key": "id",
      "value": "1",
      "description": ""
    }
  ],
  "auth": {
    "type": "none"
  },
  "body": {
    "mode": "none"
  },
  "timeout_ms": 30000,
  "follow_redirects": true
}
```

### 9.2 Rust 类型

```rust
pub struct RequestSpec {
    pub params: Vec<KeyValue>,
    pub headers: Vec<KeyValue>,
    pub path_variables: Vec<KeyValue>,
    pub auth: AuthSpec,
    pub body: BodySpec,
    pub timeout_ms: u64,
    pub follow_redirects: bool,
}

pub struct KeyValue {
    pub key: String,
    pub value: String,
    pub enabled: bool,
    pub description: String,
}
```

---

## 10. 认证模型

第一阶段支持：

1. none
2. bearer
3. basic
4. apikey

### 10.1 JSON 示例

```json
{
  "type": "bearer",
  "token": "{{token}}"
}
```

```json
{
  "type": "basic",
  "username": "admin",
  "password": "123456"
}
```

```json
{
  "type": "apikey",
  "key": "X-API-KEY",
  "value": "{{api_key}}",
  "in": "header"
}
```

### 10.2 Rust 类型

```rust
pub enum AuthSpec {
    None,
    Bearer { token: String },
    Basic { username: String, password: String },
    ApiKey { key: String, value: String, location: ApiKeyLocation },
}

pub enum ApiKeyLocation {
    Header,
    Query,
}
```

---

## 11. Body 模型

第一阶段支持：

1. none
2. json
3. text
4. urlencoded
5. multipart

### 11.1 Rust 类型

```rust
pub enum BodySpec {
    None,
    Json { raw: String },
    Text { raw: String },
    UrlEncoded { fields: Vec<KeyValue> },
    Multipart { fields: Vec<MultipartField> },
}

pub struct MultipartField {
    pub key: String,
    pub value_type: MultipartValueType,
    pub value: String,
    pub enabled: bool,
}

pub enum MultipartValueType {
    Text,
    FilePath,
}
```

---

## 12. 环境模型

### 12.1 环境变量

环境变量是键值对。

```json
{
  "base_url": "https://api.example.com",
  "token": "abc"
}
```

### 12.2 变量解析优先级

变量解析顺序必须如下：

```text
运行时变量 > 环境变量 > 项目变量 > 内置变量
```

### 12.3 变量语法

```text
{{name}}
```

支持嵌套变量，但最多递归 10 层。

示例：

```text
{{base_url}}/users/{{user_id}}
```

### 12.4 内置变量

| 变量 | 含义 |
|---|---|
| `{{$uuid}}` | UUID v4 |
| `{{$timestamp}}` | 当前秒级时间戳 |
| `{{$isoTimestamp}}` | ISO 8601 当前时间 |
| `{{$randomInt}}` | 0 到 1000 随机整数 |

---

## 13. URL 构建规则

### 13.1 base_url

每个环境建议包含：

```text
base_url
```

### 13.2 拼接规则

1. 如果接口 path 是完整 URL，例如：

```text
https://api.example.com/users
```

则直接使用该 URL。

2. 如果接口 path 是相对路径，例如：

```text
/users/{id}
```

则使用环境变量中的 `base_url` 拼接。

3. Path 变量必须替换。

示例：

```text
base_url = https://api.example.com
path = /users/{id}
path_variables.id = 10
```

最终：

```text
https://api.example.com/users/10
```

---

## 14. HTTP 请求执行规则

### 14.1 请求流程

```text
1. 加载 endpoint
2. 加载环境
3. 合并变量
4. 渲染 URL
5. 渲染 Query
6. 渲染 Headers
7. 渲染 Auth
8. 渲染 Body
9. 发送请求
10. 记录耗时
11. 保存历史
12. 返回响应
```

### 14.2 响应结构

```rust
pub struct HttpResponseData {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: bytes::Bytes,
    pub duration_ms: u64,
    pub size_bytes: usize,
    pub cookies: Vec<CookieData>,
}

pub struct CookieData {
    pub name: String,
    pub value: String,
}
```

### 14.3 限制

1. 默认超时 30 秒。
2. 最大响应体保存 20MB。
3. 超过 20MB 时只保存前 20MB，并标记 `truncated = true`。
4. 请求必须支持取消。
5. UI 不允许阻塞。

---

## 15. OpenAPI 导入导出

### 15.1 导入范围

第一阶段必须支持：

1. OpenAPI 3.0 JSON
2. OpenAPI 3.0 YAML

可选支持：

```text
OpenAPI 3.1
Swagger 2.0
```

如果第一阶段无法支持 3.1，必须明确报错。

### 15.2 导入映射

| OpenAPI | RustFox |
|---|---|
| info.title | 项目名称或导入目录名 |
| paths | endpoints |
| path item method | endpoint method |
| parameters query | params |
| parameters header | headers |
| parameters path | path_variables |
| requestBody application/json | BodySpec::Json |
| requestBody x-www-form-urlencoded | BodySpec::UrlEncoded |
| responses | response_examples |

### 15.3 导入策略

导入时必须提供三种冲突策略：

```text
skip
overwrite
duplicate
```

默认：

```text
skip
```

判断重复条件：

```text
同一项目内 method + path 相同
```

### 15.4 导出规则

导出 OpenAPI JSON 时：

1. 每个 endpoint 转成 path item。
2. request params 转 parameters。
3. JSON body 转 requestBody。
4. response_examples 转 responses。
5. 如果没有 response example，默认生成 200 空响应。

导出文件示例：

```json
{
  "openapi": "3.0.3",
  "info": {
    "title": "Project Name",
    "version": "1.0.0"
  },
  "paths": {}
}
```

---

## 16. Mock Server 规则

### 16.1 Mock Server 基础要求

1. 使用 axum 实现。
2. 默认绑定：

```text
127.0.0.1
```

3. 默认端口：

```text
4010
```

4. 如果端口被占用，自动尝试：

```text
4011
4012
4013
...
```

最多尝试 20 次。

5. Mock Server 必须支持启动、停止、重启。

---

### 16.2 Mock 匹配规则

优先级：

```text
自定义 MockRule > Endpoint ResponseExample > 默认 JSON
```

### 16.3 路径匹配

支持路径参数：

```text
/users/{id}
```

示例：

```text
GET /users/1
```

匹配：

```text
/users/{id}
```

提取：

```json
{
  "id": "1"
}
```

### 16.4 Mock 模板变量

Mock Body 支持以下模板：

```text
{{params.id}}
{{query.name}}
{{headers.X-Token}}
{{mock.uuid}}
{{mock.email}}
{{mock.name}}
{{mock.word}}
{{mock.timestamp}}
{{mock.int}}
```

示例：

```json
{
  "id": "{{params.id}}",
  "email": "{{mock.email}}",
  "name": "{{mock.name}}",
  "createdAt": "{{mock.timestamp}}"
}
```

---

## 17. 自动化测试规则

### 17.1 测试对象

第一阶段支持：

1. 单个接口测试
2. 文件夹测试
3. 整个项目测试

### 17.2 测试运行流程

```text
1. 选择项目
2. 选择环境
3. 收集 endpoints
4. 按目录排序执行
5. 每个请求前执行变量设置
6. 发送请求
7. 提取变量
8. 执行断言
9. 记录结果
```

### 17.3 测试配置

测试配置存储在 endpoint 的 request_json 中：

```json
{
  "tests": {
    "pre_request": [
      {
        "type": "set_variable",
        "name": "timestamp",
        "value": "{{$timestamp}}"
      }
    ],
    "extract": [
      {
        "name": "userId",
        "from": "body",
        "path": "$.id"
      }
    ],
    "assertions": [
      {
        "type": "status",
        "op": "eq",
        "expected": 200
      },
      {
        "type": "jsonpath",
        "path": "$.name",
        "op": "contains",
        "expected": "test"
      },
      {
        "type": "response_time_ms",
        "op": "lt",
        "expected": 2000
      }
    ]
  }
}
```

### 17.4 断言类型

| type | 说明 |
|---|---|
| status | HTTP 状态码 |
| header | Header 断言 |
| body | Body 文本断言 |
| jsonpath | JSONPath 断言 |
| response_time_ms | 响应时间断言 |

### 17.5 操作符

```text
eq
neq
contains
not_contains
gt
gte
lt
lte
exists
not_exists
```

---

## 18. UI 规范

### 18.1 总体布局

应用采用三栏布局：

```text
┌──────────────────────────────────────────────────────┐
│ TopBar                                               │
├──────────────┬───────────────────────────────────────┤
│ Sidebar      │ MainArea                              │
│              │                                       │
│ ProjectTree  │ RequestEditor                         │
│              │                                       │
│              ├───────────────────────────────────────┤
│              │ ResponseViewer                        │
└──────────────┴───────────────────────────────────────┘
```

### 18.2 顶部栏

顶部栏包含：

1. 项目选择器
2. 环境选择器
3. Mock Server 状态
4. 搜索框
5. 设置入口

### 18.3 左侧树

左侧树显示：

```text
项目
├── 文件夹
│   ├── 文件夹
│   └── 接口
└── 接口
```

支持：

1. 新建文件夹
2. 新建接口
3. 删除
4. 重命名
5. 复制
6. 搜索过滤

第一阶段不强制支持拖拽排序。

### 18.4 主编辑区

主编辑区包含：

1. Method 选择
2. URL 输入
3. Send 按钮
4. Save 按钮
5. Cancel 按钮
6. Tabs

Tabs：

```text
Params
Headers
Body
Auth
Tests
Docs
```

### 18.5 Params Tab

提供一个键值表：

```text
Key | Value | Description | Enabled
```

### 18.6 Headers Tab

同上。

### 18.7 Body Tab

Body 类型选择：

```text
none
json
text
x-www-form-urlencoded
multipart/form-data
```

JSON Body 使用等宽字体 textarea。

必须提供：

```text
Format JSON
```

### 18.8 Auth Tab

支持：

```text
none
bearer
basic
apikey
```

### 18.9 Tests Tab

支持编辑：

1. pre_request
2. extract
3. assertions

第一阶段可以使用 JSON 编辑器形式。

### 18.10 Docs Tab

显示：

1. 接口名称
2. Method
3. Path
4. 描述
5. 参数表
6. Body 示例
7. Response 示例

---

## 19. 响应展示区

### 19.1 必须显示

1. HTTP Status
2. 响应时间
3. 响应大小
4. Headers
5. Cookies
6. Body

### 19.2 Body 展示

如果 Content-Type 是 JSON：

```text
自动格式化
```

如果是 Text：

```text
普通文本
```

如果是二进制：

```text
显示大小
提供保存按钮
```

### 19.3 响应操作

必须支持：

1. 复制响应
2. 保存为示例
3. 下载 Body

---

## 20. 页面与导航

由于第一阶段是桌面应用，不强制使用 URL route。

内部导航状态：

```rust
pub enum Page {
    Home,
    Project(uuid::Uuid),
    Endpoint(uuid::Uuid),
    Environments(uuid::Uuid),
    Mock(uuid::Uuid),
    TestRunner(uuid::Uuid),
    Settings,
}
```

---

## 21. 状态管理

### 21.1 全局状态

```rust
pub struct AppState {
    pub current_project_id: Option<uuid::Uuid>,
    pub current_environment_id: Option<uuid::Uuid>,
    pub current_page: Page,
    pub open_tabs: Vec<uuid::Uuid>,
    pub active_endpoint_id: Option<uuid::Uuid>,
}
```

### 21.2 异步规则

1. 所有数据库操作必须异步。
2. 所有 HTTP 请求必须异步。
3. UI 不得阻塞。
4. 使用 Dioxus 的 `spawn` 执行异步任务。
5. 使用 Signal 管理 UI 状态。

---

## 22. 服务接口定义

### 22.1 Services

```rust
#[derive(Clone)]
pub struct Services {
    pub db: sqlx::SqlitePool,
}
```

### 22.2 ProjectService

```rust
pub async fn create_project(db: &SqlitePool, name: &str, description: &str) -> Result<Project>;
pub async fn list_projects(db: &SqlitePool) -> Result<Vec<Project>>;
pub async fn delete_project(db: &SqlitePool, project_id: uuid::Uuid) -> Result<()>;
```

### 22.3 FolderService

```rust
pub async fn create_folder(db: &SqlitePool, project_id: uuid::Uuid, parent_id: Option<uuid::Uuid>, name: &str) -> Result<Folder>;
pub async fn list_folders(db: &SqlitePool, project_id: uuid::Uuid) -> Result<Vec<Folder>>;
pub async fn delete_folder(db: &SqlitePool, folder_id: uuid::Uuid) -> Result<()>;
```

### 22.4 EndpointService

```rust
pub async fn create_endpoint(db: &SqlitePool, project_id: uuid::Uuid, folder_id: Option<uuid::Uuid>, name: &str) -> Result<Endpoint>;
pub async fn get_endpoint(db: &SqlitePool, endpoint_id: uuid::Uuid) -> Result<Endpoint>;
pub async fn update_endpoint(db: &SqlitePool, endpoint: &Endpoint) -> Result<Endpoint>;
pub async fn delete_endpoint(db: &SqlitePool, endpoint_id: uuid::Uuid) -> Result<()>;
pub async fn duplicate_endpoint(db: &SqlitePool, endpoint_id: uuid::Uuid) -> Result<Endpoint>;
pub async fn list_endpoints(db: &SqlitePool, project_id: uuid::Uuid) -> Result<Vec<Endpoint>>;
```

### 22.5 EnvironmentService

```rust
pub async fn create_environment(db: &SqlitePool, project_id: uuid::Uuid, name: &str) -> Result<Environment>;
pub async fn update_environment(db: &SqlitePool, environment: &Environment) -> Result<Environment>;
pub async fn delete_environment(db: &SqlitePool, environment_id: uuid::Uuid) -> Result<()>;
pub async fn list_environments(db: &SqlitePool, project_id: uuid::Uuid) -> Result<Vec<Environment>>;
```

### 22.6 HttpService

```rust
pub async fn send_endpoint_request(
    db: &SqlitePool,
    project_id: uuid::Uuid,
    endpoint_id: uuid::Uuid,
    environment_id: Option<uuid::Uuid>,
) -> Result<HttpResponseData>;
```

### 22.7 MockService

```rust
pub async fn start_mock_server(db: SqlitePool, project_id: uuid::Uuid, port: u16) -> Result<MockServerHandle>;
pub async fn stop_mock_server(handle: &MockServerHandle) -> Result<()>;
```

### 22.8 TestService

```rust
pub async fn run_project_tests(
    db: &SqlitePool,
    project_id: uuid::Uuid,
    environment_id: Option<uuid::Uuid>,
) -> Result<TestRunResult>;
```

---

## 23. 错误处理

### 23.1 统一错误类型

```rust
#[derive(thiserror::Error, Debug)]
pub enum AppError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("validation error: {0}")]
    Validation(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("openapi error: {0}")]
    OpenApi(String),

    #[error("mock error: {0}")]
    Mock(String),

    #[error("test error: {0}")]
    Test(String),
}
```

### 23.2 UI 错误提示

所有用户操作失败必须显示 Toast。

Toast 内容使用中文。

示例：

```text
创建项目失败
发送请求失败
Mock 服务启动失败
OpenAPI 导入失败
```

---

## 24. 日志规范

使用 `tracing`。

默认日志级别：

```text
info
```

支持环境变量：

```bash
RUST_LOG=debug
```

日志输出：

```text
stdout
{DataDir}/RustFox/logs/rustfox.log
```

---

## 25. 样式规范

第一阶段使用内置 CSS。

主题：深色。

颜色建议：

```css
:root {
  --bg: #0f172a;
  --panel: #111827;
  --border: #1f2937;
  --text: #e5e7eb;
  --muted: #9ca3af;
  --accent: #2563eb;
  --success: #22c55e;
  --warning: #f59e0b;
  --danger: #ef4444;
}
```

布局要求：

```text
TopBar 高度 48px
Sidebar 宽度 280px
Response 区域高度 45%
```

---

## 26. 快捷键

第一阶段必须支持：

| 快捷键 | 功能 |
|---|---|
| Ctrl/Cmd + Enter | 发送请求 |
| Ctrl/Cmd + S | 保存接口 |
| Ctrl/Cmd + N | 新建接口 |
| Ctrl/Cmd + F | 搜索接口 |

---

## 27. 里程碑计划

必须按以下顺序执行。

---

# M0：仓库初始化

## 目标

创建 Cargo workspace，确保空项目可以编译。

## 任务

1. 创建根目录 `Cargo.toml`
2. 创建 `rust-toolchain.toml`
3. 创建所有 crate
4. 配置 workspace dependencies
5. 创建 `docs/PROGRESS.md`
6. 确保：

```bash
cargo build --workspace
```

## 验收标准

```bash
cargo build --workspace
```

必须成功。

---

# M1：核心模型与数据库

## 目标

完成领域模型、SQLite 存储、基础 Repository。

## 任务

1. 实现 `fox-core` 模型
2. 实现 `AppError`
3. 实现变量引擎
4. 实现 `fox-storage`
5. 添加 migration
6. 实现 Project / Folder / Endpoint / Environment Repository
7. 编写单元测试

## 必须测试

1. 创建项目
2. 查询项目
3. 删除项目
4. 创建文件夹
5. 创建接口
6. 更新接口
7. 删除接口
8. 环境 CRUD
9. 变量替换

## 验收标准

```bash
cargo test -p fox-core
cargo test -p fox-storage
```

必须通过。

---

# M2：桌面应用骨架

## 目标

启动 Dioxus 桌面应用。

## 任务

1. 初始化日志
2. 创建数据目录
3. 初始化 SQLite
4. 创建 Dioxus App
5. 渲染基础布局
6. 加载项目列表
7. 支持创建项目
8. 支持切换项目

## 验收标准

运行：

```bash
cargo run -p fox-desktop
```

必须显示应用窗口。

可以：

1. 创建项目
2. 选择项目
3. 删除项目

---

# M3：目录树与接口管理

## 目标

完成左侧目录树与接口 CRUD。

## 任务

1. 显示文件夹树
2. 显示接口列表
3. 新建文件夹
4. 新建接口
5. 删除接口
6. 复制接口
7. 重命名接口
8. 搜索接口
9. 打开接口 Tab

## 验收标准

1. 可以创建文件夹
2. 可以创建接口
3. 可以打开接口
4. 可以删除接口
5. 可以复制接口
6. 搜索可以过滤接口

---

# M4：接口编辑器

## 目标

完成接口编辑区。

## 任务

1. Method 选择
2. Path 编辑
3. Name 编辑
4. Description 编辑
5. Params 编辑
6. Headers 编辑
7. Body 编辑
8. Auth 编辑
9. Save 保存
10. JSON Format

## 验收标准

1. 编辑后可以保存
2. 重新打开数据仍然存在
3. JSON Format 可格式化合法 JSON
4. 非法 JSON 给出提示

---

# M5：HTTP 调试

## 目标

可以发送真实 HTTP 请求。

## 任务

1. 实现 `fox-http`
2. 支持 GET/POST/PUT/DELETE/PATCH
3. 支持 Query
4. 支持 Headers
5. 支持 JSON Body
6. 支持 Text Body
7. 支持 UrlEncoded
8. 支持 Bearer / Basic / ApiKey
9. 支持超时
10. 支持取消
11. 保存请求历史
12. 展示响应

## 验收标准

1. 可以向公开测试 API 发送请求
2. 响应状态、Header、Body 正常显示
3. JSON 自动格式化
4. 可以取消请求
5. 请求历史可查看

---

# M6：环境与变量

## 目标

完成环境管理。

## 任务

1. 环境列表
2. 新建环境
3. 删除环境
4. 编辑环境变量
5. 选择当前环境
6. 请求发送时使用变量
7. 支持项目变量
8. 支持内置变量

## 验收标准

1. 切换环境后请求 URL 随之变化
2. `{{base_url}}` 可正确替换
3. `{{$uuid}}` 可生成
4. 变量嵌套最多递归 10 层

---

# M7：OpenAPI 导入导出

## 目标

支持 OpenAPI 3.0。

## 任务

1. 实现导入 JSON/YAML
2. 将 paths 转成 endpoints
3. 将 parameters 转成 request 数据
4. 将 requestBody 转成 body
5. 将 responses 转成 examples
6. 导出 OpenAPI JSON
7. 导入冲突策略

## 验收标准

1. 导入合法 OpenAPI 文件成功
2. 导入后 endpoints 可见
3. 导出 JSON 可被再次导入
4. 非法文件给出中文错误提示

---

# M8：Mock Server

## 目标

完成本地 Mock。

## 任务

1. 启动 Mock Server
2. 停止 Mock Server
3. 显示 Mock 地址
4. 根据 endpoint 自动 Mock
5. 支持 response example
6. 支持自定义 mock rules
7. 支持路径参数
8. 支持 query/header 匹配
9. 支持模板变量
10. 支持延迟返回

## 验收标准

启动 Mock 后：

```bash
curl http://127.0.0.1:4010/your/path
```

必须返回 Mock 数据。

---

# M9：自动化测试

## 目标

完成测试运行器。

## 任务

1. 单接口测试
2. 文件夹测试
3. 项目测试
4. 断言
5. 变量提取
6. 测试结果展示
7. 测试结果入库
8. 失败高亮

## 验收标准

1. 可以运行整个项目测试
2. 可以看到每个接口通过/失败
3. 可以查看断言失败原因
4. 变量提取可在后续请求中使用

---

# M10：文档与备份

## 目标

完成文档导出和本地备份。

## 任务

1. Docs Tab
2. Markdown 导出
3. OpenAPI JSON 导出
4. 项目 JSON 备份
5. JSON 恢复
6. 示例 response 管理

## 验收标准

1. 可导出 Markdown
2. 可导出 OpenAPI JSON
3. 可备份项目
4. 可恢复项目
5. 恢复后数据一致

---

## 28. 测试要求

### 28.1 单元测试

必须覆盖：

1. 变量替换
2. URL 拼接
3. Auth 构建
4. Body 构建
5. Mock 路径匹配
6. Mock 模板渲染
7. OpenAPI 转换
8. 断言执行
9. JSONPath 提取

### 28.2 集成测试

必须覆盖：

1. SQLite CRUD
2. HTTP Client 请求本地测试服务
3. Mock Server 启动和访问
4. OpenAPI 导入导出闭环
5. Test Runner 执行闭环

### 28.3 测试命令

```bash
cargo test --workspace
```

必须全部通过。

---

## 29. 手动验收脚本

AI Coder 完成全部功能后，必须按以下流程验收。

### 29.1 启动应用

```bash
cargo run -p fox-desktop
```

### 29.2 创建项目

创建一个项目：

```text
Demo API
```

### 29.3 创建环境

创建环境：

```text
local
```

变量：

```text
base_url = https://jsonplaceholder.typicode.com
```

### 29.4 创建接口

创建接口：

```text
GET /todos/{{id}}
```

Path 变量：

```text
id = 1
```

发送请求。

预期：

```text
HTTP 200
返回 JSON
```

### 29.5 导入 OpenAPI

导入最小 OpenAPI：

```json
{
  "openapi": "3.0.0",
  "info": {
    "title": "Demo",
    "version": "1.0.0"
  },
  "paths": {
    "/ping": {
      "get": {
        "responses": {
          "200": {
            "description": "ok"
          }
        }
      }
    }
  }
}
```

预期：

```text
生成 GET /ping
```

### 29.6 启动 Mock

启动 Mock。

访问：

```bash
curl http://127.0.0.1:4010/ping
```

预期：

```text
返回 Mock 响应
```

### 29.7 运行测试

创建一个断言：

```json
{
  "assertions": [
    {
      "type": "status",
      "op": "eq",
      "expected": 200
    }
  ]
}
```

运行测试。

预期：

```text
测试通过
```

---

## 30. Definition of Done

项目完成的定义如下：

1. 所有里程碑完成。
2. 以下命令全部通过：

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p fox-desktop
```

3. 应用可以完成以下闭环：

```text
创建项目
创建环境
创建接口
发送请求
保存响应示例
启动 Mock
访问 Mock
运行测试
导出 OpenAPI
导出 Markdown
备份项目
恢复项目
```

4. 所有用户错误必须有中文提示。
5. 不允许出现未处理 panic。
6. `docs/PROGRESS.md` 已更新。

---

## 31. 风险与替代方案

### 31.1 Dioxus 复杂组件风险

如果 Dioxus 某些复杂组件实现困难，允许降级：

1. 使用 textarea 替代代码编辑器。
2. 使用简单 table 替代复杂虚拟表格。
3. 使用 JSON 编辑替代可视化脚本编辑。

但不允许引入 React/Vue。

### 31.2 SQLite 依赖风险

如果系统 SQLite 不可用，在 `fox-storage` 中加入 bundled sqlite 支持。

可添加：

```toml
libsqlite3-sys = { version = "0.28", features = ["bundled"] }
```

如版本冲突，调整到与 sqlx 兼容的版本。

### 31.3 Linux 依赖

Linux 桌面运行 Dioxus 需要 webkit2gtk。

Ubuntu/Debian：

```bash
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file \
  libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

如果无法安装，允许先使用单元测试和 headless 测试验证逻辑。

---

## 32. 给 AI Coder 的阶段性 Prompt

你可以把下面的 Prompt 分阶段喂给 AI Coder。

### 32.1 初始化 Prompt

```text
请阅读 docs/SPEC.md，并执行 M0。
要求：
1. 创建 Cargo workspace
2. 创建所有 crates
3. 配置 workspace dependencies
4. 创建 rust-toolchain.toml
5. 创建 docs/PROGRESS.md
6. 保证 cargo build --workspace 成功
不要实现业务逻辑。
```

### 32.2 M1 Prompt

```text
请阅读 docs/SPEC.md，并执行 M1。
要求：
1. 实现 fox-core 领域模型
2. 实现 AppError
3. 实现变量引擎
4. 实现 fox-storage SQLite 迁移和 Repository
5. 编写单元测试
6. 更新 docs/PROGRESS.md
必须通过 cargo test -p fox-core 和 cargo test -p fox-storage。
```

### 32.3 M2 Prompt

```text
请阅读 docs/SPEC.md，并执行 M2。
要求：
1. 初始化日志
2. 初始化 SQLite
3. 创建 Dioxus 桌面应用
4. 实现基础布局
5. 实现项目创建、列表、删除、切换
6. 更新 docs/PROGRESS.md
必须保证 cargo run -p fox-desktop 可以启动。
```

### 32.4 后续通用 Prompt

```text
请阅读 docs/SPEC.md，并执行下一个里程碑。
要求：
1. 严格按照 SPEC 中当前里程碑的任务实现
2. 不要跳过测试
3. 不要修改已完成里程碑的验收标准
4. 完成后运行 cargo fmt、cargo clippy、cargo test
5. 更新 docs/PROGRESS.md
```

---

## 33. 后续版本路线

第一阶段完成后，可以进入 V2。

### V2 建议功能

1. 用户登录
2. 团队协作
3. 云端同步
4. 接口版本历史
5. 接口变更对比
6. 权限管理
7. Postman 导入
8. Swagger 2.0 导入
9. WebSocket 调试
10. GraphQL 调试
11. 脚本系统
12. 定时任务
13. CLI 工具
14. API 分享链接

### V2 推荐服务端技术

仍然使用 Rust：

```text
axum
tokio
sqlx
PostgreSQL
serde
jsonwebtoken
```

---

## 34. 最终交付物

项目完成后必须交付：

1. 完整源码
2. Cargo workspace
3. SQLite migrations
4. README
5. docs/SPEC.md
6. docs/PROGRESS.md
7. 单元测试
8. 集成测试
9. 示例 OpenAPI 文件
10. 手动验收脚本

---

# 结束

本文档就是 RustFox 第一阶段的唯一执行标准。
AI Coder 必须按照本文档从 M0 到 M10 顺序实现，不得跳阶段。
每个阶段完成后必须通过格式化、clippy、测试，并更新进度文档。
