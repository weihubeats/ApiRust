# RustFox Tauri 2.0 + Vue 3 迁移架构设计

> 目标:将 UI 层从 Dioxus 0.5 迁移到 Tauri 2.0 + Vue 3 + TailwindCSS,
> 保留全部 Rust 业务逻辑,实现「后端先行、前端逐步替换」的平滑迁移。

## 0. 现状盘点

| 模块 | 说明 | 迁移状态 |
|---|---|---|
| `crates/fox-core` | 模型 + 纯逻辑(变量渲染、cURL 解析) | 无 UI 依赖,直接复用 ✅ |
| `crates/fox-storage` | SQLite 连接 + repository 层 | 无 UI 依赖,直接复用 ✅ |
| `crates/fox-http` | HTTP 请求执行、WS、脚本沙箱 | 无 UI 依赖,直接复用 ✅ |
| `crates/fox-tauri` | Tauri 2 插件(Command 层),独立 workspace | 已建雏形 🟡 |
| `frontend/` | Vue 3 + `useFoxApi` + `foxApi.d.ts`(无构建配置) | 已建雏形 🟡 |
| `crates/fox-desktop` | 旧 Dioxus 桌面应用(3449 行 workspace.rs 等) | 阶段 5 已删除 ✅ |

`fox-tauri` 已实现:Project / Endpoint / Environment 的 CRUD + `execute_request`(变量渲染 → 校验 → 发送),
错误约定 `Result<T, CommandError>` 统一映射为 `{ code, message }`。

---

## 1. 目标架构

```
┌──────────────────────────────────────────────────┐
│ WebView 前端 (Vue 3 + TS + TailwindCSS + Vite)   │
│  ├─ Pinia 状态层 (tabs/drafts/dirty/active... )  │
│  ├─ TanStack Query (服务端状态缓存/失效)          │
│  ├─ 第三方生态: ag-grid / tanstack-table / dnd   │
│  ├─ 工具栏/编辑器/响应区/树 = .vue 组件            │
└───────────────┬──────────────────────────────────┘
                │ Tauri IPC
                │ invoke('cmd', args) → Promise<T>
                │ emit/listen: 后端→前端事件推送
┌───────────────┴──────────────────────────────────┐
│ 应用壳 frontend/src-tauri/ (Tauri 2)              │
│  ├─ main.rs: Builder + fox_tauri::plugin::init() │
│  ├─ tauri.conf.json (窗口/托盘/前端 assets)       │
│  └─ 数据库初始化、日志、窗口状态(复用旧逻辑)       │
└───────────────┬──────────────────────────────────┘
                │ 普通 Rust 函数调用
┌───────────────┴──────────────────────────────────┐
│ fox-tauri 插件 (Command 层, 薄)                   │
│  ├─ AppState: SqlitePool + active 上下文 (RwLock)│
│  ├─ 薄 Command: 直接转发 repo (list_endpoints)   │
│  └─ 厚 Command: 组合业务 (execute_request)        │
└───────────────┬──────────────────────────────────┘
                │
┌───────────────┴──────────────────────────────────┐
│ 纯业务库 (无任何 UI 依赖):                        │
│ fox-core / fox-storage / fox-http / fox-openapi  │
│ fox-oauth / fox-mock / fox-secret / fox-backup   │
│ fox-codegen / fox-test / fox-smoke                │
└──────────────────────────────────────────────────┘
```

### 关键设计决策

1. **分层原则**:`fox-*` 核心库永不依赖 UI 框架。逻辑层与视图层的唯一边界是
   `fox-tauri` 的 Command 签名,该签名即前后端契约(JSON-Serializable)。
2. **泛化壳与插件分离**:应用壳(`src-tauri`)只负责窗口/生命周期/插件装配;
   业务全部落在 `fox-tauri` 插件 crate 里,便于单元测试与复用。
3. **状态分离**:
   - *Rust 侧*(真相源):数据库 + 激活项目/环境上下文(`AppState::active`)。
   - *前端侧*(会话状态):打开的标签、草稿缓存、脏标记、UI 开关等纯视图状态。
4. **类型契约优先**:用 `tauri-specta` 从 Command 声明自动生成 `bindings.ts`,
   避免手工维护 d.ts 漂移(fox-tauri 文档中已有方案 A/B,推荐方案 A)。
5. **事件推送**:后端异步完成(OAuth 回调、压测进度、备份结果)用
   `app.emit` → 前端 `listen`,替代轮询。

### 新增目录结构

```
frontend/                      # 新前端主目录(替换旧 frontend 脚手架内容)
├─ src/
│  ├─ main.ts / App.vue
│  ├─ router/                  # vue-router
│  ├─ stores/                  # Pinia: workspace.ts / project.ts / ui.ts
│  ├─ composables/             # useFoxApi(已有)、useAsync、useDirty...
│  ├─ components/              # EditorTabs / RequestTable / ResponsePanel ...
│  ├─ views/                   # Home / Workspace / Settings / GraphQL ...
│  └─ types/foxApi.d.ts        # 由 specta 生成(或手工镜像)
├─ src-tauri/
│  ├─ src/main.rs              # tauri::Builder + fox_tauri::plugin::init()
│  ├─ tauri.conf.json
│  └─ capabilities/default.json (IPC 权限)
└─ package.json / vite.config.ts / tailwind.config.js
```

---

## 2. Dioxus Hook → Vue 3 / React 依赖映射

> 推荐 Vue 3:现有 `frontend/` 脚手架已是 Vue(frontend/src/composables/useFoxApi.ts 等),零成本复用。

| Dioxus | Vue 3(推荐) | React | 说明 |
|---|---|---|---|
| `use_signal(T)` | `ref(T)` / 全局跨组件 → Pinia store | `useState` / 全局 → Zustand | 组件内状态同理;跨页面共享必须提为全局 store |
| `Signal<HashMap>` / 集合 | `reactive(new Map())` 或 `ref` | `useState` | 注意 Vue 对 Map/Set 需用 `reactive` 包裹才有响应式 |
| `sig.read()` / `sig.peek()` | `sig.value` / 无 peek(渲染只读就是普通访问) | value | Vue 中"不订阅"的读取无需特殊 API |
| `sig.write().insert()` | 直接赋值/`Object.assign` | 不可变更新 | |
| `use_effect(闭包自动收集依赖)` | `watchEffect(自动)` / `watch(显式)` | `useEffect` | 注意时机差异:Dioxus effect 在渲染后,Dioxus 的 dirty 类副作用适合 `watch` |
| `use_future` / `use_resource`(异步加载) | 自己写 `async` + `ref` + `onMounted`,或 TanStack Query `useQuery` | TanStack Query `useQuery` / SWR | 数据 fetch 一律走 Query 库:自动 loading/error/缓存/失效 |
| `spawn(异步任务)` | `await invoke()`(命令天然异步)或 `void fn()` | 同上 | 不再需要 spawn;依赖 hook 外的生命周期用 `onUnmounted` 清理 |
| `use_context` / `use_context_provider` | `provide` / `inject`;全局数据优先 Pinia | `useContext` / Context.Provider | 大部分场景被 store 取代 |
| `memo`(派生值缓存) | `computed` | `useMemo` / `useCallback` | |
| 自定义 Hook(返回 Element+状态) | Composable(`useXxx.ts`) | 自定义 Hooks | `useFoxApi`/`useToast` 已按此模式 |
| `eval()`(JS 互操作:拖拽、resizer) | 直接写 DOM API 或引第三方库 | 同左 | 这是迁移最大收益点之一 |
| 事件处理器 `onclick/oninput` | `@click` / `v-model` / `@input` | `onClick`(受控组件) | |
| RSX 条件渲染 `if cond { }` | `v-if` / `v-show` | `{cond && ...}` / 三目 | |
| 列表渲染 `for node in nodes` | `v-for` :key | `items.map(...)` key | |
| `use_router` / 页面枚举 | `vue-router` | `react-router` | 现状用 `Page` 枚举,迁移期可先保留枚举 + 顶部导航 |
| 拖拽树 | `vuedraggable` / 原生 HTML5 DnD | `dnd-kit` | 替换 Dioxus 的 eval+JS 双向通信 |
| 复杂表格/单元格编辑 | `ag-grid-vue` 或自行封装 | `ag-grid-react` | 替换手写 KV 表格 |
| 代码编辑器 | `Monaco Editor`(vscode 同源) | 同左 | 替换 dioxus textarea |
| 主题/样式 | TailwindCSS + CSS 变量(移植 `styles.rs` 的 `--rf-*` 令牌) | 同左 | |

### 迁移要点(状态管理)

Dioxus 的全局 `Signal` 本质是"模块级单例响应式状态",对应前端的最自然位置是 **Pinia store**:

| 现有 Dioxus 状态 | 迁移到 |
|---|---|
| `AppState.endpoints/folders/projects/environments`(服务端数据) | TanStack Query + 失效重取(Pinia 只存"当前激活"引用) |
| `open_tabs / active_endpoint_id / tab_drafts / dirty` | Pinia `workspace` store |
| `draft`(当前编辑草稿) | Pinia `workspace.draft`(与标签切换联动) |
| `toasts` | Pinia `ui.store`(已有 `useToast` composable) |
| `pending_save / save_name` | Pinia `workspace`(弹窗状态) |
| 命令开关(curl_open / codegen_open) | Pinia `ui` store 或组件局部状态 |

---

## 3. 代码转换示例:标签页 + 脏标记 + 草稿切换

以 `fox-desktop/src/pages/workspace.rs` 的 M15 标签/草稿/脏标记流程为例
(此前刚修过"黄点不即时出现"bug,逻辑已定型)。

### 3.1 Dioxus 原版(精选)

```rust
// state.rs —— 打开标签
pub fn open_endpoint_tab(&self, endpoint_id: Uuid) {
    let mut page = self.current_page;
    let mut active = self.active_endpoint_id;
    let mut tabs = self.open_tabs;
    active.set(Some(endpoint_id));
    page.set(Page::Workspace);
    if !tabs.peek().contains(&endpoint_id) {
        tabs.write().push(endpoint_id);
    }
}

// workspace.rs —— 活动标签切换草稿(写回缓存 → 拾取新草稿)
let tab_drafts: Signal<HashMap<Uuid, Endpoint>> = use_signal(HashMap::new);
use_effect(move || {
    let active = *st.active_endpoint_id.read();
    if let Some(ep) = d.peek().clone() {           // 旧草稿写回缓存
        tds.write().insert(ep.id, ep);
    }
    match active {
        Some(id) => {
            if d.peek().as_ref().map(|e| e.id) != Some(id) {
                let ep = tds.peek().get(&id).cloned()
                    .or(unsaved).or_else(|| {      // 缓存 → 未落库 → 数据库
                        st.endpoints.read().iter().find(|e| e.id == id)
                    });
                d.set(ep);
            }
        }
        None => d.set(None),
    }
});

// workspace.rs —— 草稿变化 → 缓存 + 脏标记对比
use_effect(move || {
    let Some(ep) = d.read().clone() else { return; };
    tds.write().insert(ep.id, ep.clone());
    let saved = st.endpoints.read().iter().find(|e| e.id == ep.id).cloned();
    match saved {
        Some(s) if eq_ignoring_updated_at(&s, &ep) => dirty.write().remove(&ep.id),
        _ => dirty.write().insert(ep.id),
    }
});

// workspace.rs —— Tab 栏渲染(标题:草稿名 → 已保存名 → 「接口」兜底)
let tab_infos = tab_ids.iter().map(|id| {
    let title = tab_drafts.read().get(id)
        .or_else(|| saved_endpoints.iter().find(|e| e.id == *id))
        .map(|e| if e.name.trim().is_empty() {
            format!("{} {}", e.method, e.path)
        } else { e.name.clone() })
        .unwrap_or_else(|| "接口".into());
    (*id, title, dirty_flag.contains(id))
}).collect();
```

### 3.2 Vue 3 重写

**`frontend/src/stores/workspace.ts`(Pinia 取代 Signal + effect):**

```ts
import { defineStore } from 'pinia'
import { computed, reactive, ref } from 'vue'
import { useFoxApi } from '../composables/useFoxApi'
import type { Endpoint } from '../types/foxApi'

/** 忽略 updated_at 比较(与 Rust 侧 eq_ignoring_updated_at 对应)。 */
function eqIgnoringUpdatedAt(a: Endpoint, b: Endpoint): boolean {
  return JSON.stringify({ ...a, updated_at: null }) === JSON.stringify({ ...b, updated_at: null })
}

export const useWorkspaceStore = defineStore('workspace', () => {
  const api = useFoxApi()

  // 对应 Signal: open_tabs / active_endpoint_id / tab_drafts / dirty
  const openTabs = ref<string[]>([])
  const activeId = ref<string | null>(null)
  const drafts = reactive(new Map<string, Endpoint>()) // 标签草稿缓存
  const dirtyIds = ref(new Set<string>())              // 脏标记
  const savedById = ref(new Map<string, Endpoint>())   // 已保存副本(供脏标记对比)

  const draft = computed(() => activeId.value ? drafts.get(activeId.value) ?? null : null)

  // 取代 open_endpoint_tab:状态更新 + 通过 IPC 懒加载草稿
  async function openEndpointTab(id: string) {
    activeId.value = id
    if (!openTabs.value.includes(id)) openTabs.value.push(id)
    if (!drafts.has(id)) {
      try {
        drafts.set(id, await api.getEndpoint(id))       // invoke('get_endpoint')
        savedById.value.set(id, drafts.get(id)!)
      } catch (e) {
        toast.error('接口加载失败', { message: (e as Error).message })
      }
    }
  }

  function closeTab(id: string) {
    const idx = openTabs.value.indexOf(id)
    if (idx >= 0) openTabs.value.splice(idx, 1)
    drafts.delete(id); dirtyIds.value.delete(id); savedById.value.delete(id)
    if (activeId.value === id) activeId.value = openTabs.value.at(-1) ?? null
  }

  /** 编辑草稿任意字段时调用:更新缓存 + 重算脏标记(取代 Dioxus 的 use_effect)。 */
  function updateDraft(patch: Partial<Endpoint>) {
    const cur = draft.value
    if (!cur) return
    const next = { ...cur, ...patch }
    drafts.set(next.id, next)
    const saved = savedById.value.get(next.id)
    const dirty = !saved || !eqIgnoringUpdatedAt(saved, next)
    if (dirty) dirtyIds.value.add(next.id) else dirtyIds.value.delete(next.id)
  }

  // 标签标题:草稿名 → 已保存名 → 兜底(与 Rust 渲染一致)
  const tabInfos = computed(() => {
    const epOf = (id: string) => drafts.get(id) ?? savedById.value.get(id)
    return openTabs.value.map((id) => {
      const ep = epOf(id)
      return {
        id,
        title: ep?.name?.trim() || `${ep?.method ?? 'GET'} ${ep?.path ?? '/'}`,
        dirty: dirtyIds.value.has(id),
        active: activeId.value === id,
      }
    })
  })

  return { openTabs, activeId, draft, tabInfos, openEndpointTab, closeTab, updateDraft, drafts }
})
```

**`frontend/src/components/EditorTabs.vue`:**

```vue
<script setup lang="ts">
import { useWorkspaceStore } from '../stores/workspace'
const ws = useWorkspaceStore()
</script>

<template>
  <div class="tab-bar flex items-end gap-1 bg-panel-2">
    <div
      v-for="tab in ws.tabInfos"
      :key="tab.id"
      class="editor-tab flex items-center gap-2 px-3 py-2 cursor-pointer"
      :class="{ active: tab.active }"
      @click="ws.openEndpointTab(tab.id)"
    >
      <span>{{ tab.title }}</span>                <!-- 首次点击即正确名称 -->
      <span v-if="tab.dirty" class="text-amber-500 text-xs">●</span>
      <button class="tab-close opacity-0 group-hover:opacity-100" @click.stop="ws.closeTab(tab.id)">×</button>
    </div>
  </div>
</template>
```

### 3.3 前端通过 `invoke` 调用后端(发送请求示例)

```ts
// useFoxApi.ts 已封装:call() = invoke + 错误映射(+DECRYPT 弹窗)
async function send() {
  const ep = ws.draft.value
  if (!ep) return
  sending.value = true                      // 取代 Dioxus 的 sending Signal
  try {
    response.value = await api.executeRequest({   // → invoke('execute_request')
      url: ep.path,
      method: ep.method,
      spec: ep.request,                     // RequestSpec 与 Rust 侧 JSON 对齐
      environment_id: activeEnvironmentId.value,
    })
    // 记录历史/示例 → invoke('save_test_history') ...
  } catch (e) {
    toast.error('请求失败', { message: (e as Error).message })
  } finally {
    sending.value = false
  }
}
```

**对照要点:**
- `use_signal` 状态 → `ref`/`reactive` + Pinia;Dioxus 的 effect 自动去抖/顺序差异在
  Vue 中由 `watch(显式依赖)` 精确控制,不再有"黄点不即时出现"这类订阅遗漏。
- 异步不再用 `spawn` + 状态闭包,直接 `await invoke()`,loading/错误在组件内即可;
  需要取消/重试的列表请求交给 TanStack Query。
- 拖拽树(Dioxus eval + DropMessage)换成 `vuedraggable`;复杂 KV/表格换
  `ag-grid-vue`;json 编辑换 Monaco——生态问题直接解决。

---

## 4. 迁移路线图(后端先行,前端逐步替换)

### 阶段 0:脚手架(已完成 ⏳)

`fox-tauri` 插件雏形 + Vue 3 前端雏形 + 统一错误约定。**无构建配置、无 Tauri 壳**。

### 阶段 1:后端先行——补全 Command 覆盖 ✅ 目标:前端可调全部能力

- [x] 补齐命令:文件夹 CRUD(`list_folders`/`save_folder`/`delete_folder`)、cURL 导入(`parse_curl_command`)
- [x] OAuth2 授权流:`oauth_authorize`(本地回调 9090 + 系统浏览器 + 换 token)、`oauth_access_token`(缓存/静默刷新);
      代码生成 `codegen_render`(curl/python/js/go/java/php)——命令共 25 个
- [ ] 补齐命令(待做):搜索、`openapi/postman/swagger2` 导入导出
- [ ] 补齐服务型命令:`fox-mock`(起停服务)、`fox-test`(断言/压测)、
      `fox-backup`/`fox-secret`、`fox-smoke`
- [ ] 事件命令:`execute_request` 之外增加流式进度(`app.emit('load-progress')`)
- [ ] 接入 `tauri-specta` 生成 `bindings.ts`;`fox-core::model` 上加 `#[cfg_attr(feature="specta", derive(specta::Type))]`
- [x] 建应用壳:`frontend/src-tauri/`(main.rs 装配插件、tauri.conf.json、capabilities)——`cargo check` 通过,
      权限标识统一为 `fox:*`(插件 `links = "fox"`,capabilities 配 `fox:default`,invoke 前缀 `plugin:fox|`)
- [x] 建构建链:Vite + `@tauri-apps/cli` + TailwindCSS;`vue-tsc` + `vite build` 通过
- **验收**:`cargo test`(fox-tauri)全绿;空壳启动显示 Vue 首页;Rust 侧零 UI 依赖。
      壳的 `npm run tauri dev` 实际启动留待接线机验证(需 webkit2gtk 运行环境)。

### 阶段 2:前端骨架——Home + 项目/接口树 + 路由

- [x] Pinia stores(`workspace` 已建:project/folders/endpoints/标签草稿);TanStack Query 暂不引入(useFoxApi 已足够)
- [x] `styles.rs` 的 `--rf-*` 设计令牌 → 全局 CSS 变量(frontend/src/style.css,深色默认 + data-theme 覆盖)
- [x] 通用组件:Toast / Progress 全局挂载;Modal(CurlImportDialog);Confirm 暂用 window.confirm
- [x] 路由:`Home` → `ProjectList` → `/workspace`(树 + 编辑器)
- [x] 项目/文件夹/接口列表 + CRUD 全流程(对应 `project_tree.rs`):文件夹递归树 + 行内新建/重命名/删除;
      接口新建/重命名/复制/删除;cURL 导入(parse_curl_command,目标文件夹可选)
- [x] 拖拽排序/移动:HTML5 DnD,落文件夹内 / 兄弟之前 / 根末尾;重排 sort_order 后落库
      (复用 save_folder/save_endpoint,列表按 `ORDER BY sort_order` 读取)
- **验收**:树中可新建/重命名/删除/拖拽接口;点击接口打开系统外占位标签页。✓

### 阶段 3:编辑器核心——最高价值优先

- [x] Tab 栏 + 草稿缓存 + 脏标记(树中 ●)+ 保存(Ctrl+S)——已按 §3 示例实现
- [x] 请求编辑区:方法/路径/名称/Params/Headers(键值行)/Body(none·JSON·Text·GraphQL·表单·多部件)/Auth(none·Bearer·Basic·APIKey·OAuth2 配置)
- [x] 发送请求 + 响应区(状态码/耗时/大小/正文,JSON 美化,Ctrl+Enter)——走 `execute_request`;环境选择器
      (list/set_active_environment,后端按 environment_id 注入变量)已接;新建环境(名称)已支持
- [x] 响应示例:新增 `list_examples`/`save_example`/`delete_example` 命令(22 个),前端「保存为示例」+ 示例列表/查看/删除
- [x] OAuth2 授权流:`oauth_authorize`(编辑器「立即授权」→ 系统浏览器 → 回调 9090 → 令牌写入草稿)+
      发送前 `oauth_access_token` 取/刷新令牌(共 25 个命令);编辑器「生成代码」区(generate_code 6 语言 + 复制)
- [x] 历史记录持久化:`execute_request` 成功后自动落 `request_histories` + `list_request_histories`(26 个命令);
      编辑器「请求历史」面板(方法/URL/状态/耗时/时间)
- [x] Mock 服务:`mock_start`(4010 起自动探测端口,启用规则 + 接口示例生成定义)/ `mock_stop` / `mock_status`
      (29 个命令);工作区顶部开关 + 运行地址显示
- [x] 备份/恢复:`backup_export`(项目全量 JSON,设置页导出下载)/ `backup_restore`(格式校验 + UUID 全量重映射为
      新项目落库,31 个命令);设置页导入文件恢复
- [x] 导入导出:`import_document`(OpenAPI 3.0 / Swagger 2.0 / Postman v2.1 自动识别,预览后确认落库,
      按 folder_hint 建夹 + 示例一并导入)/ `export_openapi`(项目导出 OpenAPI 3.0 JSON);工作区顶部按钮 + 导入对话框
- [x] 测试/压测:`test_endpoint`(编辑器断言 JSON 编辑 + 运行,outcomes 明细展示)/ `load_test`(并发/总数,
      汇总:成功率/耗时/分位/RPS)——命令 35 个
- [x] GraphQL 视图接线:`/graphql` 独立路由可用(props 可选 + 环境回退工作区激活环境),工作区/首页均有入口
- [x] 窗口状态记忆:壳接入 `tauri-plugin-window-state`(位置/尺寸/最大化持久化)
- [x] 事件推送:`load_test` 经 `fox:load-progress` 事件实时推送进度(后端 `AppHandle` 接入,插件改非泛型 Wry;
      `run_load` 增可选进度回调,调用方 `None` 兼容);编辑器压测区进度条展示
- **验收**:日常调试闭环(编辑 → 发送 → 看响应 → 存示例)在 Vue 下完整可用;
  与 Dioxus 版功能对齐(可对拍)。

### 阶段 4:高级功能对齐

- [x] 打包脚本与 CI:`scripts/package-tauri.sh`(npm ci + tauri build,bundle 产物)、release.yml 改 Tauri 流程
      (node 22 + rust-cache + 平台 bundle 上传,打 `v*` tag 触发)
- [x] 事件推送(压测进度经 tauri 事件)——阶段 3 落地;smoke test(fox-smoke 集成测试仓,随主仓 cargo test)
- [x] 导入导出(OpenAPI/Postman/Swagger2)、压测/断言(`fox-test`)、备份/恢复(`fox-backup`)、Mock 服务(`fox-mock`)
      ——已随阶段 3 落地
- [x] OAuth2 授权流(编辑器内授权按钮)、代码生成(编辑器内多语言 + 复制)——已随阶段 3 落地
- [x] GraphQL 视图(`/graphql` 独立路由,环境回退工作区激活环境)
- [x] 窗口状态记忆(`tauri-plugin-window-state`)
- [x] 环境变量编辑(设置页)与 Mock 规则管理(工作区对话框)对齐 Dioxus 设置页能力

### 阶段 5:收尾

- [x] 双壳并存期间的回归对拍(对拍清单见下;同一 SQLite 文件与数据目录,零 schema 变更)
- [x] 删除 `crates/fox-desktop`;移除 dioxus/gtk/webkit 依赖树(root workspace members + `dioxus` workspace dep)
- [x] CI 简化:去掉 `libwebkit2gtk/gtk` 安装步骤(见 `.github/workflows/ci.yml`),
      release 换 `scripts/package-tauri.sh`(tauri build)
- [ ] 性能/体积核对(WebView vs GTK 内存占用、启动时间)——Dioxus 版已移除,此项归档

### 迁移期间保障(始终可编译可运行)

1. **并行双壳**(已结束):`crates/fox-desktop`(Dioxus)与 `frontend/src-tauri`(Tauri)共存,
   `fox-*` 核心被两者共用;data 目录与 SQLite 文件完全一致,随时可回滚。阶段 5 已移除旧壳。
2. **workspace 隔离**:`fox-tauri` 与 `src-tauri` 保持独立 `[workspace]`(或新加成员),
   不拖慢主仓 `cargo check --workspace`;CI 分两个 job,互不阻塞。
3. **契约先行**:每个阶段先定 `bindings.ts`(specta),前端 mock 可在没有壳时用
   `@tauri-apps/api/mocks` 或简单 fetch stub 开发。
4. **小步验收**:每个阶段都产出可运行 demo,不搞"大爆炸"切换。

### 风险与对策

| 风险 | 对策 |
|---|---|
| invoke 大对象 JSON 序列化开销(大响应体) | 响应体走 `channel`/分块或 `tauri::ipc::Response` 流式返回 |
| WebView 性能(复杂表格) | ag-grid 虚拟滚动;必要时 `webview2`/`wry` 可行优化 |
| 双壳并存期维护成本 | 阶段 2 即冻结 Dioxus 新功能,仅修 bug;阶段 5 一次性删除 |
| 状态语义差异(use_effect 自动重跑 vs watch) | 关键副作用显式 `watch` + 代码评审清单 |
| 旧 d.ts 与 Rust 模型漂移 | specta 自动导出为唯一真相源 |

---

## 5. 结论

现有代码已具备迁移的 90% 条件:核心库零 UI 依赖、`fox-tauri` 命令层雏形、Vue 3 封装已成。
剩余工作集中在前端工程化(Vite/Tailwind/Pinia)、命令补齐和应用壳装配。
建议顺序:**阶段 1(后端补全)→ 阶段 2(骨架)→ 阶段 3(编辑器核心)**,阶段 3 完成后即可
向用户发布双壳并行版本,再进入阶段 4-5 收尾。