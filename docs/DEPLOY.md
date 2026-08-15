# RustFox 部署与使用指南

RustFox 是一个本地运行的 API 调试与接口测试工具（桌面应用 + 内置 Mock Server），
单机离线运行，数据默认保存在本机，无需外部服务。

## 环境要求

| 项目 | 要求 |
| --- | --- |
| 操作系统 | macOS / Linux / Windows（Dioxus 桌面） |
| Rust 工具链 | ≥ 1.79（构建时需要，运行不需要） |
| 网络 | 仅构建期需要下载依赖；运行期完全离线 |

中国大陆网络建议先配置镜像，见 [MIRROR_CN.md](MIRROR_CN.md)。

## 构建

```bash
# 开发模式（HMR + 调试）
npm --prefix frontend install
npm --prefix frontend run tauri dev

# 发布构建（bundle 产物）
scripts/package-tauri.sh
```

产物在 `frontend/src-tauri/target/release/bundle/`（`.app`/`.dmg`/`.deb`/`.AppImage`/`.msi`/NSIS `.exe` 按平台产出）。

## 运行

- **开发**:`npm --prefix frontend run tauri dev` 自动启动应用窗口
- **发布**:安装对应平台分发包即可;macOS 拖 `RustFox.app` 进「应用程序」

启动后会自动初始化 SQLite 数据库与数据目录。

## 数据与文件

所有数据在「系统数据目录 / RustFox」下：

| 路径 | 内容 |
| --- | --- |
| `~/.local/share/RustFox/rustfox.db`（Linux/macOS 类似） | 主数据库（项目/接口/环境/Mock 规则/响应示例/测试历史/请求历史） |
| `~/.local/share/RustFox/master.key` | 环境变量加密主密钥（AES-256-GCM，权限 0600） |
| `~/.local/share/RustFox/backups/` | 「备份当前项目」导出的 JSON |
| `~/.local/share/RustFox/exports/` | 「导出项目 Markdown」生成的文档 |

> 提示：macOS 为 `~/Library/Application Support/RustFox`。

### 环境变量加密

环境变量值在写入数据库前使用 AES-256-GCM 加密，密钥保存在 `master.key`。
旧版本（明文）数据读取时自动兼容，下次保存后转为密文。
**请勿删除 `master.key`**——删除后已加密的变量将无法解密（程序会容错按密文原样显示，
此时可用「备份恢复」从备份 JSON 找回明文数据）。

## 核心功能速览

| 功能 | 位置 |
| --- | --- |
| 项目管理（新建/导入/导出） | 项目首页 + 设置页 |
| 多标签编辑（独立草稿 / 未保存标记 / 新建） | 工作区顶部标签栏 |
| 接口编辑（参数/请求头/JSON Body/认证） | 工作区 Params/Headers/Body/Auth Tab |
| 发送请求 + 历史记录 | 工作区地址栏 + 历史按钮 |
| 环境与项目变量 | 设置页（{{变量}} 在请求中自动解析） |
| 导入兼容（OpenAPI 3.0 / Swagger 2.0 / Postman v2.1，JSON+YAML） | 设置页 |
| 自动化测试（配置/断言/变量链） | 工作区 Tests Tab + 历史测试列表 |
| 接口压测（并发基准） | 工作区 Tests Tab「压测」区 |
| 客户端代码生成（curl / Python / JS / Go） | 工作区地址栏「生成代码」 |
| Mock Server | 设置页（端口 4010 起自动 +1） |
| 响应示例 / 项目文档导出 | 工作区 Docs Tab |
| 备份 / 恢复 | 设置页 |

## 多标签编辑

- 点击左侧目录中的接口 → 在顶部标签栏打开（已打开则激活）；「＋ 新建」创建空白接口草稿。
- 每个标签的未保存修改独立保留，切换标签不丢失；标题上的「●」表示未保存（保存后消失）。
- 关闭标签：× 按钮；关闭活动标签自动切换到最后一个，如有未保存修改会提示后丢弃。

## 客户端代码生成

工作区地址栏「生成代码」按钮 → 弹窗选择语言（curl / Python(requests) / JavaScript(fetch) / Go(net/http)），
基于**渲染后**的请求生成（自动完成变量、环境变量、路径变量、base_url 替换；含认证头与启用的请求头）。

## 接口压测（并发基准）

Tests Tab「压测」区：输入并发数（默认 10）与总请求数（默认 100）→「开始压测」。
结果展示：成功/失败次数、总耗时、QPS、平均耗时、P50/P90/P99 分位耗时、错误示例（最多 5 条）；
结束后作为一行「压测」记录写入测试历史（json 含 kind=load 与完整统计）。

## Mock Server

- 启动后监听 `http://127.0.0.1:4010`；端口被占用时自动 +1（最多尝试 20 次）。
- 优先级：自定义规则 > 接口响应示例 > 默认响应。
- 路径参数：`/users/{id}`；Query/Header 匹配每行 `key=value`。
- body 模板变量：`{{params.id}}`、`{{query.name}}`、`{{headers.X-Token}}`、
  `{{mock.uuid|email|name|word|timestamp|int}}`。
- 修改接口或规则后需重启 Mock 生效。

## 自动化测试

Tests Tab 的 JSON 配置（写入接口 `request_json.tests`，保存后生效）：

```json
{
  "pre_request": [{ "name": "t", "value": "{{$timestamp}}" }],
  "extract": [{ "from": "body", "path": "$.id", "name": "uid" }],
  "assertions": [
    { "type": "status", "op": "eq", "expected": 200 },
    { "type": "body", "op": "contains", "expected": "hello" },
    { "type": "jsonpath", "op": "eq", "path": "$.code", "expected": 0 },
    { "type": "response_time_ms", "op": "lt", "expected": 1000 }
  ]
}
```

- 变量按目录顺序在请求间传递；`expected` 支持 `{{变量}}` 解析。
- 支持运行当前接口 / 当前文件夹 / 整个项目，结果自动入库（历史 20 条，可展开查看与删除）。

## 备份与恢复

- 备份：设置页 →「备份当前项目」，JSON 写入 `backups/`（含接口、环境、Mock 规则、响应示例）。
- 恢复：粘贴备份 JSON →「恢复」，创建为**全新项目**（ID 全部重新映射，不覆盖现有数据）。

## 常见问题

| 现象 | 处理 |
| --- | --- |
| 构建失败：`invalid metadata files for crate rustversion` | `rm -f target/release/deps/librustversion-*.dylib` 后重新构建 |
| Mock 端口被占用 | 工具自动 +1；如需固定端口，先停止占用方 |
| 导入 OpenAPI 报「无法识别」 | 支持 OpenAPI 3.0 / Swagger 2.0 / Postman v2.1（JSON+YAML），3.1+ 请先转换；检查是否为有效 JSON/YAML |
| 环境变量显示为密文 | 见上文「环境变量加密」，用备份恢复找回 |
| 数据库损坏 | 全部数据在 `rustfox.db`，备份目录中的 JSON 可完整恢复 |

## 开发

```bash
# 全部测试（单元 + 集成）
cargo test --workspace

# 代码规范
cargo fmt
cargo clippy --workspace --all-targets -- -D warnings

# 单 crate 测试
cargo test -p fox-backup -p fox-secret
```

里程碑规划见 [PROGRESS.md](PROGRESS.md)。
