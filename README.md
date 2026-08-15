# RustFox

基于 Rust 的本地优先 API 管理工具。一个二进制，开箱即用。

## 优势

| 维度 | RustFox | Postman | Bruno | Insomnia |
| --- | --- | --- | --- | --- |
| 安装包体积 | **~21 MB**（单二进制） | ~310 MB (Electron) | ~433 MB (Electron) | ~200 MB (Electron) |
| 首屏启动 | **< 1 秒** | 2-4 秒 | 2-5 秒 | 2-4 秒 |
| 运行时内存 | **~40 MB** | ~500 MB+ | ~300 MB+ | ~200 MB+ |
| 构建语言 | Rust | Chromium + Node.js | Chromium + Node.js | Chromium + Node.js |

**体积缩小 15-20 倍**：同类 Electron 工具把整个 Chromium 浏览器打包进去，RustFox 用 Dioxus Desktop 只编译一份 Rust 二进制，无运行时依赖，无 Node.js 沙箱。

**性能优势来源**：Rust LTO 链接优化 + 单一进程模型 + SQLite 零拷贝本地存储。49 个源文件、17,800+ 行代码、1,142 个传递依赖，最终产物仅 21 MB。

## 功能

| 类别 | 能力 |
| --- | --- |
| 项目管理 | 项目 / 文件夹 / 接口 三级树结构 |
| 请求编辑 | 8 种 HTTP 方法、6 种请求体（JSON / Form / x-www-form-urlencoded / Multipart / GraphQL / Text） |
| 认证 | API Key / Basic / Bearer / OAuth2（Authorization Code / Client Credentials / Password / Implicit） |
| 环境变量 | 全局 / 环境级变量 + `{{name}}` 自动解析 |
| 响应查看 | 格式化 JSON / Raw / 响应头 / 耗时 / 状态码 / 下载保存 |
| 请求历史 | 侧边栏抽屉，支持重发、删除 |
| Mock Server | 本地轻量 Mock 服务，支持方法+路径+Header+Body 精确匹配 |
| 自动化测试 | 自定义 Test Steps、运行测试、查看结果、历史记录 |
| OpenAPI | 导入 / 导出 OpenAPI 3.x、Swagger 2.0 |
| Postman | 导入 / 导出 Postman Collection v2 / v2.1 |
| cURL | 一键粘贴导入，自动识别方法 / URL / Header / Body / Basic Auth |
| Markdown | 单接口 / 全项目导出 Markdown 文档 |
| 代码生成 | 自动生成客户端代码（多语言） |
| WebSocket | 连接、发送、记录 |
| 备份恢复 | 导出 / 导入数据文件 |
| 主题 | 深色 / 浅色 / 跟随系统 |

## 下载使用（无需安装 Rust）

在项目的 GitHub **Releases** 页面下载对应平台的二进制包，解压/安装后双击即可使用：

- **Windows**：`RustFox-*-setup.exe` 安装包（自动创建开始菜单与桌面快捷方式）或便携 zip
- **macOS**：`RustFox-*-macos-*.dmg`（拖拽安装，推荐）或 zip（内含 `RustFox.app`，拖入「应用程序」即可）
- **Linux**：`tar.gz`，解压后运行 `./install_linux.sh` 添加到应用菜单

### 基本使用流程

1. 启动应用，首页点击「创建项目」输入项目名称
2. 在左侧目录树新建文件夹 / 接口，填写方法、路径、参数
3. 顶部选择环境（若无则到设置页创建，支持变量 `{{name}}`）
4. 点击「发送」调试请求，查看响应 / 耗时 / 历史
5. 设置页可配置 Mock Server、运行自动化测试、导入导出 OpenAPI、备份恢复

完整用户手册见 [docs/USER_GUIDE.md](docs/USER_GUIDE.md)。

## 从源码构建（开发者）

前置要求：Rust 工具链（[rustup.rs](https://rustup.rs/) 安装），版本要求见 `rust-toolchain.toml`。

```
cargo run --release -p fox-desktop
```

```bash
cargo build --workspace        # 构建全部 crate
cargo run -p fox-desktop       # 启动桌面应用（调试模式）
cargo test --workspace         # 运行全部测试
cargo clippy --workspace --all-targets -- -D warnings   # 静态检查
```

## 打包分发包

打包脚本自动构建 release 二进制并产出分发包到 `dist/` 目录：

```bash
scripts/package.sh    # Linux / macOS：tar.gz 或 RustFox.app
scripts/package.bat   # Windows：便携 zip + NSIS 安装包
```

产物说明：

| 平台 | 脚本 | 产物 |
| --- | --- | --- |
| Linux | `scripts/package.sh` | `dist/RustFox-<version>-linux-<arch>.tar.gz`（含安装脚本） |
| macOS | `scripts/package.sh` | `dist/RustFox-<version>-macos-<arch>.zip` + `dist/RustFox-<version>-macos-<arch>.dmg`（内含 `RustFox.app`） |
| Windows | `scripts/package.bat` | `dist/RustFox-<version>-windows-x86_64.zip`（便携版） + `dist/RustFox-<version>-setup.exe`（NSIS 安装包） |

注意事项：

- 版本号默认取自 `Cargo.toml`，可用 `RUSTFOX_VERSION=1.2.3` 环境变量覆盖（仅 `package.sh`）
- Windows 需要本地安装 [NSIS](https://nsis.sourceforge.io/)（`makensis`），未安装时仅生成便携 zip；CI 中会自动安装
- 推 `v*` 标签自动触发 GitHub Actions 构建三平台分发包并发布 Release（见 `.github/workflows/release.yml`）

## 技术栈

- **UI**：Dioxus 0.5（Rust 驱动的虚拟 DOM，单进程渲染）
- **运行时**：Tokio 异步运行时
- **存储**：SQLite（sqlx，零拷贝查询）
- **网络**：reqwest（HTTP 客户端）
- **Mock**：axum（Mock Server）
- **OpenAPI**：openapiv3（导入导出）
- **构建**：Cargo + ziglinker（可选）进一步压缩产物

## 文档

| 文档 | 说明 |
| --- | --- |
| [docs/USER_GUIDE.md](docs/USER_GUIDE.md) | 用户手册 |
| [docs/SPEC.md](docs/SPEC.md) | 详细规范 |
| [docs/MILESTONES.md](docs/MILESTONES.md) | 里程碑总览 |
| [docs/PROGRESS.md](docs/PROGRESS.md) | 开发进度记录 |
| [docs/DEPLOY.md](docs/DEPLOY.md) | 部署指南 |
| [docs/MIRROR_CN.md](docs/MIRROR_CN.md) | 国内网络镜像配置 |
| [docs/SMOKE_TEST.md](docs/SMOKE_TEST.md) | 手动验收清单 |
