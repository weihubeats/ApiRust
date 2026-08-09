/**
 * useFoxApi：Tauri IPC 的统一前端封装（Vue 3 Composable）。
 *
 * 职责：
 * 1. 把 `invoke('command', args)` 收敛为类型安全的 API 方法（类型来自 foxApi.d.ts）；
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
import type {
  CommandError,
  Endpoint,
  Environment,
  ExecuteRequestArgs,
  ExecuteResponse,
  Project,
} from '../types/foxApi'

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

/** 统一的带错误映射的 invoke 封装。 */
async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args)
  } catch (e) {
    throw toFoxError(e)
  }
}

export function useFoxApi() {
  /** 全局请求中标记（可用于按钮 loading）。 */
  const pending = ref(false)

  /** 激活项目 / 环境的响应式缓存（由 setActive* 命令同步）。 */
  const activeProject = ref<Project | null>(null)
  const activeEnvironment = ref<Environment | null>(null)

  /** 执行一个异步任务并维护 pending 状态。 */
  async function run<T>(task: () => Promise<T>): Promise<T> {
    pending.value = true
    try {
      return await task()
    } finally {
      pending.value = false
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

  // ---------- 请求执行 ----------
  const executeRequest = (args: ExecuteRequestArgs) =>
    run(() => call<ExecuteResponse>('execute_request', { args }))

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
    listEnvironments,
    saveEnvironment,
    setActiveEnvironment,
    getActiveEnvironment,
    executeRequest,
  }
}

/** 供 provide/inject 或 store 使用的 Api 类型。 */
export type FoxApi = ReturnType<typeof useFoxApi>
