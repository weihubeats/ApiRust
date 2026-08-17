# RustFox

> 约 10 MB 的轻量跨平台 API 调试工具。一个安装包，开箱即用。

<!--
  示例图片：docs/imags/home.png（主页）、docs/imags/api-home.png（工作区）。
  更新截图后替换下面两行的路径即可。
-->
[![RustFox 主页截图](docs/imags/home.png)](docs/imags/home.png)

[![RustFox 工作区截图](docs/imags/api-home.png)](docs/imags/api-home.png)

## 为什么选择 RustFox？

### 🪶 10 MB，零运行时依赖

同类工具把整个 Chromium/Node.js 运行时打包进安装包，RustFox 用 **Rust + Tauri 2 + 系统 WebView**——不内置浏览器内核、无 Node 沙箱、无 JRE，装完即用。

| 维度 | RustFox | Postman | Bruno | Insomnia |
| --- | --- | --- | --- | --- |
| 安装包体积 | **~10 MB** | ~310 MB (Electron) | ~433 MB (Electron) | ~200 MB (Electron) |
| 首屏启动 | **< 1 秒** | 2–4 秒 | 2–5 秒 | 2–4 秒 |
| 运行时内存 | **~40 MB** | ~500 MB+ | ~300 MB+ | ~200 MB+ |
| 应用壳 | 系统 WebView（无内置 Chromium） | Chromium + Node.js | Chromium + Node.js | Chromium + Node.js |

同等工作负载下，安装包体积缩小 **20–40 倍**、启动快 **2–5 倍**、占用内存减少一个数量级。

### ⚡ 快，但功能不缺

Rust LTO 优化 + 单一进程模型 + SQLite 零拷贝本地存储——秒开、秒搜、秒发；请求 / Mock / 测试 / 压测全部内置，不依赖云端服务。

### 🔒 本地优先，数据在自己手里

数据只存在本机 `rustfox.db`，环境变量值 **AES-256-GCM 加密**存储；一键备份为 JSON，随时可恢复。

## 功能

### 请求与编辑

- 8 种 HTTP 方法、6 种请求体（JSON / Form / x-www-form-urlencoded / Multipart / GraphQL / Text）
- Params / Headers / Body / Auth / Tests / Docs 分页编辑，未保存草稿自动标记
- cURL 一键粘贴导入，自动识别方法 / URL / Header / Body / Basic Auth
- 环境变量 `{{name}}` 任意位置自动解析（环境 > 项目 优先级）

### 认证与安全

- API Key / Basic / Bearer / OAuth2（Authorization Code / Client Credentials / Password / Implicit 四模式）
- 令牌自动附加请求头，无需手写

### 响应体验

- 格式化 JSON 树 / Raw / 响应头 / 状态码 / 耗时 / 大小
- 流式下载保存到本地，请求历史可重发、可删除

### Mock Server

- 本地 axum Mock（端口 4010 起自动探测），无需联网
- **Mock 规则**（方法 + 路径 + Header + Body 匹配）优先，接口「响应示例」兜底
- 模板变量：`{{params.id}}` `{{headers.X-Token}}` `{{mock.uuid|email|name|word|int}}`

### 自动化测试与压测

- JSON 测试脚本：`pre_request` 注入变量、`extract` 提取传递、`assertions` 断言
- 单接口 / 文件夹 / 全项目一键运行，结果与历史留存
- 压测：并发 × 总请求数，输出 QPS、平均耗时、P50/P90/P99、错误样例，chart.js 图表

### 导入导出与协作

- OpenAPI 3.x / Swagger 2.0 / Postman Collection v2.1 导入导出
- 单接口 / 全项目导出 Markdown 文档
- 客户端代码生成：curl / Python / JavaScript / Go（自动含变量替换与认证头）

### 更多

- GraphQL 调试视图
- 备份（JSON）与恢复（ID 全量重映射，绝不覆盖现有数据）
- 深色 / 浅色 / 跟随系统主题

## 下载与安装

在 [Releases](https://github.com/weihubeats/RustFox/releases) 下载对应平台安装包：

| 平台 | 安装包 |
| --- | --- |
| Windows | `RustFox_*-x64-setup.exe`（NSIS 安装包，SmartScreen 提示时选「更多信息 → 仍要运行」） |
| macOS | `RustFox_*-aarch64.dmg`（Apple Silicon）/ `RustFox_*-x64.dmg`（Intel） |
| Linux | `.deb` / `.rpm` / `.AppImage` |

> **macOS 首次打开提示「已损坏，无法打开」？** 应用本身没有损坏——这是 Gatekeeper
> 对未做 Apple 签名公证应用的拦截。把应用拖入「应用程序」后，在终端执行一次：
>
> ```bash
> xattr -cr /Applications/RustFox.app
> ```
>
> 然后右键 →「打开」即可。详见[使用手册](docs/USER_GUIDE.md#12-macos)。

安装后可在应用内「关于 → Check for Updates」检查并一键升级新版本（v0.0.3 起支持自动更新）。

## 从源码构建（开发者）

前置：Rust 工具链 + Node 22。

```bash
cargo build --workspace        # 构建全部后端 crate
cargo test --workspace         # 运行全部测试
npm --prefix frontend install
npm --prefix frontend run tauri dev     # 开发模式（Vite HMR）
scripts/package-tauri.sh                # 一键打包分发包
```

> 架构原理、Crate 划分、IPC 与数据流等开发细节见 **[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)**（Tauri 2 + Vue 3 三层架构，含架构图）。

## 文档

| 文档 | 说明 |
| --- | --- |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | 架构总览（Tauri 2 + Vue 3，含架构图与界面布局图） |
| [docs/USER_GUIDE.md](docs/USER_GUIDE.md) | 用户手册 |
| [docs/SPEC.md](docs/SPEC.md) | 详细规范（模型 / 数据库 / 命令） |
| [docs/SMOKE_TEST.md](docs/SMOKE_TEST.md) | 手动验收清单 |
| [docs/DEPLOY.md](docs/DEPLOY.md) | 发布与部署 |
| [docs/MILESTONES.md](docs/MILESTONES.md) | 里程碑总览 |
| [docs/PROGRESS.md](docs/PROGRESS.md) | 开发进度记录 |

## License

[Apache-2.0](LICENSE)