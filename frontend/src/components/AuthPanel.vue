<script setup lang="ts">
/**
 * AuthPanel：认证配置面板（none / bearer / basic / apikey / oauth2）。
 * OAuth2 授权流：后端起本地回调 + 打开系统浏览器，完成后令牌写入草稿。
 */
import { computed, ref } from 'vue'
import { useFoxApi } from '../composables/useFoxApi'
import { useToast } from '../composables/useToast'
import CustomSelect from './ui/CustomSelect.vue'
import type { ApiKeyLocation, AuthSpec, Endpoint, OAuth2Token } from '../types/foxApi'

const props = defineProps<{ draft: Endpoint | null }>()

const api = useFoxApi()
const toast = useToast()

const AUTH_TYPES: Array<{ value: string; label: string }> = [
  { value: 'none', label: '无认证' },
  { value: 'bearer', label: 'Bearer Token' },
  { value: 'basic', label: 'Basic' },
  { value: 'apikey', label: 'API Key' },
  { value: 'oauth2', label: 'OAuth2' },
]
const AUTH_IN_OPTIONS = [
  { value: 'header', label: 'Header' },
  { value: 'query', label: 'Query' },
]

/** Auth 编辑区；type 切换时替换为对应默认对象。所有分支字段统一为可选项。 */
type EditableAuth = AuthSpec & {
  token?: OAuth2Token
  key?: string
  value?: string
  in?: ApiKeyLocation | string
  username?: string
  password?: string
  client_id?: string
  client_secret?: string
  auth_url?: string
  token_url?: string
  scope?: string
  redirect_uri?: string
}

const authAny = computed(() => props.draft?.request.auth as EditableAuth)
const authorizing = ref(false)

/** OAuth2 授权状态文案。 */
const oauthStatus = computed(() => {
  const token = authAny.value?.token as
    | { access_token?: string; expires_at?: string }
    | undefined
  if (!token?.access_token) return '未授权'
  const expires = token.expires_at ? new Date(token.expires_at) : null
  const expiring = expires ? expires.getTime() - Date.now() < 5 * 60_000 : false
  return expires && !expiring
    ? `已授权，有效期至 ${expires.toLocaleString('zh-CN')}`
    : expiring
      ? '令牌即将过期，发送时将自动刷新'
      : '已授权（发送时自动刷新）'
})

/** 发起完整授权流：后端起本地回调 + 打开系统浏览器；完成后令牌写入草稿。 */
async function oauthAuthorize(): Promise<void> {
  if (!props.draft) return
  authorizing.value = true
  try {
    const token = await api.oauthAuthorize(authAny.value as AuthSpec)
    const req = props.draft.request
    req.auth = { ...authAny.value, token } as AuthSpec
    toast.success('OAuth2 授权成功，请保存 (⌘S) 持久化')
  } catch (err) {
    toast.error('OAuth2 授权失败', { message: err instanceof Error ? err.message : String(err) })
  } finally {
    authorizing.value = false
  }
}

function setAuthType(type: string): void {
  const req = props.draft?.request
  if (!req) return
  switch (type) {
    case 'none':
      req.auth = { type: 'none' }
      break
    case 'bearer':
      req.auth = { type: 'bearer', token: '' }
      break
    case 'basic':
      req.auth = { type: 'basic', username: '', password: '' }
      break
    case 'apikey':
      req.auth = { type: 'apikey', key: '', value: '', in: 'header' }
      break
    case 'oauth2':
      req.auth = {
        type: 'oauth2',
        client_id: '',
        client_secret: '',
        auth_url: '',
        token_url: '',
        scope: '',
        redirect_uri: '',
      }
      break
  }
}
</script>

<template>
  <div class="panel">
    <CustomSelect
      :model-value="authAny?.type ?? 'none'"
      :options="AUTH_TYPES"
      size="sm"
      class="auth-type-select"
      @update:model-value="setAuthType(String($event))"
    />
    <div v-if="authAny?.type === 'bearer'" class="kv-row">
      <input
        v-model="authAny.token"
        class="rf-input rf-input-sm kv-value"
        placeholder="Token"
        spellcheck="false"
      />
    </div>
    <div v-else-if="authAny?.type === 'basic'" class="kv-row">
      <input
        v-model="authAny.username"
        class="rf-input rf-input-sm kv-key"
        placeholder="用户名"
      />
      <input
        v-model="authAny.password"
        class="rf-input rf-input-sm kv-value"
        placeholder="密码"
        type="password"
      />
    </div>
    <div v-else-if="authAny?.type === 'oauth2'" class="oauth-form">
      <p class="oauth-hint">
        <span class="oauth-status" :class="{ ok: oauthStatus !== '未授权' }">{{ oauthStatus }}</span>
        <button
          class="rf-btn rf-btn-sm"
          type="button"
          :disabled="authorizing"
          @click="oauthAuthorize"
        >
          {{ authorizing ? '授权中…' : '立即授权' }}
        </button>
      </p>
      <div class="kv-row">
        <input v-model="authAny.client_id" class="rf-input rf-input-sm kv-key" placeholder="Client ID" />
        <input v-model="authAny.client_secret" class="rf-input rf-input-sm kv-value" placeholder="Client Secret" type="password" />
      </div>
      <div class="kv-row">
        <input v-model="authAny.auth_url" class="rf-input rf-input-sm kv-key" placeholder="Authorize URL" />
        <input v-model="authAny.token_url" class="rf-input rf-input-sm kv-value" placeholder="Token URL" />
      </div>
      <div class="kv-row">
        <input v-model="authAny.scope" class="rf-input rf-input-sm kv-key" placeholder="Scope（空格分隔）" />
        <input v-model="authAny.redirect_uri" class="rf-input rf-input-sm kv-value" placeholder="Redirect URI" />
      </div>
    </div>
    <div v-else-if="authAny?.type === 'apikey'" class="kv-row">
      <input v-model="authAny.key" class="rf-input rf-input-sm kv-key" placeholder="Key" />
      <input
        v-model="authAny.value"
        class="rf-input rf-input-sm kv-value"
        placeholder="Value"
        spellcheck="false"
      />
      <CustomSelect v-model="authAny.in" :options="AUTH_IN_OPTIONS" size="sm" class="auth-in-select" />
    </div>
  </div>
</template>

<style scoped>
.panel {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.kv-row {
  display: flex;
  gap: 6px;
  align-items: center;
}

.kv-key {
  width: 220px;
}

.kv-value {
  flex: 1;
}

.auth-type-select {
  width: 160px;
}

.auth-in-select {
  width: 100px;
}

.oauth-hint {
  margin: 0;
  display: flex;
  align-items: center;
  gap: 10px;
}

.oauth-status {
  font-size: 12px;
  color: var(--text-3);
}

.oauth-status.ok {
  color: var(--success);
}
</style>