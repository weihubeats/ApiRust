# RustFox 架构

本文档基于**当前代码库**整理（Tauri 2 迁移完成后的架构），是 `docs/SPEC.md` 中旧 Dioxus
架构章节的替代。若代码与本文不符，以代码为准并更新本文。

## 1. 总览

![RustFox 系统架构](imags/architecture.png)

（可编辑源文件：[imags/architecture.svg](imags/architecture.svg)）

三个层次：

| 层 | 形态 | 位置 |
|---|---|---|
| Layer 1 · 前端 | Vue 3 + TypeScript（Vite + Tailwind 4 + Pinia） | `frontend/` |
| Layer 2 · 命令层 | Tauri 2 插件 `fox`（`Builder::new("fox")`，生成 `plugin:fox|*` 命令） | `crates/fox-tauri/`（独立工作区） |
| Layer 3 · 领域层 | 纯库 crate，不依赖任何 UI 框架 | `crates/` 其余目录 |

前端与后端通过 **Tauri IPC**（`invoke('plugin:fox|command', args)`）通信；
领域 crate 之间是**路径依赖直接调用**（Rust 层），不经过 IPC。

## 2. 技术栈（实际依赖清单）

| 模块 | 技术 |
|---|---|
| 桌面壳 | Tauri 2（`tauri` crate + `@tauri-apps/api` v2） |
| 前端框架 | Vue 3.5（`<script setup>` 组合式 API）+ TypeScript 5.6 |
| 构建 | Vite 6 + `@vitejs/plugin-vue` + Tailwind CSS 4（`@tailwindcss/vite`） |
| 状态 | Pinia 2.2（单 store：`stores/workspace.ts`） |
| 路由 | vue-router 4（web history，单窗口 SPA） |
| 图表 | chart.js + vue-chartjs（压测结果图） |
| IPC 封装 | `composables/useFoxApi.ts`（类型安全 + 错误映射） |
| 异步运行时 | Tokio（Cargo workspace 统一 `tokio = "1"`） |
| 本地数据库 | SQLite + SQLx（`runtime-tokio-rustls` / `sqlite` / `migrate`） |
| HTTP 客户端 | reqwest 0.12（rustls-tls / cookies / multipart / stream） |
| Mock Server | axum 0.7 + tower / tower-http |
| OpenAPI | openapiv3 2.0（导入导出） |
| 加密 | aes-gcm（fox-secret，环境变量 AES-256-GCM，`master.key`） |
| 断言/模拟 | jsonpath-rust、fake、rand（测试与 Mock 模板变量） |
| 日志 | tracing + tracing-subscriber |
| 文件对话框 | rfd（导入导出文件） |

> 旧文档中「Dioxus Desktop」与「禁止 TypeScript/Vue」的约束**已失效**，详见
> [`docs/TAURI_MIGRATION.md`](TAURI_MIGRATION.md)。

## 3. Workspace 布局

```text
rustfox/
├── Cargo.toml                     # 根 workspace（11 个纯库 crate，不含 tauri）
├── crates/
│   ├── fox-core/                  # 领域模型、错误、变量引擎（{{name}} 解析）
│   ├── fox-storage/               # SQLx 迁移（migrations/）、Repository、db 初始化
│   ├── fox-http/                  # reqwest 请求构建 / 发送 / cURL 解析 / WS 客户端
│   ├── fox-openapi/               # OpenAPI 3.x / Swagger 2.0 导入导出
│   ├── fox-mock/                  # axum Mock Server（端口 4010 起自动探测）
│   ├── fox-test/                  # 自动化测试运行器（断言、变量提取、压测）
│   ├── fox-backup/                # JSON 备份 / 恢复（ID 重映射）
│   ├── fox-secret/                # AES-256-GCM 加密（环境变量值、master.key）
│   ├── fox-codegen/               # 多语言客户端代码生成
│   ├── fox-oauth/                 # OAuth2 四模式（浏览器授权 / 令牌端点）
│   └── fox-smoke/                 # 冒烟测试（启动应用骨架、健康检查）
├── crates/fox-tauri/              # 独立工作区：Tauri 2 插件封装（见 §4）
├── frontend/                      # Vue 3 前端（见 §5）
│   ├── src-tauri/                 # Tauri 应用壳（tauri.conf.json、capabilities、icons）
│   └── dist/                      # vite build 产物（frontendDist 指向 ../dist）
├── scripts/                       # package 脚本（package-tauri.sh 等）
└── docs/                          # 本文档
```

> `crates/fox-tauri` 不加入根 workspace：tauri 依赖较重，避免拖慢主仓构建；其内部通过
> 路径依赖引用 `fox-*`，`[workspace]` 置空以独立解析（见其 `Cargo.toml` 注释）。
> `links = "fox"` 决定权限命名空间前缀，与 `Builder::new("fox")` 必须一致。

## 4. fox-tauri 命令层

- 入口：`crates/fox-tauri/src/lib.rs` 的 `plugin::init()`。
- `setup` 流程：`fox_storage::db::init_db(database_path())`（建目录 + 跑迁移）→
  `app.manage(AppState::new(db))` 托管连接池与激活上下文。
- `AppState`（`state.rs`）：`SqlitePool` + `RwLock` 激活项目/环境 + Mock 运行状态 `Mutex`。
- 命令：`generate_handler!` 注册 40+ 命令，源文件按模块拆分
  （`commands/{project,folder,endpoint,environment,request,history,example,mock,mock_rule,load_test,oauth,codegen,import_export,backup,curl}.rs`）。
- 错误约定：所有命令返回 `Result<T, CommandError>`；`CommandError` 序列化为
  `{ code, message }`（`VALIDATION` / `NOT_FOUND` / `DECRYPT` / `IO` / …），
  前端 `useFoxApi.call()` 统一转换为携带 `code` 的 `Error`。
- 事件推送：如压测进度经 `AppHandle.emit("fox:load-progress", …)` 到前端监听。
- 权限：`frontend/src-tauri/capabilities/` 中声明 `fox:default` 权限集
  （`permissions/` 由 `tauri-plugin` build 依赖生成）。

## 5. 前端结构

```text
frontend/
├── src/
│   ├── main.ts                     # createApp + Pinia + router
│   ├── App.vue                     # 全局错误边界 / 语法高亮 / 主题
│   ├── router/index.ts             # /projects、/workspace、/graphql
│   ├── views/
│   │   ├── ProjectList.vue         # 首页：统计卡 + 项目卡片 + 快速请求
│   │   ├── WorkspaceView.vue       # 工作区：顶栏 + 侧边栏 + 标签 + 编辑器
│   │   └── GraphQLView.vue         # GraphQL 调试视图
│   ├── stores/workspace.ts         # 唯一 Pinia store（项目/环境/标签/目录/历史）
│   ├── composables/
│   │   ├── useFoxApi.ts            # IPC 统一封装（自动加 plugin:fox 前缀）
│   │   ├── useToast.ts / useProgress.ts / useTheme.ts
│   ├── components/                 # EndpointEditor / EndpointTree / ResponsePanel /
│   │   │                           # TabBar / EnvironmentBar / ToolsDrawer / SettingsDialog /
│   │   │                           # Params/Headers/Body/Auth/Tests/Docs 面板 / …
│   │   └── ui/                     # 基础控件库（按钮/输入/下拉/菜单…）
│   └── types/foxApi.d.ts           # 与 Rust 模型对应的命令签名（手工维护的镜像）
└── src-tauri/
    ├── tauri.conf.json             # 1360×900 主窗口（Overlay 标题栏）、devUrl:5173
    ├── capabilities/default.json   # core:default + fox:default
    └── icons/
```

UI 规范：`src/style.css` 定义设计令牌（`--bg-* / --text-* / --accent: #7c69f5`），
组件只引用令牌，支持深色/浅色/跟随系统。

## 6. 界面总览

### 首页（/projects）

![RustFox 首页布局](imags/ui-home.png)

（可编辑源文件：[imags/ui-home.svg](imags/ui-home.svg)）

- 顶栏：RustFox 品牌 + 设置。
- 左侧导航：仪表板（当前页）/ API 项目 / 集合 / API 文档 / 设置（未实现项 Toast 提示）。
- 主区：统计卡片（总 API 数 / 最近活动 / 快速请求）、项目过滤与新建、项目卡片列表
  （进入项目 → `/workspace`）。

### 工作区（/workspace）

![RustFox 工作区布局](imags/ui-workspace.png)

（可编辑源文件：[imags/ui-workspace.svg](imags/ui-workspace.svg)）

- 顶栏：品牌 / 项目切换 / 环境切换 / Mock 状态指示灯（`127.0.0.1:4010`）/ 搜索 /
  文档与 Mock 下拉 / 设置。
- 侧边栏：项目切换 + 接口搜索 + 目录树（文件夹/接口分层，拖放移动、行内重命名、
  新建/删除、右键操作）。
- 标签栏：每接口一个标签，未保存草稿以 `●` 标记。
- 请求编辑区：方法下拉 + URL（含 `{{变量}}`）+ 保存 / 发送 / 生成代码；
  Params / Headers / Body / Auth / Tests / Docs / Scripts 分页。
- 响应面板：状态码 / 耗时 / 大小 / 响应头 / 格式化 JSON 树 / Raw / 下载。

## 7. 关键数据流

### 7.1 发送请求

```text
前端 EndpointEditor（发送）
  → useFoxApi.executeRequest({ spec, environment_id, project_id })
  → IPC invoke('plugin:fox|execute_request')
  → fox-tauri commands::request::execute_request
  → fox-core 变量引擎解析 {{name}}（环境 > 项目 > 全局）
  → fox-http 构建 reqwest 请求（Auth/Body/Headers）
  → 目标服务器
  ← 响应 → fox-http 解析（状态/头/体/耗时/大小）
  ← 同时写入 fox-storage request_histories
  ← 返回 ExecuteResponse → 前端 ResponsePanel 渲染
```

请求可随时取消（`cancel_request`，Abort 令牌）；下载模式走 stream 落盘。

### 7.2 Mock Server

```text
设置页「启动 Mock」
  → mock_start → fox-mock::start（4010 起探测空闲端口）
  → axum 服务路由：
     优先匹配 Mock 规则（method + path + headers + query + body 模板）
     兜底返回接口的「响应示例」
  模板变量：{{params.*}} {{query.*}} {{headers.*}} {{mock.uuid|email|name|word|timestamp|int}}
```

### 7.3 OAuth2

```text
Auth 面板「授权」
  → oauth_authorize：启动本地回调端口，系统浏览器打开授权端点
  → 回调捕获 code → oauth_access_token：换取并存储令牌
  → send 时自动附加 Authorization 头 / cookie
```

## 8. 数据与安全

| 项 | 说明 |
|---|---|
| 数据库 | `{data_dir}/RustFox/rustfox.db`（SQLite 单文件，SQLx 迁移脚本在 `fox-storage/migrations/`） |
| 加密 | 环境变量值 AES-256-GCM，密钥文件 `master.key`；`DECRYPT` 错误码对应密钥不匹配 |
| 备份 | fox-backup 导出 JSON（含明文变量）；恢复时 ID 全量重映射为全新项目 |
| 保存语义 | 所有带 id 的保存路径为 **upsert**（`INSERT … ON CONFLICT(id) DO UPDATE`），重命名/编辑不会触发主键冲突 |

## 9. 文档索引

| 文档 | 内容 |
|---|---|
| [README.md](../README.md) | 产品介绍、下载、构建 |
| [USER_GUIDE.md](USER_GUIDE.md) | 最终用户手册 |
| [SPEC.md](SPEC.md) | 详细开发规范（功能/模型/数据库/命令） |
| [TAURI_MIGRATION.md](TAURI_MIGRATION.md) | Dioxus → Tauri 迁移记录、基线对比 |
| [DEPLOY.md](DEPLOY.md) | 打包部署 |
| [SMOKE_TEST.md](SMOKE_TEST.md) | 手动验收清单 |