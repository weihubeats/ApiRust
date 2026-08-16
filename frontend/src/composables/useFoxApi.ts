/**
 * useFoxApi：Tauri IPC 的统一前端封装（Vue 3 Composable）。
 *
 * 职责：
 * 1. 把 `invoke('plugin:fox|command', args)` 收敛为类型安全的 API 方法（类型来自 foxApi.d.ts）；
 *    前缀 `plugin:fox` 对应 fox-tauri 插件名（`Builder::new("fox")`），
 *    插件命令必须带命名空间，与 capabilities 里 `fox:default` 权限对应；
 * 2. 统一错误处理：后端 `{ code, message }` → 携带 code 的 Error；
 * 3. 维护「当前激活项目 / 环境」的响应式缓存，与后端 RwLock 状态保持单向同步；
 * 4. `pending` 标志位供全局 loading 指示。
 *
 * 用法：
 * ```ts
 * const api = useFoxApi()
 * const projects = await api.getProjects()
 * api.saveProject({ ... })          // 失败时抛出携带 code 的 Error
 * api.setActiveProject(projects[0].id)  // 自动同步 activeProject 响应式缓存
 * ```
 */
import { invoke } from '@tauri-apps/api/core'
import { ref } from 'vue'
import { useProgress } from './useProgress'
import type {
  AuthSpec,
  BackupSummary,
  BodySpec,
  CodeLang,
  CommandError,
  CurlParsed,
  Endpoint,
  EndpointResult,
  Environment,
  ExecuteRequestArgs,
  ExecuteResponse,
  Folder,
  HttpMethod,
  ImportResult,
  KeyValue,
  LoadResult,
  MockRule,
  OAuth2Token,
  Project,
  RequestHistory,
  RequestSpec,
  ResponseExample,
} from '../types/foxApi'

/** 插件命令统一前缀：`plugin:{插件名}|{命令名}`。 */
const PLUGIN = 'plugin:fox'

/** 后端 `{ code, message }` → 前端 Error（code 挂载在 err.code，供程序化分支）。 */
export function toFoxError(raw: unknown): Error {
  if (raw && typeof raw === 'object' && 'message' in raw) {
    const { code, message } = raw as CommandError
    const err = new Error(message)
    Object.defineProperty(err, 'code', { value: code, enumerable: true })
    return err
  }
  return raw instanceof Error ? raw : new Error(String(raw))
}

/** 主密钥问题提示（对应后端 DECRYPT 错误码）。 */
const DECRYPT_WARNING = '主密钥不匹配或已损坏，无法解密环境变量，请检查备份'

/** 提示去重：同一会话只提示一次，避免批量环境列表解密失败时重复弹出。 */
let decryptionWarned = false

function warnDecryptionFailed(): void {
  if (decryptionWarned) return
  decryptionWarned = true
  import('./useToast').then(({ useToast }) => {
    useToast().error('解密失败', { message: DECRYPT_WARNING, duration: 0 })
  })
}

/** 统一的带错误映射的 invoke 封装（自动加插件命名空间前缀）。 */
async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(`${PLUGIN}|${command}`, args)
  } catch (e) {
    const err = toFoxError(e)
    if ('code' in err && err.code === 'DECRYPT') {
      warnDecryptionFailed()
    }
    throw err
  }
}

export function useFoxApi() {
  /** 全局请求中标记（可用于按钮 loading）。 */
  const pending = ref(false)

  /** 全局顶部加载进度条（配合 ProgressBar 组件）。 */
  const progress = useProgress()

  /** 激活项目 / 环境的响应式缓存（由 setActive* 命令同步）。 */
  const activeProject = ref<Project | null>(null)
  const activeEnvironment = ref<Environment | null>(null)

  /** 并发请求深度：归零时才结束进度条，避免嵌套请求提前收尾。 */
  let inflight = 0

  /** 执行一个异步任务并维护 pending 状态与顶部进度条。 */
  async function run<T>(task: () => Promise<T>): Promise<T> {
    if (inflight === 0) {
      pending.value = true
      progress.start()
    }
    inflight += 1
    try {
      return await task()
    } finally {
      inflight -= 1
      if (inflight === 0) {
        pending.value = false
        progress.done()
      }
    }
  }

  // ---------- 项目 ----------
  const getProjects = () => run(() => call<Project[]>('get_projects'))

  const saveProject = (project: Project) => run(() => call<Project>('save_project', { project }))

  const deleteProject = (projectId: string) =>
    run(() => call<void>('delete_project', { projectId }))

  async function setActiveProject(projectId: string | null): Promise<Project | null> {
    const project = await run(() => call<Project | null>('set_active_project', { projectId }))
    activeProject.value = project
    return project
  }

  const getActiveProject = () => call<Project | null>('get_active_project')

  // ---------- 接口 ----------
  const listEndpoints = (projectId: string) =>
    run(() => call<Endpoint[]>('list_endpoints', { projectId }))

  const getEndpoint = (endpointId: string) => run(() => call<Endpoint>('get_endpoint', { endpointId }))

  const saveEndpoint = (endpoint: Endpoint) =>
    run(() => call<Endpoint>('save_endpoint', { endpoint }))

  const deleteEndpoint = (endpointId: string) =>
    run(() => call<void>('delete_endpoint', { endpointId }))

  const duplicateEndpoint = (endpointId: string) =>
    run(() => call<Endpoint>('duplicate_endpoint', { endpointId }))

  // ---------- 文件夹 ----------
  const listFolders = (projectId: string) =>
    run(() => call<Folder[]>('list_folders', { projectId }))

  const saveFolder = (folder: Folder) => run(() => call<Folder>('save_folder', { folder }))

  const deleteFolder = (folderId: string) =>
    run(() => call<void>('delete_folder', { folderId }))

  // ---------- cURL 导入 ----------
  const parseCurlCommand = (command: string) =>
    run(() => call<CurlParsed>('parse_curl_command', { command }))

  // ---------- 环境 ----------
  const listEnvironments = (projectId: string) =>
    run(() => call<Environment[]>('list_environments', { projectId }))

  const saveEnvironment = (environment: Environment) =>
    run(() => call<Environment>('save_environment', { environment }))

  async function setActiveEnvironment(environmentId: string | null): Promise<Environment | null> {
    const environment = await run(() =>
      call<Environment | null>('set_active_environment', { environmentId }),
    )
    activeEnvironment.value = environment
    return environment
  }

  const getActiveEnvironment = () => call<Environment | null>('get_active_environment')

  const deleteEnvironment = (environmentId: string) =>
    run(() => call<void>('delete_environment', { environmentId }))

  // ---------- 请求执行 ----------
  const executeRequest = (args: ExecuteRequestArgs) =>
    run(() => call<ExecuteResponse>('execute_request', { args }))

  /** 取消一个在途请求（requestId 不存在或已完成时返回 false）。 */
  const cancelRequest = (requestId: string) =>
    call<boolean>('cancel_request', { requestId })

  // ---------- 响应示例 ----------
  const listExamples = (endpointId: string) =>
    run(() => call<ResponseExample[]>('list_examples', { endpointId }))

  const saveExample = (example: ResponseExample) =>
    run(() => call<ResponseExample>('save_example', { example }))

  const deleteExample = (exampleId: string) =>
    run(() => call<void>('delete_example', { exampleId }))

  // ---------- OAuth2 ----------
  const oauthAuthorize = (auth: AuthSpec) =>
    run(() => call<OAuth2Token>('oauth_authorize', { auth }))

  const oauthAccessToken = (auth: AuthSpec) =>
    run(() => call<string>('oauth_access_token', { auth }))

  // ---------- 代码生成 ----------
  const codegenRender = (args: {
    lang: CodeLang
    method: HttpMethod
    url: string
    headers: KeyValue[]
    body: BodySpec
    auth: AuthSpec
  }) => run(() => call<string>('codegen_render', { args }))

  // ---------- 请求历史 ----------
  const listRequestHistories = (projectId: string, limit?: number) =>
    run(() => call<RequestHistory[]>('list_request_histories', { projectId, limit }))

  // ---------- Mock 服务 ----------
  const mockStart = () => run(() => call<string>('mock_start'))

  const mockStop = () => run(() => call<void>('mock_stop'))

  const mockStatus = () => run(() => call<string | null>('mock_status'))

  // ---------- Mock 规则 ----------
  const listMockRules = (projectId: string) =>
    run(() => call<MockRule[]>('list_mock_rules', { projectId }))

  const saveMockRule = (rule: MockRule) => run(() => call<MockRule>('save_mock_rule', { rule }))

  const deleteMockRule = (ruleId: string) =>
    run(() => call<void>('delete_mock_rule', { ruleId }))

  // ---------- HTTP 设置（全局代理） ----------
  const getHttpProxy = () => run(() => call<string | null>('get_http_proxy'))

  const setHttpProxy = (proxy: string | null) =>
    run(() => call<void>('set_http_proxy', { proxy }))

  // ---------- 备份/恢复 ----------
  const backupExport = (projectId: string) =>
    run(() => call<string>('backup_export', { projectId }))

  const backupRestore = (text: string) =>
    run(() => call<BackupSummary>('backup_restore', { text }))

  // ---------- 导入导出 ----------
  const importDocument = (text: string) =>
    run(() => call<ImportResult>('import_document', { text }))

  const exportOpenapi = (projectId: string) =>
    run(() => call<string>('export_openapi', { projectId }))

  // ---------- 测试 / 压测 ----------
  const testEndpoint = (args: { endpoint: Endpoint; url: string; environment_id: string | null }) =>
    run(() => call<EndpointResult>('test_endpoint', { args }))

  const loadTest = (args: {
    url: string
    method: HttpMethod
    spec: RequestSpec
    environment_id: string | null
    concurrency: number
    total: number
  }) => run(() => call<LoadResult>('load_test', { args }))

  return {
    pending,
    activeProject,
    activeEnvironment,
    getProjects,
    saveProject,
    deleteProject,
    setActiveProject,
    getActiveProject,
    listEndpoints,
    getEndpoint,
    saveEndpoint,
    deleteEndpoint,
    duplicateEndpoint,
    listFolders,
    saveFolder,
    deleteFolder,
    parseCurlCommand,
    listEnvironments,
    saveEnvironment,
    setActiveEnvironment,
    getActiveEnvironment,
    deleteEnvironment,
    executeRequest,
    cancelRequest,
    listExamples,
    saveExample,
    deleteExample,
    oauthAuthorize,
    oauthAccessToken,
    codegenRender,
    listRequestHistories,
    mockStart,
    mockStop,
    mockStatus,
    listMockRules,
    saveMockRule,
    deleteMockRule,
    getHttpProxy,
    setHttpProxy,
    backupExport,
    backupRestore,
    importDocument,
    exportOpenapi,
    testEndpoint,
    loadTest,
  }
}

/** 供 provide/inject 或 store 使用的 Api 类型。 */
export type FoxApi = ReturnType<typeof useFoxApi>
