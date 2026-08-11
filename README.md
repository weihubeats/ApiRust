# RustFox

基于 Rust 的本地优先 API 管理工具。

## 下载使用（无需安装 Rust）

在项目的 GitHub **Releases** 页面（点仓库右侧 Releases）下载对应平台的二进制包，解压/安装后双击即可使用：

- **Windows**：`RustFox-*-setup.exe` 安装包（自动创建开始菜单与桌面快捷方式）或便携 zip
- **macOS**：zip 内含 `RustFox.app`，拖入「应用程序」即可
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

```bash
cargo build --workspace        # 构建全部 crate
cargo run -p fox-desktop       # 启动桌面应用
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
| macOS | `scripts/package.sh` | `dist/RustFox-<version>-macos-<arch>.zip`（内含 `RustFox.app`） |
| Windows | `scripts/package.bat` | `dist/RustFox-<version>-windows-x86_64.zip`（便携版） + `dist/RustFox-<version>-setup.exe`（NSIS 安装包） |

注意事项：

- 版本号默认取自 `Cargo.toml`，可用 `RUSTFOX_VERSION=1.2.3` 环境变量覆盖（仅 `package.sh`）
- Windows 需要本地安装 [NSIS](https://nsis.sourceforge.io/)（`makensis`），未安装时仅生成便携 zip；CI 中会自动安装
- 推 `v*` 标签自动触发 GitHub Actions 构建三平台分发包并发布 Release（见 `.github/workflows/release.yml`）

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

## 技术栈

- Rust（Dioxus Desktop UI、Tokio 异步运行时）
- SQLite（sqlx 本地存储）
- reqwest（HTTP 客户端）
- axum（Mock Server）
- openapiv3（OpenAPI 导入导出）