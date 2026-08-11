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
    | 'VALIDATION'
    | 'NOT_FOUND'
    | 'OPENAPI'
    | 'MOCK'
    | 'TEST'
    | 'SCRIPT'
    | 'WEBSOCKET'
    | 'JSON'
    | 'DECRYPT'
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

/** Multipart 值类型。 */
export type MultipartValueType = 'text' | 'file_path'

export interface MultipartField {
  key: string
  value_type: MultipartValueType
  value: string
  enabled: boolean
}

/** 请求 Body（Rust `BodySpec`，tag = "mode"）。 */
export type BodySpec =
  | { mode: 'none' }
  | { mode: 'json'; raw: string }
  | { mode: 'text'; raw: string }
  | { mode: 'urlencoded'; fields: KeyValue[] }
  | { mode: 'multipart'; fields: MultipartField[] }

/** 统一请求结构（Rust `RequestSpec`）。 */
export interface RequestSpec {
  params: KeyValue[]
  headers: KeyValue[]
  path_variables: KeyValue[]
  auth: AuthSpec
  body: BodySpec
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