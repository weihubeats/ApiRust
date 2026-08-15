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
import type {
  AuthSpec,
  CurlParsed,
  Endpoint,
  Environment,
  ExecuteResponse,
  Folder,
  OAuth2Token,
  Project,
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

  /** 打开「新建接口」草稿标签页（未持久化，保存时生成 id）。 */
  function openNewEndpoint(folderId: string | null): void {
    const now = new Date().toISOString()
    const blank: Endpoint = {
      id: crypto.randomUUID(),
      project_id: project.value?.id ?? '',
      folder_id: folderId,
      name: '',
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

  /** 发送草稿请求（url 为拼接后的完整地址；环境变量由后端按 environment_id 注入）。 */
  async function send(endpoint: Endpoint, url: string): Promise<ExecuteResponse> {
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
    const changed = new Set<string>()
    const mark = (f: Folder): void => {
      changed.add(f.id)
    }
    const renumber = (list: Folder[], insertedIndex: number, skipId: string): void => {
      list
        .filter((f) => f.id !== skipId)
        .forEach((f, i) => {
          const order = i < insertedIndex ? i : i + 1
          if (f.sort_order !== order) {
            mark(f)
            f.sort_order = order
          }
        })
    }
    if (moved.parent_id === newParentId) {
      const group = folders.value.filter((f) => f.parent_id === newParentId)
      renumber(group, Math.min(targetIndex, group.length - 1), folderId)
      moved.sort_order = Math.min(targetIndex, group.length - 1)
      changed.add(folderId)
    } else {
      renumber(folders.value.filter((f) => f.parent_id === moved.parent_id), 0, folderId)
      moved.parent_id = newParentId
      const group = folders.value.filter((f) => f.parent_id === newParentId)
      renumber(group, Math.min(targetIndex, group.length), folderId)
      moved.sort_order = Math.min(targetIndex, group.length)
      changed.add(folderId)
    }
    await Promise.all(
      [...changed].map((id) => {
        const f = folders.value.find((x) => x.id === id)
        return f ? api.saveFolder({ ...f }) : Promise.resolve()
      }),
    )
    await refresh()
  }

  /** 将接口移动到 folderId（null=根）的 targetIndex 处：重排相关兄弟组并落库。 */
  async function moveEndpoint(endpointId: string, newFolderId: string | null, targetIndex: number): Promise<void> {
    const moved = endpoints.value.find((e) => e.id === endpointId)
    if (!moved) return
    const changed = new Set<string>()
    const renumber = (list: Endpoint[], insertedIndex: number, skipId: string): void => {
      list
        .filter((e) => e.id !== skipId)
        .forEach((e, i) => {
          const order = i < insertedIndex ? i : i + 1
          if (e.sort_order !== order) {
            changed.add(e.id)
            e.sort_order = order
          }
        })
    }
    if (moved.folder_id === newFolderId) {
      const group = endpoints.value.filter((e) => e.folder_id === newFolderId)
      renumber(group, Math.min(targetIndex, group.length - 1), endpointId)
      moved.sort_order = Math.min(targetIndex, group.length - 1)
      changed.add(endpointId)
    } else {
      renumber(endpoints.value.filter((e) => e.folder_id === moved.folder_id), 0, endpointId)
      moved.folder_id = newFolderId
      const group = endpoints.value.filter((e) => e.folder_id === newFolderId)
      renumber(group, Math.min(targetIndex, group.length), endpointId)
      moved.sort_order = Math.min(targetIndex, group.length)
      changed.add(endpointId)
    }
    await Promise.all(
      [...changed].map((id) => {
        const e = endpoints.value.find((x) => x.id === id)
        return e ? api.saveEndpoint({ ...e }) : Promise.resolve()
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

  /** cURL 导入落地：解析结果 → 新接口（写入目标文件夹）→ 打开标签页。 */
  async function createFromCurl(
    parsed: CurlParsed,
    folderId: string | null,
    name: string,
  ): Promise<void> {
    let path = parsed.url
    if (!path.startsWith('/')) {
      try {
        path = new URL(parsed.url).pathname
      } catch {
        path = `/${path}`
      }
    }
    const now = new Date().toISOString()
    const endpoint: Endpoint = {
      id: crypto.randomUUID(),
      project_id: project.value?.id ?? '',
      folder_id: folderId,
      name,
      method: parsed.method,
      path,
      description: '',
      status: 'designing',
      sort_order: 0,
      request: {
        params: [],
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
    const saved = await api.saveEndpoint(endpoint)
    endpoints.value.push(saved)
    openEndpoint(saved)
    toast.success(`已导入接口：${saved.name}`)
  }

  return {
    project,
    folders,
    endpoints,
    environments,
    activeEnvId,
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
    openEndpoint,
    openNewEndpoint,
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
    createFromCurl,
    importEndpoints,
    send,
    loadEnvironments,
    setEnvironment,
    createEnvironment,
    examples,
    loadExamples,
    saveAsExample,
    removeExample,
  }
})

import type { Folder as FolderInput } from '../types/foxApi'
