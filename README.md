# RustFox

基于 Rust 的本地优先 API 管理工具。

## 技术栈

- Rust（Dioxus Desktop UI、Tokio 异步运行时）
- SQLite（sqlx 本地存储）
- reqwest（HTTP 客户端）
- axum（Mock Server）
- openapiv3（OpenAPI 导入导出）

## 里程碑

- M0 仓库初始化
- M1 核心模型与数据库
- M2 桌面应用骨架
- M3 目录树与接口管理
- M4 接口编辑器
- M5 HTTP 调试
- M6 环境与变量
- M7 OpenAPI 导入导出
- M8 Mock Server
- M9 自动化测试
- M10 文档与备份
- M11 测试历史 / 变量加密 / 部署文档
- M12 导入兼容（Swagger 2.0 / Postman v2.1）
- M13 客户端代码生成（curl / Python / JS / Go）
- M14 接口压测（并发基准：QPS / 分位耗时）
- M15 多标签编辑（独立草稿 / 未保存标记 / 新建标签）

详细规范见 [docs/SPEC.md](docs/SPEC.md)，进度见 [docs/PROGRESS.md](docs/PROGRESS.md)。
部署指南见 [docs/DEPLOY.md](docs/DEPLOY.md)，国内网络镜像配置见 [docs/MIRROR_CN.md](docs/MIRROR_CN.md)。

## 开发命令

```bash
cargo build --workspace
cargo test --workspace
cargo run -p fox-desktop
cargo clippy --workspace --all-targets -- -D warnings
```