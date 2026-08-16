<script setup lang="ts">
/**
 * AboutDialog：自定义「关于 RustFox」弹窗（替代系统默认 About 面板）。
 * - 品牌 logo（狐狸图标 + 渐变底 + 柔和投影）；
 * - 名称 / 版本 / 副标题 + GitHub / 检查更新链接 + 版权行；
 * - 检查更新（tauri-plugin-updater）：发现新版本 → 展示版本号与更新说明 →
 *   下载（带进度）→ 安装 → 重启（tauri-plugin-process relaunch）。
 * 触发来源：macOS 原生菜单「About RustFox」→ rustfox://about 事件 → App.vue 打开。
 */
import { relaunch } from '@tauri-apps/plugin-process'
import { check, type Update } from '@tauri-apps/plugin-updater'
import { ref } from 'vue'
import { version } from '../../package.json'
import { useToast } from '../composables/useToast'
import Icon from './ui/Icon.vue'
import Modal from './ui/Modal.vue'

defineProps<{ open: boolean }>()
const emit = defineEmits<{ 'update:open': [open: boolean] }>()

const toast = useToast()

const GITHUB_URL = 'https://github.com/weihubeats/ApiRust'

const checking = ref(false)
const downloading = ref(false)
/** 下载进度（0-1）；null = 未在下载或总量未知。 */
const progress = ref<number | null>(null)
/** 待安装新版本的展示信息（Update 对象含 JS 私有字段，不能进响应式 ref——
 *  Vue 的 Proxy 会让私有字段访问抛 "Cannot read private member"，故只存字符串）。 */
const pendingVersion = ref<string | null>(null)
const pendingNotes = ref('')
let pending: Update | null = null

async function checkUpdates(): Promise<void> {
  if (checking.value) return
  checking.value = true
  pendingVersion.value = null
  pending?.close()
  pending = null
  try {
    const update = await check()
    if (update?.available) {
      pending = update
      pendingVersion.value = update.version
      pendingNotes.value = update.body ?? ''
    } else {
      update?.close()
      toast.info(`当前已是最新版本 v${version}`)
    }
  } catch (err) {
    toast.error('检查更新失败', {
      message: err instanceof Error ? err.message : String(err),
    })
  } finally {
    checking.value = false
  }
}

async function installUpdate(): Promise<void> {
  const update = pending
  if (!update || downloading.value) return
  downloading.value = true
  progress.value = null
  try {
    let total = 0
    await update.downloadAndInstall((event) => {
      switch (event.event) {
        case 'Started':
          total = event.data.contentLength ?? 0
          progress.value = total > 0 ? 0 : null
          break
        case 'Progress':
          progress.value = total > 0 ? Math.min(event.data.chunkLength / total, 1) : null
          break
        case 'Finished':
          progress.value = 1
          break
      }
    })
    update.close()
    pending = null
    pendingVersion.value = null
    toast.success('更新已安装，即将重启')
    setTimeout(() => relaunch(), 800)
  } catch (err) {
    toast.error('下载更新失败', {
      message: err instanceof Error ? err.message : String(err),
    })
  } finally {
    downloading.value = false
  }
}
</script>

<template>
  <Modal
    :open="open"
    title="关于 RustFox"
    width="380px"
    @update:open="emit('update:open', $event)"
  >
    <div class="about">
      <div class="a-logo" aria-hidden="true">
        <svg width="38" height="38" viewBox="0 0 24 24" fill="none">
          <path d="M13.2 2 4.4 13.6h6.2L9.1 22l8.9-11.6h-6.3L13.2 2z" fill="currentColor" />
        </svg>
      </div>

      <div class="a-title">RustFox</div>
      <div class="a-version">v{{ version }}</div>
      <div class="a-subtitle">High-Performance Native API Testing Suite</div>

      <div class="a-links">
        <a class="a-link" :href="GITHUB_URL" target="_blank" rel="noopener noreferrer">
          <Icon name="globe" :size="12" /> GitHub Repository
        </a>
        <span class="a-dot" aria-hidden="true"></span>
        <button class="a-link" type="button" :disabled="checking" @click="checkUpdates">
          <Icon name="refresh" :size="12" /> {{ checking ? '检查中…' : 'Check for Updates' }}
        </button>
      </div>

      <div v-if="pendingVersion" class="a-update">
        <div class="a-update-title">
          发现新版本 <span class="a-update-version">v{{ pendingVersion }}</span>
        </div>
        <p v-if="pendingNotes" class="a-update-notes">{{ pendingNotes }}</p>
        <div v-if="downloading" class="a-update-progress">
          <div
            class="a-update-progress-bar"
            :style="{ width: progress == null ? '100%' : `${Math.round(progress * 100)}%` }"
            :class="{ indeterminate: progress == null }"
          ></div>
        </div>
        <span v-if="downloading" class="a-update-status">
          {{ progress == null ? '下载中…' : `${Math.round(progress * 100)}%` }}
        </span>
        <button
          v-else
          class="a-update-btn"
          type="button"
          @click="installUpdate"
        >
          下载并安装
        </button>
      </div>

      <div class="a-copyright">© 2026 RustFox Team. Open source under MIT License.</div>
    </div>
  </Modal>
</template>

<style scoped>
.about {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  padding: 6px 4px 2px;
  text-align: center;
}

.a-logo {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 64px;
  height: 64px;
  margin-bottom: 6px;
  color: #fff;
  border-radius: 18px;
  background: linear-gradient(135deg, var(--accent) 0%, var(--put) 100%);
  box-shadow:
    0 10px 24px rgba(168, 85, 247, 0.35),
    inset 0 1px 0 rgba(255, 255, 255, 0.28);
}

.a-title {
  font-size: 18px;
  font-weight: 700;
  color: var(--text-1);
  letter-spacing: 0.2px;
}

.a-version {
  padding: 1px 9px;
  border-radius: 999px;
  font-family: var(--font-mono);
  font-size: 11.5px;
  color: var(--accent);
  background: var(--accent-tint, rgba(168, 85, 247, 0.14));
}

.a-subtitle {
  margin-top: 4px;
  font-size: 12.5px;
  color: var(--text-2);
}

.a-links {
  display: flex;
  align-items: center;
  gap: 10px;
  margin: 14px 0 4px;
}

.a-link {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  border: none;
  background: none;
  padding: 0;
  font-family: inherit;
  font-size: 12.5px;
  color: var(--accent);
  text-decoration: none;
  cursor: pointer;
  transition:
    color var(--dur) var(--ease),
    opacity var(--dur) var(--ease);
}
.a-link:hover {
  color: var(--accent-hover, var(--accent));
  opacity: 0.85;
}
.a-link:focus-visible {
  outline: 2px solid var(--focus-ring);
  outline-offset: 2px;
  border-radius: 4px;
}

.a-dot {
  width: 3px;
  height: 3px;
  border-radius: 50%;
  background: var(--text-3);
  opacity: 0.6;
}

.a-copyright {
  margin-top: 6px;
  font-size: 11px;
  color: var(--text-3);
}

.a-update {
  width: 100%;
  margin-top: 10px;
  padding: 12px;
  border: 1px solid var(--border, rgba(127, 127, 127, 0.25));
  border-radius: 10px;
  background: var(--bg-2, rgba(127, 127, 127, 0.08));
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
}

.a-update-title {
  font-size: 13px;
  color: var(--text-1);
}

.a-update-version {
  font-family: var(--font-mono);
  color: var(--accent);
}

.a-update-notes {
  max-height: 96px;
  overflow-y: auto;
  margin: 0;
  font-size: 12px;
  line-height: 1.5;
  color: var(--text-2);
  white-space: pre-wrap;
  text-align: left;
}

.a-update-progress {
  width: 100%;
  height: 6px;
  border-radius: 999px;
  background: var(--bg-3, rgba(127, 127, 127, 0.2));
  overflow: hidden;
}

.a-update-progress-bar {
  height: 100%;
  border-radius: 999px;
  background: var(--accent);
  transition: width 0.2s ease;
}

.a-update-progress-bar.indeterminate {
  animation: a-update-slide 1.2s ease-in-out infinite;
}

@keyframes a-update-slide {
  0% {
    transform: translateX(-100%);
  }
  100% {
    transform: translateX(100%);
  }
}

.a-update-status {
  font-family: var(--font-mono);
  font-size: 11.5px;
  color: var(--text-2);
}

.a-update-btn {
  padding: 5px 16px;
  border: none;
  border-radius: 7px;
  font-family: inherit;
  font-size: 12.5px;
  color: #fff;
  background: var(--accent);
  cursor: pointer;
  transition: background var(--dur) var(--ease);
}

.a-update-btn:hover {
  background: var(--accent-hover, var(--accent));
}
</style>