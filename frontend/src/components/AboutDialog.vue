<script setup lang="ts">
/**
 * AboutDialog：自定义「关于 RustFox」弹窗（替代系统默认 About 面板）。
 * - 品牌 logo（狐狸图标 + 渐变底 + 柔和投影）；
 * - 名称 / 版本 / 副标题 + GitHub / 检查更新链接 + 版权行。
 * 触发来源：macOS 原生菜单「About RustFox」→ rustfox://about 事件 → App.vue 打开。
 */
import { version } from '../../package.json'
import { useToast } from '../composables/useToast'
import Icon from './ui/Icon.vue'
import Modal from './ui/Modal.vue'

defineProps<{ open: boolean }>()
const emit = defineEmits<{ 'update:open': [open: boolean] }>()

const toast = useToast()

const GITHUB_URL = 'https://github.com/weihubeats/ApiRust'

function checkUpdates(): void {
  toast.info(`当前已是最新版本 v${version}`)
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
        <button class="a-link" type="button" @click="checkUpdates">
          <Icon name="refresh" :size="12" /> Check for Updates
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
</style>