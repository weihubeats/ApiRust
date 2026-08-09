# RustFox 开发进度

进度记录：每完成一个里程碑追加一条记录。

## M0：仓库初始化（✅ 完成）

- [x] 创建 Cargo workspace（resolver = "2"），根 `Cargo.toml` 与 SPEC 第 6 节一致
- [x] 创建 `rust-toolchain.toml`（stable）
- [x] 创建全部 7 个 crate：fox-core / fox-storage / fox-http / fox-openapi / fox-mock / fox-test / fox-desktop
- [x] 配置 workspace dependencies
- [x] 创建 `docs/SPEC.md`、`docs/PROGRESS.md`、`README.md`
- [x] `cargo build --workspace` 通过

备注：

- `async-trait` 从未发布 1.0，实际最新为 0.1.x，workspace 中修正为 `"0.1"`（SPEC 笔误）。
- 内部 crate 依赖使用 path 依赖，根 `Cargo.toml` 保持与 SPEC 完全一致。

## M1：核心模型与数据库（✅ 完成）

- [x] fox-core 领域模型：HttpMethod / EndpointStatus / RequestSpec / AuthSpec / BodySpec / Project / Folder / Endpoint / Environment（含 ResponseExample / MockRule / TestRun / RequestHistory）
- [x] AppError 统一错误类型（含 user_message 中文提示）
- [x] 变量引擎：`{{name}}` 解析、内置变量（$uuid/$timestamp/$isoTimestamp/$randomInt）、嵌套递归上限 10 层、优先级合并（运行时 > 环境 > 项目）
- [x] 工具函数：URL 拼接规则、路径变量替换、JSON 格式化
- [x] fox-storage：SQLite 连接（WAL、外键）、`migrations/0001_init.sql`、数据目录 `{DataDir}/RustFox`
- [x] Repository：Project / Folder / Endpoint / Environment CRUD、duplicate_endpoint、settings 键值
- [x] 单元测试：变量替换、URL 拼接、模型序列化（25 个用例）
- [x] 集成测试：SQLite CRUD、级联删除（6 个用例）
- [x] `cargo fmt --all` / `cargo clippy -D warnings` / `cargo test --workspace` 全部通过

备注：

- `AppError` 需要引用 sqlx / reqwest 错误类型，因此 fox-core 增加了这两个依赖（符合 SPEC §23.1 单一错误类型要求）。
- `MigrateError` 不属于 `sqlx::Error`，迁移失败显式映射为 `AppError::Database`。

## M2：桌面应用骨架（✅ 完成）

- [x] 日志初始化：stdout + `{DataDir}/RustFox/logs/rustfox.log`（RUST_LOG 控制级别，默认 info）
- [x] 数据目录与 SQLite 初始化（`~/Library/Application Support/RustFox/rustfox.db`）
- [x] Dioxus 桌面应用：深色主题、三栏骨架（TopBar 48px / Sidebar 280px / Main）
- [x] TopBar：项目名、环境选择器、搜索框、设置入口
- [x] 首页：项目创建、项目列表、切换项目、删除项目（Toast 中文提示）
- [x] 设置页占位
- [x] `cargo run -p fox-desktop` 可正常启动并渲染（冒烟测试 20s 无 panic）
- [x] fmt / clippy（-D warnings）/ test 全部通过

备注：

- Dioxus 0.5.6 的 `use_context_provider` 初始化闭包内不允许调用 hook（会 BorrowMutError panic），全局状态改用 `Signal::new` 构造。
- `Signal` 为 Copy 类型，异步任务通过克隆出的 `mut` 局部信号完成写入。

## M3：目录树与接口管理（✅ 完成）

- components/project_tree.rs：SideBar 侧栏（目录树）完整实现。
  - 递归 FolderNode 渲染文件夹树（含子文件夹），EndpointRow 渲染接口行（方法徽章 + 名称 + 路径）。
  - 顶部按钮：新建文件夹、新建接口；行内操作：接口 / 子目录 / 改名 / 删除 / 复制。
  - 模态框（内联）：新建文件夹 / 新建接口 / 重命名，支持 Enter 提交、空名称 Toast 拦截。
  - 搜索过滤：TopBar 搜索框按名称/路径过滤树内接口（无匹配提示）。
  - 分发器模式：`Dispatcher = Rc<RefCell<dyn FnMut(TreeAction)>>` 经 use_context_provider 下发给子组件（Signal 非 Send/Sync，故用 Rc；Signal::set 需 &mut，故 FnMut）。
- state.rs：新增 delete_folder / delete_endpoint / duplicate_endpoint / create_folder_at / create_endpoint_at / rename_folder / rename_endpoint / open_endpoint_tab（打开 Tab、切到 Workspace 页、高亮选中），移除 open_tabs / active_endpoint_id / current_project_id 的 #[allow(dead_code)]。
- fox-storage：新增 repository::update_folder（重命名文件夹落库），folder_crud 测试补充重命名断言。
- pages/workspace.rs：Workspace 占位页（显示当前接口名 / 方法 / 路径 + 返回按钮），M4 替换为完整编辑器；app.rs 增加 Page::Workspace 分支。
- 门禁：fmt + clippy -D warnings + test（31 通过）+ 冒烟 12s 无 panic。

## M4：接口编辑器（✅ 完成）

- pages/workspace.rs：完整接口编辑器。
  - url-bar：Method 下拉（7 种方法，FromStr 解析）+ Path 编辑 + 保存按钮。
  - editor-meta：Name / Description 编辑。
  - Tabs（Params / Headers / Body / Auth）：
    - Params / Headers：KeyValue 行编辑（启用勾选、Key、Value、描述、删除、添加行）。
    - Body：模式切换（无 / JSON / 文本 / UrlEncoded），JSON/文本 textarea 编辑，JSON 模式带「格式化 JSON」按钮（合法则格式化并写回，非法弹中文 Toast）；UrlEncoded 复用 KeyValue 表。
    - Auth：无认证 / Bearer / Basic / API Key（含请求头/查询参数位置选择），切换时保留已有字段。
  - 草稿机制：`draft: Signal<Option<Endpoint>>` + use_effect 守卫（仅当 active_endpoint_id 变化时重新装载，避免重渲染覆盖编辑）；保存走 state.save_endpoint（repo::update_endpoint + 列表更新 + Toast）。
  - Rust 2024 陷阱记录：`if let Some(x) = sig.write().as_mut()` 的临时 Write guard 在 if-let 判定后立即释放（edition 2024 临时作用域变化），必须显式 `let mut guard = d.write();`。
- state.rs：新增 save_endpoint（校验名称为空由页面拦截）。
- styles.rs：补齐 .editor-meta / .body-editor / .tabs button / .auth-field / .label-hint / .modal-backdrop / .modal（此前模态框无样式）。
- 单元测试 +3（body 模式切换保留 raw、auth 切换保留字段、format_json 合法/非法）。
- 门禁：fmt + clippy -D warnings（0 错误）+ test（34 通过）+ 冒烟 12s 无 panic。

## M5：HTTP 调试（✅ 完成）

- fox-http：`crates/fox-http/src/client.rs` 请求引擎完整实现。
  - `send_request(method, url, RequestSpec, timeout)`：Query 参数、Headers、Body（JSON / Text / UrlEncoded，Multipart 暂未实现返回 None）、Auth（Bearer / Basic(base64) / API Key 请求头或查询参数）、默认超时 30s、响应流式读取 20MB 截断（truncated=true）、cookies 收集、时长/大小统计。
  - 错误映射：`AppError::Http(#[from] reqwest::Error)`，URL 非法 / 连接失败 / 超时均有中文提示。
  - 单元测试 10 个（本地 TcpListener 伪 HTTP 服务器）：Query 拼接、JSON POST、Basic Auth、UrlEncoded、非法 URL、连接拒绝、超时、20MB 截断、Payload 构建、API Key 查询参数。
  - Cargo.toml 新增依赖：`futures`（workspace）、`base64 = "0.22"`、`serde_urlencoded = "0.7"`。
- fox-storage：repository 新增 `save_request_history` / `list_request_histories`（对应 migrations request_histories 表）。
- state.rs：新增 `toast_success` / `toast_info`（此前仅 toast_error）。
- pages/workspace.rs：发送 / 取消 / 响应展示 / 历史。
  - do_send：校验（草稿存在、项目已选、URL 前缀）、变量渲染（merged_vars 项目+环境、路径/Query/Headers/Auth/Body 全字段 resolve）、`tokio::oneshot` + `tokio::select!` 实现取消，发送后保存历史并刷新 50 条。
  - 响应区：状态码（2xx 绿 / 其他红）、耗时、大小、截断警告、Content-Type、响应头列表、body 预览（JSON 自动格式化）。
  - 历史模态框：列表（方法徽章 / URL / 状态 / 时间）、点击查看摘要（request/response JSON 美化）。
  - 教训：rsx 事件闭包内调用捕获的闭包（FnMut/FnOnce 捕获冲突）→ 抽为模块级普通函数（load_history_list）或预克隆句柄；for 循环迭代 clone 避免移动 Vec。
- styles.rs：新增响应区 / 历史模态框 / .btn.send / .btn.danger / 状态徽章样式。
- 门禁：fmt + clippy -D warnings（0 错误）+ test --workspace（44 通过：25+6+3+10）+ 冒烟 12s 无 panic。

## M6：环境与变量（✅ 完成）

- state.rs：新增环境管理方法。
  - `create_environment`：新建并自动设为当前环境（中文 Toast）。
  - `delete_environment`：删除并清理当前环境选择。
  - `save_environment`：名称 + 变量落库（repo::update_environment）。
  - `select_environment`：切换当前环境（顶部选择器与设置页共用 current_environment_id）。
  - `save_project_variables`：项目变量落库（repo::update_project 的 variables_json）。
- pages/settings.rs：设置页实现。
  - 环境管理：新建（名称校验）、列表（使用 / 编辑 / 删除）、环境编辑器（名称 + 变量行增删改，{{key}} 引用提示）。
  - 项目变量：KeyValue 行编辑 + 保存，提示「环境 > 项目 > 内置」优先级。
  - 编辑草稿按 env_item 辅助函数渲染，避免 rsx 循环内多闭包 move 捕获冲突（预克隆 state 句柄 select_btn/delete_btn/save_btn）。
- 变量替换复用 M5 的 merged_vars + resolve_text：环境 / 项目 / 内置变量优先级正确（SPEC §12.2），TopBar 环境选择器切换后请求 URL 随之变化（验收 1/2/3 覆盖）。
- 单元测试 +3（vars_from_rows：跳过空键、键去空白、空输入 → 空 Map）。
- styles.rs：新增 .settings-section / .section-title / .env-item / .env-current / .env-editor / .kv-title。
- 门禁：fmt + clippy -D warnings（0 错误）+ test --workspace（47 通过）+ 冒烟 12s 无 panic。

## M7：OpenAPI 导入导出（✅ 通过）

- fox-openapi/import.rs：OpenAPI 3.0 解析（自动识别 JSON/YAML）+ 版本校验（3.1 及其他版本明确中文报错）。
  - paths → endpoints；summary 兜底 "{method} {path}" 命名。
  - parameters query/header/path 分别映射到 params / headers / path_variables（example 转值）。
  - requestBody application/json → BodySpec::Json（无 example 时 "{}"）；x-www-form-urlencoded → UrlEncoded（example 对象转字段）。
  - responses → ResponseExample（状态码、content-type、headers、body）。
  - `ConflictStrategy { Skip(默认) / Overwrite / Duplicate }`。
  - 引用（$ref）条目跳过（阶段①策略）。
- fox-openapi/export.rs：项目 → OpenAPI 3.0.3 JSON。
  - 每个 endpoint 转 path item；params/headers/path_vars 转 parameters（query/header/path）；json body 转 requestBody。
  - response_examples 转 responses；无示例时默认生成 200 空响应。
  - 依赖新增 `indexmap = { workspace = true }`（openapiv3 不 re-export IndexMap）。
- fox-storage：ResponseExample 行映射 + create_response_example / list_response_examples / delete_response_examples。
- state.rs：`import_openapi(text, strategy)`（三步：同步解析校验 → 异步落库 → 汇总 Toast「新建/覆盖/跳过」）；`export_openapi(cb)`（回调返回 JSON 文本或中文错误）。
- 设置页新增「OpenAPI 导入导出」区块：粘贴 JSON/YAML、冲突策略下拉、导入 / 导出当前项目按钮（导出结果回填文本框）。
- 单元测试 +10（导入 JSON/YAML/3.1 拒绝/非法文档/空 paths/参数与 body 映射、冲突策略、导出结构、导出→再导入 roundtrip）。
- 门禁：fmt + clippy -D warnings（0 错误）+ test --workspace（57 通过）+ 冒烟 12s 无 panic。

## M8：Mock Server（✅ 通过）

- `crates/fox-mock` 引擎：`MockDefinition`（method/path/match_query/match_headers/status/headers/body_template/delay_ms/priority/source：Rule > Example > Default）、`MockStore`（set_definitions）、路径参数匹配 `/users/{id}`、query/header 匹配、模板渲染（`{{params.id}}/{{query.name}}/{{headers.X-Token}}/{{mock.uuid|email|name|word|timestamp|int}}`，未知变量置空）、延迟模拟、`MockServer` 端口 4010..4029 自动 +1（最多 20 次）。
- 单元测试 7 + 集成测试 4（真实启动 + HTTP 验证：模板路径响应、规则 query/header 命中/未命中 404、延迟计时、未匹配 404）。
- fox-storage：`MockRuleRow` + create/list/update/delete_mock_rule（match_query/match_headers/response_headers 存 JSON，delay_ms/enabled/priority）。
- fox-desktop：`mock_rules`/`mock_handle` 状态、`refresh_project_data` 加载规则、`start_mock`（跳过 Deprecated 接口，规则 > 示例 > 默认构建 defs）/`stop_mock`/`add_mock_rule`（表单校验+解析）/`create_mock_rule`/`delete_mock_rule`；设置页新增 Mock Server 区块（运行状态、启动/停止、规则表单：名称/方法/路径/状态码/优先级/延迟/Query/Header 匹配/body 模板、规则列表+删除）。
- 门禁：fmt + clippy -D warnings（0 错误）+ test --workspace（68 通过：25+6+10+7+4+10+6）+ 冒烟 12s 无 panic。

## M9：自动化测试（✅ 通过）

- `crates/fox-test` 引擎（SPEC §17）：
  - `config.rs`：`TestSpec { pre_request: [set_variable], extract: [from: body|header + path], assertions: [type: status|header|body|jsonpath|response_time_ms, op, expected, path] }`，从 `request_json.tests` 容错解析（非法配置直接失败）。
  - `assert.rs`：10 种操作符 eq/neq/contains/not_contains/gt/gte/lt/lte/exists/not_exists；宽松比较（数字/布尔/字符串）；大小写不敏感 Header。
  - `extract.rs`：jsonpath-rust JSONPath 提取（首个匹配，Null 视为未提取）与 Header 提取。
  - `runner.rs`：`run_endpoint`（pre_request 变量注入{{$timestamp}}与运行时上下文 → 发送 → 断言（expected 支持 {{变量}} 解析）→ extract 回写运行时变量）、`order_endpoints` 按目录+sort_order 排序执行。
  - 单元测试 14 + 集成测试 3（axum 真实服务：变量链流转、断言失败原因上报、无配置跳过/坏配置快速失败）。
- fox-storage：`test_runs` 表 CRUD（`save_test_run` / `list_test_runs`，TestRunRow JSON 列存结果）。
- fox-desktop（workspace.rs Tests Tab）：JSON 配置编辑（草稿同步、解析错误提示）、运行测试（当前接口）/ 运行文件夹测试 / 运行项目测试按钮（进行中禁用）、测试结果区（通过/失败/跳过汇总 + 逐接口状态行 + 断言失败原因 + 失败行红色高亮）、结果入库 + Toast 汇总。
- 门禁：fmt + clippy -D warnings（0 错误）+ test --workspace（85 通过：25+6+10+7+4+10+6+14+3）+ 冒烟 12s 无 panic。

## M10：文档导出与备份（✅ 通过）

- fox-openapi/markdown.rs：`export_markdown(project_name, endpoints, examples_by_endpoint)` → Markdown 文档（项目标题 + 每接口：方法/路径/名称/描述/查询参数/请求头/请求体/认证 + 响应示例列表），测试 +2。
- `crates/fox-backup`：`BackupFile { format/version/exported_at + 项目、文件夹、接口、环境、Mock 规则、响应示例全量 }`，`build_backup` / `serialize` / `parse` / `restore_backup`（恢复时全部 UUID 重映射为全新项目）。
- fox-storage：`save_project/save_folder/save_endpoint/save_environment/save_mock_rule/save_response_example`（INSERT 指定 ID，供恢复使用）+ `delete_response_example`（单条删除）。
- fox-desktop（workspace.rs Docs Tab）：接口文档页（方法徽章/路径/描述/启用的参数与请求头/请求体/认证概览 + 响应示例列表与删除）、「保存为示例」按钮（当前响应 → 响应示例入库）、「导出项目 Markdown」（写入 ~/.rustfox/exports/ 并 Toast 完整路径）；响应示例随接口切换自动加载。
- fox-desktop（settings.rs 备份 / 恢复）：「备份当前项目」→ JSON 写入 ~/.rustfox/backups/；粘贴备份 JSON → 恢复为全新项目（重映射 UUID，不覆盖现有数据）。
- 门禁：fmt + clippy -D warnings（0 错误）+ test --workspace（91 通过：4+25+6+10+7+4+12+6+14+3）+ 冒烟（release 构建成功）。

## M11：测试历史 / 变量加密 / 部署文档（✅ 通过）

- `crates/fox-secret`：AES-256-GCM 加密（密钥 `~/.rustfox/master.key` 权限 0600，格式 `base64(nonce):base64(cipher)`，非密文格式原样容错），单测 6。
- fox-storage：环境变量写库整体加密、读库自动解密（`EnvironmentRow::from_model/into_model` 一处收敛；密钥不可用时降级明文保证可用）；新增 `delete_test_run`；repository_test 增加「落库密文 ≠ 明文」断言。
- workspace.rs Tests Tab：新增「历史测试」区块——自动加载最近 20 次（测试结果变化或切换项目时刷新），逐条显示时间/名称/通过/失败/跳过，可展开查看逐接口结果与失败原因，可单条删除。
- 文档：`docs/DEPLOY.md`（部署/数据目录/加密说明/Mock/测试/备份恢复/FAQ）、`docs/MIRROR_CN.md`（rustup + crates.io 墙内镜像、cargo vendor 离线方案）。
- 门禁：fmt + clippy -D warnings（0 错误）+ test --workspace（97 通过：4+25+6+10+7+4+12+6+6+14+3）+ 冒烟 8s 无 panic。

## M12：导入兼容（Swagger 2.0 + Postman v2.1）（✅ 通过）

- fox-openapi/import.rs：`ImportFormat` 自动识别（OpenAPI 3.0 / Swagger 2.0 / Postman v2.1 / 无法识别，JSON+YAML 统一解析）；`import_any` 统一入口；`ImportedEndpoint` 新增 `folder_hint`（OpenAPI tags / Postman 分组兜底）。
- 新模块 `fox-openapi/swagger2.rs`：paths + parameters（query/header/path/body/formData）、securityDefinitions（显式优先级 basic > bearer(Authorization) > apiKey）、produces/consumes、responses → 响应示例；formData 带 multipart 判定。
- 新模块 `fox-openapi/postman.rs`：item 递归（分组 → 文件夹）、url 对象/字符串（query disabled → 停用，路径剥离查询串保留 {{变量}}）、header、body（raw JSON/Text、urlencoded、formdata 文件/文本、file）、request/collection 级 auth（basic/bearer/apikey 真实结构）、response[] → 响应示例。
- state.rs：`import_openapi` 自动识别格式（导入 Toast 显示格式标签），folder_hint 自动建顶级文件夹。
- 设置页导入区文案更新（支持三种格式）。
- 单元测试 +14（detect 6、swagger2 4、postman 4）。
- 门禁：fmt + clippy -D warnings（0 错误）+ test --workspace（111 通过：26+25+6+10+7+4+6+14+3+10）+ 冒烟 8s 无 panic。

## M13：客户端代码生成（✅ 通过）

- `crates/fox-codegen`：生成 curl / Python(requests) / JavaScript(fetch) / Go(net/http) 客户端代码；`GenRequest`（method/url/headers/body/auth）+ `auth_headers`（Bearer/Basic/apikey 头合并去重）+ `body_parts`（JSON/text/urlencoded 百分号编码）+ 各语言转义（sh 单引号、JS/Python 双引号、Go raw string）；单测 7。
- fox-desktop workspace.rs：URL 栏「生成代码」按钮 → 弹窗（语言下拉切换即时重渲染，深色代码块）；基于 `render_request` 的变量/路径变量/base_url 渲染后的真实请求生成。
- 门禁：fmt + clippy -D warnings（0 错误）+ test --workspace（118 通过：7+26+25+6+10+7+4+6+14+3+10）+ 冒烟 8s 无 panic。

## M14：接口压测（✅ 通过）

- fox-test 新模块 `load.rs`：`run_load(method,url,spec,LoadConfig)` 信号量限并发，N 次请求统计 通过/失败/总耗时/平均/P50/P90/P99/QPS + 最多 5 条错误明细；单元测试 +3（percentile 边界 + 本地 axum 服务 20 请求 × 4 并发 + 连接失败容错）。
- fox-desktop workspace.rs Tests Tab 「压测」区：并发数 / 总次数输入 + 开始压测（进行中禁用），结果网格展示（成功/失败、总耗时、QPS、平均、P50/P90/P99、错误示例）；结束后作为单行测试结果入库（kind=load）进入测试历史，Toast 汇总。
- 门禁：fmt + clippy -D warnings（0 错误）+ test --workspace（121 通过：7+26+25+6+10+7+4+20+6+14+3+3）+ 冒烟 8s 无 panic。

## M15：多标签编辑（✅ 通过）

- 工作区顶部新增标签栏：左侧目录点击接口 → 打开/激活标签（复用 `state.open_tabs`，删除接口自动清理）；「＋ 新建」创建空白接口草稿标签。
- 每个标签独立保存未保存修改：`tab_drafts` 缓存 + 草稿切换前写回，切换/关闭不丢失；“●”脏标记（对比仓库已保存副本，保存后自动消除）。
- 关闭标签：活动标签关闭后自动切换到最后一个；存在未保存修改时 Toast 提示（丢弃）。
- 门禁：fmt + clippy -D warnings（0 错误）+ test --workspace（121 通过：7+26+25+6+10+7+4+20+6+14+3+3）+ 冒烟 8s 无 panic。

## M16：UI 视觉翻新（✅ 通过）

- styles.rs 全面重做（类名不变，仅视觉）：统一设计规范——深色渐变背景层级（bg/panel/panel-2）、主色 #3b82f6、成功/警告/危险语义色、10px 圆角、全局 4px 间距刻度、细滚动条、focus 高亮环（键盘可操作）。
- 可用性：按钮 hover/active/disabled 反馈与统一 32px 高度；输入框 hover/focus 状态；侧边树选中态描边；Toast 移到右下角；编辑器响应区深色代码底；标签栏与面板层级一致；method 徽章统一。
- 门禁：fmt + clippy -D warnings（0 错误）+ test --workspace（121 通过）+ 冒烟 8s 无 panic。

## M2.5：设计系统重构（✅ 通过）

- `styles.rs` 全面重写为 `DESIGN_SYSTEM_CSS`：（与 M16 的不同在于类名统一切换为 `rf-` 前缀）
  - 设计令牌全部收敛为 CSS 变量：字号 --fs-xxs..--fs-xl、颜色 --bg-0/1/2、--panel-*、--accent、--danger/--success/--warning、--text/--muted/--placeholder、间距 --s-1..--s-9（4px 刻度）、圆角 --r-*、焦点光环 --focus-ring、阴影、滚动条、z 层级（toast 100 / modal 90 / dropdown 80）。
  - 控件体系：`.rf-btn`（primary/ghost/danger/sm）、`.rf-input`/`.rf-input-sm`、`.rf-textarea`、`.rf-checkbox`、`.rf-card`。
  - 组件体系：`.rf-topbar`（48px、logo/分隔/下拉/spacer/搜索）、`.rf-tree-item`、`.rf-dropdown`（trigger/backdrop/menu/item）、`.rf-toast*`、`.rf-modal`、`.rf-kv-table`、`.rf-tabs`、`.rf-badge*`、`.rf-method-chip*`、`.rf-history-*`、`.rf-load-*`、`.rf-codegen-modal`、`.rf-docs-*`、`.rf-settings-*`、`.rf-env-*`、`.rf-mock-*`、`.rf-backup-*`、`.rf-test-*`、`.rf-response`、`.rf-empty`、`.rf-home*`、`.rf-project-*`；
  - 交互规范：所有可交互元素 hover / active / focus（focus 统一蓝光环 ring 3px）；按钮 26/32/40 三级高度；下拉菜单 z-index 分层；深色细滚动条。
  - 旧类名（`.sidebar`/`.editor`/`.modal` 等）在新文件末尾保留了兼容映射，业务页面已全部改用 rf- 类。
- `components/icons.rs`（新）：内联 SVG 图标工厂 `svg_base` + `SearchIcon / CaretIcon / PlusIcon / FolderIcon / SlidersIcon / XIcon`，viewBox 24、stroke=currentColor、width/height=16，颜色随文字语境变化；代码中不再使用任何 emoji/dingbat 图标（如测试失败行的 ✗ 已改为 XIcon）。
- `components/dropdown.rs`（新）：用 `.rf-dropdown` 组件替代原生下拉选择器（触发按钮 + backdrop 关闭 + Esc 关闭 + 选项回调 + 占位态），代码库中再无 `<select>`。
- 页面迁移：TopBar / 首页（空状态、项目卡片、创建表单、rf-hint 快捷键提示）/ 设置页（冲突策略、Mock 方法下拉、按钮与输入框）/ 工作区（方法/请求体/认证类型/API Key 位置/语言 5 处下拉；按钮、输入框、文本域、KV 表、测试历史、压测、codegen、文档页）全部过渡到 rf- 体系；`<style>` 由 app.rs 根组件统一挂载 `DESIGN_SYSTEM_CSS`（原 main.rs `with_custom_head` 移除），样式表只注入一次。
- 门禁：fmt + clippy -D warnings（0 错误）+ test --workspace（121 通过）+ 冒烟 8s 无 panic。

### M2.5 修复记录（2026-08-09）

- 「左上角选不了项目」：`select_project` 丢失了 `page.set(Page::Workspace)` 跳转，且全应用对 `current_project_id` 只用 `.peek()`（不订阅、不重渲染）→ 点击后界面无变化。修复：`select_project` 恢复进入工作区；`topbar.rs` / `app.rs` 改用 `.read()` 订阅。
- 「＋接口 没反应」：`project_tree.rs` 弹窗可见性/搜索/项目守卫均用 `.peek()`，`modal.set(Some(...))` 后不触发重渲染 → 弹窗永不出现。修复：`modal` / `search` / 守卫全部改 `.read()`。
- 教训：渲染期间对 Signal 的读取若需响应式更新必须用 `.read()`（订阅）；`.peek()` 只在事件回调/防抖场景使用。
- 回归修复：M2.5 顶栏重写时丢失了 M6 的「环境选择器」，已用 `rf-dropdown` 恢复（未选环境占位 + 切换即影响请求变量）；caret 箭头增加 `.rf-caret` 包裹以支持 open 旋转。
- 新增 Dropdown 无头测试（`components/dropdown_test.rs`，VirtualDom + handle_event 模拟点击：开合、选项选中回调、backdrop 关闭），门禁 test --workspace = 123 通过。

### 视觉验收清单（M2.5）

按以下 10 项逐条人工验收（已通过）：

1. 界面中不存在任何原生下拉框：源代码 grep `<select` 0 命中；全部为自定义 `rf-dropdown`（trigger 显示选中文案或占位「请选择」+ 展开后 backdrop + 菜单，点击外部 / 按 Esc 关闭，选项点击后回写并关闭）。
2. 所有控件（输入框 / 多行文本域 / 按钮 / 下拉）均带 `rf-` 前缀样式类；所有颜色 / 圆角 / 间距 / 字号全部引用 CSS 变量（styles.rs 之外的源文件无硬编码十六进制颜色）。
3. 每个可交互元素具备完整状态反馈：hover（边框/背景加深）、active（按下加深或缩放）、focus（3px 蓝色光环）；按钮 disabled 半透明且不可点。
4. 图标全部为内联 SVG（stroke=currentColor）：搜索、下拉箭头、加号、文件夹、设置滑杆、失败 X 等；界面中无 emoji 作为图标。
5. 顶栏 48px：logo「RustFox」+ 分隔线 + 项目下拉（未选时显示「未选择项目」占位、默认选中当前项目）+ 环境下拉 + spacer + 搜索框 + 反馈按钮 + 设置按钮；窗口缩放右侧元素不溢出。
6. 首页：无项目时空状态居中（文件夹图标 +「还没有项目」标题 + 引导文案）；有项目时卡片网格展示（名称 / 描述 / 接口数 / 更新时间 / 删除按钮）；创建表单两行输入 + 「创建项目」主按钮。
7. 工作区：方法选择下拉高亮当前方法徽章；目录树选中项有左侧高亮条；标签栏脏标记「●」；发送按钮 hover 变亮；响应区深色代码底 + 绿色/红色状态徽章；失败详情行红色 X 图标。
8. 设置页：环境列表行 hover 反馈；Mock 规则表单、OpenAPI 导入区、备份区均使用统一输入框/按钮/下拉风格；折叠面板的展开/收起状态清晰。
9. 全局配色为深色渐变层级（bg → panel → panel-2），主色蓝色（#3b82f6 系）、成功绿 / 警告黄 / 危险红语义色统一；Toast 右下角弹出且带对应语义色边框。
10. 质量标准：`cargo fmt --all` 无 diff、`cargo clippy -D warnings` 0 错误、`cargo test --workspace` 全部通过；任意窗口尺寸下布局不塌、无内容重叠；交互流畅无卡顿。

## M2.6：功能联通性验收与诊断（✅ 通过）

- 新增 `crates/fox-smoke`：`tests/smoke_test.rs` 端到端冒烟（纯逻辑，无 UI）：
  - `full_user_flow`：建项目 → 建环境 → 建接口 → 启动 Mock → 变量解析 + 真实 HTTP 请求 → 校验响应 → 保存请求历史 → 断言测试运行（status + jsonpath）→ 测试结果落库 → 停止 Mock；
  - `openapi_roundtrip_and_backup`：导出 OpenAPI 3.0 → 再导入（识别格式一致、path/method/参数/示例一致）→ 备份 `build_backup → serialize → parse → restore_backup`（逐字段一致性 + 重映射 UUID 后落库不冲突）。
- 关键操作日志（`tracing::info`，问题定界用）：
  - `用户点击发送请求 url={} method={}`（workspace.rs do_send）
  - `用户保存接口 id={} name={}`（workspace.rs save）
  - `用户启动 Mock port={}`（state.rs start_mock）
  - `用户运行测试 count={}`（workspace.rs run_tests）
- 操作步骤记录：`AppState::record_step`（环形保留 60 条）覆盖发送/保存/导出/导入/Mock 启停/备份/恢复/压测。
- 顶栏「反馈」按钮 → `feedback.rs::generate_report` 生成 `{数据目录}/reports/rustfox_report_时间.md`：环境信息（OS/版本/数据目录）+ 当前上下文 + 操作步骤 + 日志尾部 500 行。
- `docs/SMOKE_TEST.md`：5 组手动验收清单（基础链路 / 导入 / Mock+curl / 断言测试 / 备份恢复）+ 报告提交步骤。
- 门禁：fmt ✓、clippy -D warnings ✓、`cargo test --workspace`（125 = 123 + 冒烟 2）全绿；冒烟 10s 无 panic。
