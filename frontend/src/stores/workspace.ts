/**
 * workspace store：工作区的单一状态源（Pinia）。
 *
 * 职责：
 * 1. 持有激活项目 + 文件夹/接口的扁平列表（树形由组件递归组装）；
 * 2. 标签页管理：openTabs（接口 id 有序集合）+ activeTabId + drafts（未保存草稿）；
 * 3. 树/标签/编辑器统一通过本 store 读写，避免跨组件手递 ref。
 *
 * 草稿语义（阶段 3 对齐 Dioxus 版）：打开标签时克隆一份 Endpoint 为草稿，
 * 编辑只改草稿；「保存」调用 save_endpoint 后回写列表并清除脏标记。
 */
import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import { useFoxApi } from '../composables/useFoxApi'
import { useToast } from '../composables/useToast'
import { planCrossGroupMove, planSameGroupMove, wouldCreateCycle } from './treeOps'
import { splitUrl } from '../utils/url'
import type {
  AuthSpec,
  CurlParsed,
  Endpoint,
  Environment,
  ExecuteResponse,
  HttpMethod,
  OAuth2Token,
  Project,
  RequestHistory,
  ResponseExample,
} from '../types/foxApi'

/** 新建接口的默认请求规格（与 fox-core 模型字段一致）。 */
export function defaultRequestSpec(): Endpoint['request'] {
  return {
    params: [],
    headers: [],
    path_variables: [],
    auth: { type: 'none' },
    body: { mode: 'none' },
    timeout_ms: 30000,
    follow_redirects: true,
    tests: null,
  }
}

function eq(a: Endpoint, b: Endpoint): boolean {
  return JSON.stringify(a) === JSON.stringify(b)
}

export const useWorkspaceStore = defineStore('workspace', () => {
  const api = useFoxApi()
  const toast = useToast()

  const project = ref<Project | null>(null)
  const folders = ref<NonNullable<Awaited<ReturnType<typeof api.listFolders>>>[number][]>([])
  const endpoints = ref<Endpoint[]>([])
  const loadError = ref<string | null>(null)

  const openTabs = ref<string[]>([])
  const activeTabId = ref<string | null>(null)
  const drafts = ref<Map<string, Endpoint>>(new Map())

  /** 环境：列表 + 当前选中（execute_request 的 environment_id 来源）。 */
  const environments = ref<Environment[]>([])
  const activeEnvId = ref<string | null>(null)

  /** 会话级 Base URL（仅本次会话，不落库）；cURL 导入时自动预填为 URL 的 origin。 */
  const sessionBaseUrl = ref('http://localhost')

  /** 地址栏域名前缀（唯一真实数据源）：选中环境声明 `base_url` 变量时优先，否则回退会话 Base URL。 */
  const urlDomain = computed(() => {
    const env = environments.value.find((e) => e.id === activeEnvId.value)
    const v = env?.variables?.base_url
    if (v && v.trim()) return '{{base_url}}'
    return sessionBaseUrl.value || ''
  })

  /** 把当前选中环境的 base_url 变量更新为 url（地址栏粘贴完整 URL 时同步环境）。 */
  async function setEnvironmentBaseUrl(url: string): Promise<void> {
    const env = environments.value.find((e) => e.id === activeEnvId.value)
    if (!env) return
    const updated: Environment = {
      ...env,
      variables: { ...env.variables, base_url: url },
    }
    try {
      const saved = await api.saveEnvironment(updated)
      const idx = environments.value.findIndex((e) => e.id === env.id)
      if (idx !== -1) environments.value[idx] = saved
    } catch (err) {
      toast.error('更新环境变量失败', {
        message: err instanceof Error ? err.message : String(err),
      })
    }
  }

  /** 响应示例（按接口 id 缓存，openEndpoint 时懒加载）。 */
  const examples = ref<Map<string, ResponseExample[]>>(new Map())

  const activeEndpoint = computed(() => {
    if (!activeTabId.value) return null
    return drafts.value.get(activeTabId.value) ?? null
  })

  const isDirty = (id: string): boolean => {
    const draft = drafts.value.get(id)
    if (!draft) return false
    const saved = endpoints.value.find((e) => e.id === id)
    if (!saved) return true
    return !eq(draft, saved)
  }

  const draftOf = (id: string): Endpoint | null => drafts.value.get(id) ?? null

  /** 标签页标题：草稿名 > 保存名 > method+path。 */
  const titleOf = (id: string): string => {
    const d = drafts.value.get(id)
    if (d?.name) return d.name
    if (d) return `${d.method} ${d.path}`
    const saved = endpoints.value.find((e) => e.id === id)
    if (saved) return saved.name || `${saved.method} ${saved.path}`
    return '未保存'
  }

  async function load(projectId: string): Promise<void> {
    loadError.value = null
    try {
      const [p, f, e] = await Promise.all([
        api.getActiveProject(),
        api.listFolders(projectId),
        api.listEndpoints(projectId),
      ])
      project.value = p
      folders.value = f
      endpoints.value = e
    } catch (err) {
      loadError.value = err instanceof Error ? err.message : String(err)
      throw err
    }
  }

  /** 初始化：取激活项目（无则返回 null，调用方负责跳回项目列表）。 */
  async function init(): Promise<Project | null> {
    const p = await api.getActiveProject()
    if (!p) return null
    project.value = p
    await load(p.id)
    await loadEnvironments()
    return p
  }

  /** 刷新文件夹 + 接口（树操作后调用）。 */
  async function refresh(): Promise<void> {
    if (!project.value) return
    const [f, e] = await Promise.all([
      api.listFolders(project.value.id),
      api.listEndpoints(project.value.id),
    ])
    folders.value = f
    endpoints.value = e
  }

  /** 切换到另一项目：设为激活 → 清空标签/草稿/示例 → 重载树与环境。 */
  async function switchProject(projectId: string): Promise<void> {
    await api.setActiveProject(projectId)
    openTabs.value = []
    drafts.value = new Map()
    examples.value = new Map()
    activeTabId.value = null
    sessionBaseUrl.value = 'http://localhost'
    const p = await api.getActiveProject()
    if (!p) throw new Error('项目不存在')
    project.value = p
    await load(p.id)
    await loadEnvironments()
  }

  /** 打开接口为标签页；已打开则仅切换。草稿懒克隆自保存态。 */
  function openEndpoint(endpoint: Endpoint): void {
    if (!openTabs.value.includes(endpoint.id)) {
      openTabs.value.push(endpoint.id)
      drafts.value.set(endpoint.id, { ...endpoint, request: JSON.parse(JSON.stringify(endpoint.request)) })
      loadExamples(endpoint.id)
    }
    activeTabId.value = endpoint.id
  }

  /** 加载接口的响应示例（懒加载 + 缓存）。 */
  async function loadExamples(endpointId: string): Promise<void> {
    try {
      examples.value.set(endpointId, await api.listExamples(endpointId))
    } catch (err) {
      console.error('[workspace.loadExamples]', err)
      examples.value.set(endpointId, [])
    }
  }

  /** 把一次执行响应保存为示例（保存后刷新缓存）。 */
  async function saveAsExample(
    endpointId: string,
    name: string,
    response: ExecuteResponse,
  ): Promise<void> {
    const now = new Date().toISOString()
    const example = await api.saveExample({
      id: crypto.randomUUID(),
      endpoint_id: endpointId,
      name,
      status: response.status,
      headers: Object.fromEntries(response.headers),
      body: response.body,
      content_type: response.content_type,
      created_at: now,
      updated_at: now,
    })
    const list = examples.value.get(endpointId) ?? []
    const idx = list.findIndex((x) => x.id === example.id)
    if (idx === -1) list.unshift(example)
    else list[idx] = example
    examples.value.set(endpointId, list)
    toast.success(`示例已保存：${example.name}`)
  }

  async function removeExample(endpointId: string, exampleId: string): Promise<void> {
    await api.deleteExample(exampleId)
    const list = (examples.value.get(endpointId) ?? []).filter((x) => x.id !== exampleId)
    examples.value.set(endpointId, list)
  }

  /** 打开「新建接口」草稿标签页（未持久化，保存时生成 id）；默认标题「未命名接口」，创建后自动聚焦全选便于输入。 */
  function openNewEndpoint(folderId: string | null): void {
    const now = new Date().toISOString()
    const blank: Endpoint = {
      id: crypto.randomUUID(),
      project_id: project.value?.id ?? '',
      folder_id: folderId,
      name: '未命名接口',
      method: 'GET',
      path: '/',
      description: '',
      status: 'designing',
      sort_order: 0,
      request: defaultRequestSpec(),
      created_at: now,
      updated_at: now,
    }
    drafts.value.set(blank.id, blank)
    if (!openTabs.value.includes(blank.id)) openTabs.value.push(blank.id)
    activeTabId.value = blank.id
    focusTitle()
  }

  /** 标题聚焦信号：新建接口时自增，编辑器监听并聚焦全选标题（TabBar「+」/ ⌘T / 树内新建共用）。 */
  const focusTitleSignal = ref(0)
  function focusTitle(): void {
    focusTitleSignal.value += 1
  }

  function setDraft(endpoint: Endpoint): void {
    drafts.value.set(endpoint.id, { ...endpoint })
  }

  function closeTab(id: string): void {
    const idx = openTabs.value.indexOf(id)
    if (idx === -1) return
    openTabs.value.splice(idx, 1)
    drafts.value.delete(id)
    if (activeTabId.value === id) {
      activeTabId.value = openTabs.value[idx] ?? openTabs.value[idx - 1] ?? null
    }
  }

  /** 保存当前草稿：新建（列表无此 id）走创建，否则走更新。 */
  async function saveActiveDraft(): Promise<boolean> {
    const draft = activeEndpoint.value
    if (!draft) return false
    if (!draft.name.trim()) {
      toast.warning('接口名称不能为空')
      return false
    }
    if (!draft.path.trim().startsWith('/')) {
      toast.warning('接口路径必须以 / 开头')
      return false
    }
    try {
      const saved = await api.saveEndpoint(draft)
      const idx = endpoints.value.findIndex((e) => e.id === saved.id)
      if (idx === -1) {
        endpoints.value.push(saved)
      } else {
        endpoints.value[idx] = saved
      }
      drafts.value.set(saved.id, { ...saved })
      toast.success(`接口已保存：${saved.name}`)
      return true
    } catch (err) {
      toast.error('保存失败', { message: err instanceof Error ? err.message : String(err) })
      return false
    }
  }

  async function deleteEndpoint(endpointId: string): Promise<void> {
    await api.deleteEndpoint(endpointId)
    closeTab(endpointId)
    await refresh()
    toast.info('接口已删除')
  }

  async function duplicateEndpoint(endpointId: string): Promise<void> {
    const dup = await api.duplicateEndpoint(endpointId)
    await refresh()
    openEndpoint(dup)
    toast.info(`已复制：${dup.name}`)
  }

  /** 加载环境列表 + 当前激活环境。 */
  async function loadEnvironments(): Promise<void> {
    if (!project.value) return
    const [envs, active] = await Promise.all([
      api.listEnvironments(project.value.id),
      api.getActiveEnvironment(),
    ])
    environments.value = envs
    activeEnvId.value = active?.id ?? null
  }

  /** 切换激活环境（null = 不使用环境）；环境须属于当前项目（后端校验）。 */
  async function setEnvironment(environmentId: string | null): Promise<void> {
    const env = await api.setActiveEnvironment(environmentId)
    activeEnvId.value = env?.id ?? null
  }

  /** 新建环境（仅名称，变量编辑后续阶段接入）。 */
  async function createEnvironment(name: string): Promise<void> {
    if (!project.value) return
    const now = new Date().toISOString()
    const env = await api.saveEnvironment({
      id: crypto.randomUUID(),
      project_id: project.value.id,
      name,
      variables: {},
      created_at: now,
      updated_at: now,
    })
    environments.value.push(env)
    toast.success(`环境已创建：${env.name}`)
  }

  /** 保存（upsert）环境并同步本地列表。 */
  async function updateEnvironment(
    env: Environment,
    opts?: { silent?: boolean },
  ): Promise<Environment> {
    const saved = await api.saveEnvironment({ ...env, updated_at: new Date().toISOString() })
    const idx = environments.value.findIndex((e) => e.id === saved.id)
    if (idx === -1) environments.value.push(saved)
    else environments.value[idx] = saved
    if (!opts?.silent) toast.success(`环境已保存：${saved.name}`)
    return saved
  }

  /** 删除环境；若删除的是当前激活环境则清空选中。 */
  async function deleteEnvironment(environmentId: string): Promise<void> {
    await api.deleteEnvironment(environmentId)
    environments.value = environments.value.filter((e) => e.id !== environmentId)
    if (activeEnvId.value === environmentId) activeEnvId.value = null
    toast.success('环境已删除')
  }

  /** 导入接口落地：按 folder_hint 复用/新建文件夹，逐接口保存并附带示例。 */
  async function importEndpoints(
    items: Array<{ name: string; method: string; path: string; description?: string; request: Endpoint['request']; examples?: Array<{ name: string; status: number; content_type: string; headers: Record<string, string>; body: string }>; folder_hint?: string | null }>,
  ): Promise<{ endpoints: number; examples: number }> {
    if (!project.value) return { endpoints: 0, examples: 0 }
    const now = new Date().toISOString()
    let exampleCount = 0

    for (const item of items) {
      let folderId: string | null = null
      if (item.folder_hint?.trim()) {
        const existing = folders.value.find((f) => f.name === item.folder_hint)
        if (existing) {
          folderId = existing.id
        } else {
          const folder = await api.saveFolder({
            id: crypto.randomUUID(),
            project_id: project.value.id,
            parent_id: null,
            name: item.folder_hint,
            sort_order: folders.value.length,
            created_at: now,
            updated_at: now,
          })
          folders.value.push(folder)
          folderId = folder.id
        }
      }
      const endpoint = await api.saveEndpoint({
        id: crypto.randomUUID(),
        project_id: project.value.id,
        folder_id: folderId,
        name: item.name,
        method: item.method as Endpoint['method'],
        path: item.path,
        description: item.description ?? '',
        request: item.request,
        sort_order: endpoints.value.length,
        status: 'designing',
        created_at: now,
        updated_at: now,
      })
      endpoints.value.push(endpoint)
      for (const ex of item.examples ?? []) {
        await api.saveExample({
          id: crypto.randomUUID(),
          endpoint_id: endpoint.id,
          name: ex.name,
          status: ex.status,
          headers: ex.headers ?? {},
          body: ex.body,
          content_type: ex.content_type,
          created_at: now,
          updated_at: now,
        })
        exampleCount++
      }
    }
    return { endpoints: items.length, examples: exampleCount }
  }

  /** 发送草稿请求（url 为拼接后的完整地址；环境变量由后端按 environment_id 注入）。
   *  提供 requestId 后该请求可被「取消」（后端中止连接并返回 CANCELLED）。 */
  async function send(endpoint: Endpoint, url: string, requestId?: string): Promise<ExecuteResponse> {
    let spec = endpoint.request
    const auth = spec.auth as AuthSpec | undefined
    if (auth?.type === 'oauth2' && (auth.auth_url?.trim() || auth.token_url?.trim())) {
      const token = await api.oauthAccessToken(auth)
      const base = (auth.token ?? {}) as Partial<Pick<OAuth2Token, 'token_type' | 'refresh_token' | 'expires_at'>>
      spec = { ...spec, auth: { ...auth, token: { ...base, access_token: token } as OAuth2Token } }
    }
    return api.executeRequest({
      url,
      method: endpoint.method,
      spec,
      environment_id: activeEnvId.value,
      project_id: project.value?.id ?? null,
      endpoint_id: endpoint.id,
      request_id: requestId ?? null,
    })
  }

  /** 树内重命名接口：保存 + 同步列表与打开中的草稿。 */
  async function renameEndpoint(endpointId: string, name: string): Promise<void> {
    const e = endpoints.value.find((x) => x.id === endpointId)
    if (!e) return
    const saved = await api.saveEndpoint({ ...e, name, updated_at: new Date().toISOString() })
    const idx = endpoints.value.findIndex((x) => x.id === endpointId)
    if (idx !== -1) endpoints.value[idx] = saved
    const draft = drafts.value.get(endpointId)
    if (draft) drafts.value.set(endpointId, { ...draft, name })
  }

  /** 将文件夹移动到 newParentId（null=根）的 targetIndex 处：重排相关兄弟组并落库。 */
  async function moveFolder(folderId: string, newParentId: string | null, targetIndex: number): Promise<void> {
    const moved = folders.value.find((f) => f.id === folderId)
    if (!moved) return
    // 防环：不允许把文件夹移入自身或其子孙（避免 parent_id 成环）
    if (wouldCreateCycle(folders.value, folderId, newParentId)) return
    let changed: Map<string, number>
    if (moved.parent_id === newParentId) {
      changed = planSameGroupMove(
        folders.value.filter((f) => f.parent_id === newParentId),
        folderId,
        targetIndex,
      )
    } else {
      changed = planCrossGroupMove(
        folders.value.filter((f) => f.parent_id === moved.parent_id),
        folders.value.filter((f) => f.parent_id === newParentId),
        folderId,
        targetIndex,
      )
      moved.parent_id = newParentId
    }
    if (changed.has(folderId)) moved.sort_order = changed.get(folderId)!
    await Promise.all(
      [...changed.keys()].map((id) => {
        const f = folders.value.find((x) => x.id === id)
        if (!f) return Promise.resolve()
        if (id === folderId) return api.saveFolder({ ...f })
        return api.saveFolder({ ...f, sort_order: changed.get(id)! })
      }),
    )
    await refresh()
  }

  /** 将接口移动到 folderId（null=根）的 targetIndex 处：重排相关兄弟组并落库。 */
  async function moveEndpoint(endpointId: string, newFolderId: string | null, targetIndex: number): Promise<void> {
    const moved = endpoints.value.find((e) => e.id === endpointId)
    if (!moved) return
    let changed: Map<string, number>
    if (moved.folder_id === newFolderId) {
      changed = planSameGroupMove(
        endpoints.value.filter((e) => e.folder_id === newFolderId),
        endpointId,
        targetIndex,
      )
    } else {
      changed = planCrossGroupMove(
        endpoints.value.filter((e) => e.folder_id === moved.folder_id),
        endpoints.value.filter((e) => e.folder_id === newFolderId),
        endpointId,
        targetIndex,
      )
      moved.folder_id = newFolderId
    }
    if (changed.has(endpointId)) moved.sort_order = changed.get(endpointId)!
    await Promise.all(
      [...changed.keys()].map((id) => {
        const e = endpoints.value.find((x) => x.id === id)
        if (!e) return Promise.resolve()
        if (id === endpointId) return api.saveEndpoint({ ...e })
        return api.saveEndpoint({ ...e, sort_order: changed.get(id)! })
      }),
    )
    await refresh()
  }

  async function saveFolder(folder: FolderInput): Promise<void> {
    await api.saveFolder(folder)
    await refresh()
  }

  async function deleteFolder(folderId: string): Promise<void> {
    const removed = endpoints.value.filter((e) => e.folder_id === folderId)
    removed.forEach((e) => closeTab(e.id))
    await api.deleteFolder(folderId)
    await refresh()
    toast.info('文件夹已删除（含子项）')
  }

  /** cURL 导入：打开为默认标题「未命名接口」的草稿（不落库），保存时生成 id；会话 Base URL 预填为 URL origin。 */
  function openCurlDraft(parsed: CurlParsed, folderId: string | null): void {
    const { path, params, origin } = splitUrl(parsed.url)
    const now = new Date().toISOString()
    const blank: Endpoint = {
      id: crypto.randomUUID(),
      project_id: project.value?.id ?? '',
      folder_id: folderId,
      name: '未命名接口',
      method: parsed.method,
      path,
      description: '',
      status: 'designing',
      sort_order: 0,
      request: {
        params,
        headers: parsed.headers,
        path_variables: [],
        auth: parsed.auth,
        body: parsed.body ?? { mode: 'none' },
        timeout_ms: 30000,
        follow_redirects: true,
        tests: null,
      },
      created_at: now,
      updated_at: now,
    }
    sessionBaseUrl.value = origin
    drafts.value.set(blank.id, blank)
    if (!openTabs.value.includes(blank.id)) openTabs.value.push(blank.id)
    activeTabId.value = blank.id
  }

  // ---------- 请求历史（侧栏「请求历史」页签；发送成功后由编辑器触发刷新） ----------
  const histories = ref<RequestHistory[]>([])
  /** 「仅当前接口」过滤（HistoryPanel 复选框；变更后需重新 loadHistories）。 */
  const historyOnlyCurrent = ref(false)

  async function loadHistories(): Promise<void> {
    if (!project.value) return
    const endpointId = historyOnlyCurrent.value ? activeEndpoint.value?.id ?? null : null
    try {
      histories.value = (await api.listRequestHistories(project.value.id, 50, endpointId)) ?? []
    } catch {
      // 历史为辅助数据，加载失败静默（避免干扰主流程）
    }
  }

  async function clearHistories(): Promise<void> {
    if (!project.value) return
    const endpointId = historyOnlyCurrent.value ? activeEndpoint.value?.id ?? null : null
    try {
      const removed = await api.clearRequestHistories(project.value.id, endpointId)
      histories.value = []
      toast.success(`已清空 ${removed} 条请求历史`)
    } catch (err) {
      toast.error('清空历史失败', {
        message: err instanceof Error ? err.message : String(err),
      })
    }
  }

  /** 历史摘要（request_summary_json）中可恢复的字段。 */
  interface HistorySummary {
    method?: HttpMethod
    url?: string
    spec?: Partial<Endpoint['request']>
  }

  /**
   * 点击历史记录 → 恢复到主编辑器。
   * - 归属接口存在：打开其草稿标签页并回填；不存在（临时请求）：新建「未命名接口」草稿；
   * - 回填 method / path / params / headers / body（摘要值为变量渲染后的实际发送值）；
   * - 认证保留接口自身配置（摘要入库时后端已置空）；URL origin 预填会话 Base URL。
   */
  function restoreFromHistory(h: RequestHistory): void {
    let summary: HistorySummary = {}
    try {
      summary = JSON.parse(h.request_summary_json) as HistorySummary
    } catch {
      // 旧版记录只有 method/url，按降级路径恢复
    }
    const url = summary.url ?? h.url
    const { path, params, origin } = splitUrl(url)

    const target = h.endpoint_id ? endpoints.value.find((e) => e.id === h.endpoint_id) : null
    let id: string
    if (target) {
      openEndpoint(target)
      id = target.id
    } else {
      id = crypto.randomUUID()
      const now = new Date().toISOString()
      drafts.value.set(id, {
        id,
        project_id: project.value?.id ?? '',
        folder_id: null,
        name: '未命名接口',
        method: (summary.method ?? h.method) as HttpMethod,
        path,
        description: '',
        status: 'designing',
        sort_order: 0,
        request: defaultRequestSpec(),
        created_at: now,
        updated_at: now,
      })
      if (!openTabs.value.includes(id)) openTabs.value.push(id)
      activeTabId.value = id
    }
    const draft = drafts.value.get(id)
    if (!draft) return

    draft.method = (summary.method ?? h.method) as HttpMethod
    draft.path = path
    if (origin) sessionBaseUrl.value = origin
    const spec = summary.spec
    draft.request.params = spec?.params?.length ? spec.params : params
    if (spec?.headers) draft.request.headers = spec.headers
    if (spec?.body) draft.request.body = spec.body
    if (spec?.path_variables?.length) draft.request.path_variables = spec.path_variables
    toast.info('已恢复该次请求到编辑器')
  }

  return {
    project,
    folders,
    endpoints,
    environments,
    activeEnvId,
    sessionBaseUrl,
    urlDomain,
    setEnvironmentBaseUrl,
    loadError,
    openTabs,
    activeTabId,
    activeEndpoint,
    isDirty,
    draftOf,
    titleOf,
    init,
    load,
    refresh,
    switchProject,
    openEndpoint,
    openNewEndpoint,
    focusTitleSignal,
    setDraft,
    closeTab,
    saveActiveDraft,
    deleteEndpoint,
    duplicateEndpoint,
    renameEndpoint,
    moveFolder,
    moveEndpoint,
    saveFolder,
    deleteFolder,
    openCurlDraft,
    importEndpoints,
    send,
    loadEnvironments,
    setEnvironment,
    createEnvironment,
    updateEnvironment,
    deleteEnvironment,
    examples,
    loadExamples,
    saveAsExample,
    removeExample,
    histories,
    historyOnlyCurrent,
    loadHistories,
    clearHistories,
    restoreFromHistory,
  }
})

import type { Folder as FolderInput } from '../types/foxApi'
