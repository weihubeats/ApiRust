# RustFox 冒烟测试 / 手动验收流程（SMOKE_TEST）

> 本文档对应的自动化补充见 `crates/fox-smoke/tests/smoke_test.rs`
> （`cargo test --workspace` 时自动执行，纯逻辑、不需 UI）。

## 0. 术语

- **数据目录**：macOS `~/Library/Application Support/RustFox`；日志在 `{数据目录}/logs/rustfox.log`；
  诊断报告生成在 `{数据目录}/reports/`；备份在 `{数据目录}/backups/`；导出文档在 `{数据目录}/exports/`。

## 1. 基础链路：创建项目 → 环境 → 接口 → 发送请求 → 查看响应

1. 启动应用（`cargo run -p fox-desktop`），首次进入「首页」空状态。
2. 创建项目：输入名称（如 `演示项目`）+ 描述，点「创建项目」→ Toast「项目创建成功」，卡片出现。
3. 左上角项目下拉选中该项目 → 进入「工作区」，左侧目录树出现。
4. 右上「＋ 新建接口」（或树内「＋ 接口」）→ 输入名称保存。
5. **无环境变量**：URL 输入框填完整地址（如 `https://httpbin.org/get`），方法选 GET，点「发送」。
   - 预期：状态徽章 200 绿色、耗时/大小显示、正文区展示 JSON；Toast「请求完成，已保存历史」。
6. 点「历史」标签 → 最近 50 条出现，含方法/URL/状态/耗时。
7. 设置页「变量」→ 该接口路径改为 `{{base_url}}/get`，项目变量 `base_url=https://httpbin.org`（或建环境「测试」再设 `base_url`）→ 重新发送，预期同样 200。

**自动化对应**：`smoke_test.rs` 中 `full_user_flow`（含环境创建 + mock 请求 + 历史落库）。

## 2. 导入 OpenAPI → 查看接口 → 发送请求

1. 设置页「OpenAPI 导入」文本框粘贴任意 OpenAPI 3.0 / Swagger 2.0 / Postman v2.1 JSON/YAML（可从 https://petstore.swagger.io/v2/swagger.json 取样例）。
2. 选择冲突策略，点「导入」→ Toast「导入完成（OpenAPI 3.0）：新建 N，覆盖 M，跳过 K」。
3. 左侧树出现按 tag 分组的文件夹与接口；双击打开任一接口，确认方法/路径/参数/示例已填充。
4. 若接口指向本机（启动 Mock 后把路径头部改为 mock 地址，见第 3 节）可真实发送验证。

**自动化对应**：`openapi_export_import_and_backup`（导出 → 再导入 → 断言 path/method/参数一致）。

## 3. 启动 Mock → curl 验证 → 停止

1. 在工作区保存一个接口（GET `/api/hello`，Path 填 `/api/hello`）。
2. 设置页「Mock」→ 确认端口（默认 4010 起）→ 点「启动 Mock」→ Toast「Mock 服务已启动：http://127.0.0.1:4010」。
3. 终端验证：
   ```bash
   curl -i http://127.0.0.1:4010/api/hello
   # 预期 200 + JSON 响应体（含接口名/假数据或已保存的响应示例）
   curl -i http://127.0.0.1:4010/不存在路径
   # 预期 404 JSON
   ```
4. 点「停止 Mock」→ Toast「Mock 服务已停止」，再次 curl 应连接失败。
5. 日志 `rustfox.log` 应出现 `用户启动 Mock port=4010` 与 `[mock] Mock 服务已启动`。

**自动化对应**：`full_user_flow` 中启动/请求/停止真实 Mock 服务。

## 4. 配置断言 → 运行测试 → 查看结果

1. 打开某接口 → Tests 标签：
   ```json
   { "assertions": [
       {"name": "状态码 200", "type": "status", "op": "eq", "expected": 200},
       {"type": "jsonpath", "path": "$.title", "op": "exists"}
   ] }
   ```
2. 保存接口 → 点「运行测试」→ 预期：通过 1 / 失败 0，行内状态「通过」。
3. 把断言改成 `expected: 500` → 运行 → 预期显示失败及原因。
4. 「运行项目测试」遍历所有接口；点历史任意条目「详情」查看逐条明细。
5. Toast：`测试完成：N 通过 / M 失败 / K 跳过`；日志出现 `用户运行测试 count=N`。

**自动化对应**：`full_user_flow` 的 `run_endpoint`（status + jsonpath 断言通过）与 `save_test_run`。

## 5. 备份项目 → 恢复项目 → 验证数据一致

1. 设置页「备份」→ 点「备份项目」→ Toast 显示备份文件路径（`{数据目录}/backups/项目名_时间.json`）。
2. 删除或改动当前项目数据（可仅删一个接口验证）。
3. 设置页「恢复」→ 粘贴上一步文件内容 → 点「恢复备份」→ 新项目出现，名称 = 备份时的名称。
4. 打开恢复的项目 → 接口/文件夹/环境/规则数量与备份前一致；发送一个请求验证数据可用。
5. 备份文件内容为 JSON，`format` 字段 = `rustfox-project-backup`。

**自动化对应**：`openapi_roundtrip_and_backup` 中 `build_backup → serialize → parse → restore_backup` 后
逐字段核对（接口数、路径、方法、环境变量、示例）。

## 6. 诊断报告（问题反馈）

1. 顶栏「反馈」按钮 → Toast 显示报告路径：`{数据目录}/reports/rustfox_report_时间.md`。
2. 打开文件核对：应包含 ① 环境信息（OS/架构/版本/数据目录）② 最近操作步骤列表 ③ 最近日志（最多 500 行）。
3. 将报告内容粘贴到 GitHub Issue（https://github.com/weihubeats/ApiRust/issues）即可提交。

## 7. 验收标准

- [ ] `cargo test --workspace` 全绿（含 `fox-smoke` 2 个用例）；
- [ ] `rustfox.log` 中出现 4 类关键日志（发送请求 / 保存接口 / 启动 Mock / 运行测试）；
- [ ] 反馈按钮一次点击生成报告文件；
- [ ] 上述 1~5 节每一步结果与「预期」一致。