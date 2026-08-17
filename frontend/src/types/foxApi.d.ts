/**
 * fox-core / fox-http 模型的 TypeScript 镜像。
 *
 * 生成方案（二选一）：
 * 方案 A（推荐）：tauri-specta 在构建期自动生成（`fox-tauri` 插件的 `bindings.ts`），
 *                Rust 侧模型需要 derive `specta::Type`，产物为「命令 + 类型」一体文件；
 * 方案 B：手写维护本文件。注意：修改 Rust 模型后必须同步本文件，并在 CI 里
 *        加一个字段快照比对（如 `ts-json-schema-generator` + `git diff`）防漂移。
 */

/** 统一命令错误（后端 AppError → { code, message }）。 */
export interface CommandError {
  code:
    | 'DATABASE'
    | 'IO'
    | 'HTTP'
    | 'TIMEOUT'
    | 'SSL'
    | 'DNS'
    | 'CONNECTION'
    | 'VALIDATION'
    | 'NOT_FOUND'
    | 'OPENAPI'
    | 'MOCK'
    | 'TEST'
    | 'SCRIPT'
    | 'WEBSOCKET'
    | 'JSON'
    | 'DECRYPT'
    | 'OAUTH2'
  message: string
}

export type HttpMethod = 'GET' | 'POST' | 'PUT' | 'DELETE' | 'PATCH' | 'HEAD' | 'OPTIONS'

export type EndpointStatus =
  | 'designing'
  | 'developing'
  | 'testing'
  | 'released'
  | 'deprecated'

export type ApiKeyLocation = 'header' | 'query'

/** Query / Header / Path 变量等键值对（Rust `KeyValue`）。 */
export interface KeyValue {
  key: string
  value: string
  enabled: boolean
  description: string
}

/** 认证方式（Rust `AuthSpec`，tag = "type"）。 */
export type AuthSpec =
  | { type: 'none' }
  | { type: 'bearer'; token: string }
  | { type: 'basic'; username: string; password: string }
  | { type: 'apikey'; key: string; value: string; in: ApiKeyLocation }
  | {
      type: 'oauth2'
      client_id: string
      client_secret: string
      auth_url: string
      token_url: string
      scope: string
      redirect_uri: string
      token?: OAuth2Token
    }

/** OAuth2 令牌（`expires_at` 为 UTC 时刻）。 */
export interface OAuth2Token {
  access_token: string
  token_type?: string
  refresh_token?: string
  expires_at: string
}

/** OAuth2 授权状态（UI 状态指示器用）。 */
export type OAuth2Status = 'unauthorized' | 'valid' | 'expiring_soon' | 'expired'

/** Multipart 值类型。 */
export type MultipartValueType = 'text' | 'file_path'

export interface MultipartField {
  key: string
  value_type: MultipartValueType
  value: string
  enabled: boolean
}

/** GraphQL 请求（Rust `GraphQLSpec`，变量为 JSON 文本，operationName 可空）。 */
export interface GraphQLSpec {
  query: string
  variables: string
  operation_name: string
}

/** 请求 Body（Rust `BodySpec`，tag = "mode"）。 */
export type BodySpec =
  | { mode: 'none' }
  | { mode: 'json'; raw: string }
  | { mode: 'text'; raw: string }
  | { mode: 'urlencoded'; fields: KeyValue[] }
  | { mode: 'multipart'; fields: MultipartField[] }
  | { mode: 'graphql'; spec: GraphQLSpec }
  | { mode: 'binary'; path: string }

/** GraphQL 错误位置（Rust `GraphQLErrorLocation`）。 */
export interface GraphQLErrorLocation {
  line: number
  column: number
}

/** GraphQL 错误条目（Rust `GraphQLError`）。 */
export interface GraphQLError {
  message: string
  locations: GraphQLErrorLocation[] | null
  path: (string | number | boolean | null)[] | null
}

/** 解析后的 GraphQL 响应（Rust `GraphQLResponse`）。 */
export interface GraphQLResponse {
  data: unknown
  errors: GraphQLError[]
}

/** 统一请求结构（Rust `RequestSpec`）。 */
export interface RequestSpec {
  params: KeyValue[]
  headers: KeyValue[]
  path_variables: KeyValue[]
  auth: AuthSpec
  body: BodySpec
  /** 编辑器配置 Tab 记忆（params/auth/headers/body/...），空时按 Method 智能默认。 */
  active_tab?: string | null
  timeout_ms: number
  follow_redirects: boolean
  tests: unknown | null
}

/** 项目（Rust `Project`）。 */
export interface Project {
  id: string
  name: string
  description: string
  variables: Record<string, string>
  created_at: string
  updated_at: string
}

/** 文件夹（Rust `Folder`）。 */
export interface Folder {
  id: string
  project_id: string
  parent_id: string | null
  name: string
  sort_order: number
  created_at: string
  updated_at: string
}

/** 接口（Rust `Endpoint`）。 */
export interface Endpoint {
  id: string
  project_id: string
  folder_id: string | null
  name: string
  method: HttpMethod
  path: string
  description: string
  status: EndpointStatus
  sort_order: number
  request: RequestSpec
  created_at: string
  updated_at: string
}

/** 环境（Rust `Environment`）。 */
export interface Environment {
  id: string
  project_id: string
  name: string
  variables: Record<string, string>
  created_at: string
  updated_at: string
}

/** 执行请求入参（Rust `ExecuteRequestArgs`）。 */
export interface ExecuteRequestArgs {
  url: string
  method: HttpMethod
  spec: RequestSpec
  environment_id: string | null
  project_id?: string | null
  endpoint_id?: string | null
  /** 请求取消标识（前端生成；提供后可通过 cancelRequest 中止在途请求）。 */
  request_id?: string | null
}

/** 请求历史（Rust `RequestHistory`，fox-tauri `list_request_histories` 返回）。 */
export interface RequestHistory {
  id: string
  project_id: string
  endpoint_id: string | null
  method: string
  url: string
  status: number | null
  duration_ms: number | null
  request_summary_json: string
  response_summary_json: string
  created_at: string
}

/** 执行请求出参（Rust `ExecuteResponse`）。 */
export interface ExecuteResponse {
  status: number
  headers: [string, string][]
  body: string
  content_type: string
  duration_ms: number
  size_bytes: number
  truncated: boolean
}

/** cURL 命令解析结果（Rust `CurlParsed`，fox-tauri `parse_curl_command` 返回）。 */
export interface CurlParsed {
  url: string
  method: HttpMethod
  headers: KeyValue[]
  body: BodySpec | null
  auth: AuthSpec
}

/** 响应示例（Rust `ResponseExample`，fox-tauri `list_examples` 等返回）。 */
export interface ResponseExample {
  id: string
  endpoint_id: string
  name: string
  status: number
  headers: Record<string, string>
  body: string
  content_type: string
  created_at: string
  updated_at: string
}

/** 请求用例（Rust `RequestExample`）：接口请求快照，可一键回填编辑器。 */
export interface RequestExample {
  id: string
  endpoint_id: string
  name: string
  request: RequestSpec
  created_at: string
  updated_at: string
}

/** 测试用例分组（与后端 CATEGORIES 一致）。 */
export type TestCaseCategory = '正向' | '负向' | '边界值' | '安全性' | '其他'

/** 测试用例运行状态（Rust `TestCaseStatus`）。 */
export type TestCaseStatus = 'Success' | 'Failed' | 'Untested'

/** 测试用例（Rust `TestCase`，fox-tauri `list_test_cases` 等返回）。 */
export interface TestCase {
  id: string
  /** 关联的主接口 ID（endpoints.id）。 */
  request_id: string
  name: string
  category: TestCaseCategory
  method: HttpMethod
  url_path: string
  params: KeyValue[]
  headers: KeyValue[]
  /** body 类型标识：json / form-data / raw / urlencoded / graphql / binary / none。 */
  body_type: string
  body_content: string
  last_run_status: TestCaseStatus
  created_at: string
}

/** 代码生成语言（Rust `Lang`，fox-tauri `codegen_render` 的 `lang` 取值）。 */
export type CodeLang = 'curl' | 'python' | 'js' | 'go' | 'java' | 'php'

/** 备份恢复摘要（fox-tauri `backup_restore` 返回）。 */
export interface BackupSummary {
  id: string
  name: string
  folders: number
  endpoints: number
  environments: number
  mock_rules: number
  response_examples: number
}

/** 导入文档格式（Rust `ImportFormat`）。 */
export type ImportFormat = 'openapi30' | 'swagger20' | 'postman21' | 'unknown'

/** 导入的示例（Rust `ImportedExample`）。 */
export interface ImportedExample {
  name: string
  status: number
  content_type: string
  headers: Record<string, string>
  body: string
}

/** 导入的接口（Rust `ImportedEndpoint`，fox-tauri `import_document` 返回）。 */
export interface ImportedEndpoint {
  name: string
  method: HttpMethod
  path: string
  description: string
  request: RequestSpec
  examples: ImportedExample[]
  folder_hint: string | null
}

/** 导入解析结果。 */
export interface ImportResult {
  format: ImportFormat
  endpoints: ImportedEndpoint[]
}

/** 单条断言结果（Rust `Outcome`）。 */
export interface Outcome {
  description: string
  passed: boolean
  reason: string | null
}

/** 单接口测试结果（Rust `EndpointResult`）。 */
export interface EndpointResult {
  endpoint_id: string
  endpoint_name: string
  method: string
  path: string
  ok: boolean
  status: number | null
  duration_ms: number | null
  request_error: string | null
  outcomes: Outcome[]
}

/** 压测结果（Rust `LoadResult`）。 */
export interface LoadResult {
  total: number
  ok: number
  failed: number
  total_ms: number
  avg_ms: number
  p50_ms: number
  p90_ms: number
  p99_ms: number
  rps: number
  errors: string[]
}

/** Mock 匹配项（Rust `MockMatchItem`，query / header 匹配键值）。 */
export interface MockMatchItem {
  key: string
  value: string
}

/** Mock 规则（Rust `MockRule`，fox-tauri `list_mock_rules` 等返回）。 */
export interface MockRule {
  id: string
  project_id: string
  endpoint_id: string | null
  name: string
  method: HttpMethod
  path: string
  match_query: MockMatchItem[]
  match_headers: MockMatchItem[]
  response_status: number
  response_headers: Record<string, string>
  response_body_template: string
  delay_ms: number
  enabled: boolean
  priority: number
  created_at: string
  updated_at: string
}