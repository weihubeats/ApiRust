<script setup lang="ts">
/**
 * SettingsDialog：设置弹框（Modal 形式，替代独立设置页）。
 * 含备份/恢复与环境管理入口；在仪表板右上角齿轮与左侧导航「设置」处弹出。
 */
import { onMounted, ref } from 'vue'
import { useFoxApi } from '../composables/useFoxApi'
import { useToast } from '../composables/useToast'
import EnvironmentManager from './EnvironmentManager.vue'
import Modal from './ui/Modal.vue'
import type { Project } from '../types/foxApi'

const emit = defineEmits<{ close: [] }>()

const api = useFoxApi()
const toast = useToast()

const project = ref<Project | null>(null)
const busy = ref(false)
const fileInput = ref<HTMLInputElement | null>(null)

const showManager = ref(false)

onMounted(async () => {
  try {
    project.value = (await api.getActiveProject()) ?? null
  } catch {
    project.value = null
  }
  try {
    proxyUrl.value = (await api.getHttpProxy()) ?? ''
  } catch {
    proxyUrl.value = ''
  }
})

// ---------- HTTP 代理 ----------
const proxyUrl = ref('')
const proxyBusy = ref(false)

async function saveProxy(): Promise<void> {
  proxyBusy.value = true
  try {
    await api.setHttpProxy(proxyUrl.value.trim() || null)
    toast.success(proxyUrl.value.trim() ? `代理已设置：${proxyUrl.value.trim()}` : '已切换为直连')
  } catch (err) {
    toast.error('代理设置失败', { message: err instanceof Error ? err.message : String(err) })
  } finally {
    proxyBusy.value = false
  }
}

// ---------- 备份与恢复 ----------
async function exportBackup(): Promise<void> {
  if (!project.value) return
  busy.value = true
  try {
    const text = await api.backupExport(project.value.id)
    const blob = new Blob([text], { type: 'application/json' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    const stamp = new Date().toISOString().slice(0, 10)
    a.href = url
    a.download = `${project.value.name}-备份-${stamp}.json`
    a.click()
    URL.revokeObjectURL(url)
    toast.success('备份已导出')
  } catch (err) {
    toast.error('导出失败', { message: err instanceof Error ? err.message : String(err) })
  } finally {
    busy.value = false
  }
}

async function onImportFile(event: Event): Promise<void> {
  const input = event.target as HTMLInputElement
  const file = input.files?.[0]
  input.value = ''
  if (!file) return
  busy.value = true
  try {
    const text = await file.text()
    const summary = await api.backupRestore(text)
    toast.success(
      `已恢复为「${summary.name}」：接口 ${summary.endpoints} 个、环境 ${summary.environments} 个`,
    )
    emit('close')
  } catch (err) {
    toast.error('导入失败', { message: err instanceof Error ? err.message : String(err) })
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <Modal :open="true" title="设置" width="720px" @close="emit('close')">
    <div class="settings">
      <section class="settings-section">
        <h2 class="rf-subheading">备份与恢复</h2>
        <p class="rf-hint">
          {{ project ? `当前激活项目：${project.name}` : '未选择项目，请先进入任一项目工作区' }}
        </p>
        <div class="settings-actions">
          <button
            class="rf-btn"
            type="button"
            :disabled="!project || busy"
            @click="exportBackup"
          >
            {{ busy ? '处理中…' : '导出当前项目备份' }}
          </button>
          <button class="rf-btn" type="button" :disabled="busy" @click="fileInput?.click()">
            导入备份文件
          </button>
          <input
            ref="fileInput"
            type="file"
            accept=".json,application/json"
            class="settings-file"
            @change="onImportFile"
          />
        </div>
      </section>

      <section class="settings-section">
        <h2 class="rf-subheading">网络代理</h2>
        <p class="rf-hint">
          全局 HTTP 代理，应用于所有请求（如 <code>http://127.0.0.1:7890</code>、
          <code>socks5://host:1080</code>）；留空表示直连。
        </p>
        <div class="settings-proxy">
          <input
            v-model="proxyUrl"
            class="settings-proxy-input"
            type="text"
            placeholder="http://127.0.0.1:7890（留空 = 直连）"
            spellcheck="false"
          />
          <button class="rf-btn" type="button" :disabled="proxyBusy" @click="saveProxy">
            {{ proxyBusy ? '保存中…' : '保存' }}
          </button>
        </div>
      </section>

      <section class="settings-section">
        <h2 class="rf-subheading">环境管理</h2>
        <p class="rf-hint">
          创建 / 编辑 / 删除环境变量，变量经 <code>&#123;&#123;变量&#125;&#125;</code>
          注入请求，可在工作区顶部快速切换。
        </p>
        <div class="settings-actions">
          <button class="rf-btn" type="button" @click="showManager = true">打开环境管理</button>
        </div>
      </section>

      <section class="settings-section">
        <h2 class="rf-subheading">其他</h2>
        <p class="rf-hint">主题、快捷键与数据目录等在后续阶段接入。</p>
      </section>
    </div>

    <EnvironmentManager v-model:open="showManager" />
  </Modal>
</template>

<style scoped>
.settings {
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.settings-section {
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding-top: 16px;
  border-top: 1px solid var(--border);
}

.rf-subheading {
  margin: 0;
  font-size: 13px;
  font-weight: 600;
}

.rf-hint {
  margin: 0;
  font-size: 12.5px;
  color: var(--text-2);
}

.settings-actions {
  display: flex;
  gap: 10px;
  align-items: center;
}

.settings-file {
  display: none;
}

.settings-proxy {
  display: flex;
  gap: 10px;
  align-items: center;
}

.settings-proxy-input {
  flex: 1;
  padding: 7px 10px;
  border: 1px solid var(--border, rgba(127, 127, 127, 0.3));
  border-radius: 8px;
  font-family: var(--font-mono);
  font-size: 12.5px;
  background: var(--bg-2, transparent);
  color: var(--text-1);
}
</style>