<script setup lang="ts">
/**
 * EnvironmentBar：环境选择器（工作区顶部栏右侧）。
 * - 触发框仅显示 状态色点 + 环境名；悬停 tooltip 展示完整 Base URL；
 * - 下拉项两行：环境名 + 小字 Base URL；末项「管理环境…」（顶部分隔线）打开 EnvironmentManager；
 * - 👁️（环境变量速览）与下拉融为一个边线相连的组控（eb-group）。
 */
import { computed, ref } from 'vue'
import { useWorkspaceStore } from '../stores/workspace'
import { envBaseUrl, envColorClass } from '../utils/environment'
import CustomSelect from './ui/CustomSelect.vue'
import EnvironmentManager from './EnvironmentManager.vue'
import EnvironmentQuickView from './EnvironmentQuickView.vue'
import IconButton from './ui/IconButton.vue'
import Tooltip from './ui/Tooltip.vue'
import type { Environment } from '../types/foxApi'

const MANAGE_VALUE = '__manage__'

const store = useWorkspaceStore()

const barEl = ref<HTMLElement | null>(null)
const showQuick = ref(false)
const showManager = ref(false)

const activeEnv = computed(
  () => store.environments.find((e) => e.id === store.activeEnvId) ?? null,
)

const options = computed(() => [
  { value: '', label: '无环境' },
  ...store.environments.map((env) => ({ value: env.id, label: env.name })),
  { value: MANAGE_VALUE, label: '管理环境…' },
])

/** 悬停 tooltip：完整 Base URL（无环境 / 未配置时不显示）。 */
const tooltipContent = computed(() => (activeEnv.value ? envBaseUrl(activeEnv.value) : ''))

function envByValue(value: string | number): Environment | null | undefined {
  return store.environments.find((e) => e.id === String(value))
}

function onChange(value: string | number): void {
  if (value === MANAGE_VALUE) {
    showManager.value = true
    return
  }
  void store.setEnvironment(value === '' ? null : String(value))
}

function colorClass(name: string): string {
  return envColorClass(name)
}
</script>

<template>
  <div ref="barEl" class="env-bar">
    <div class="eb-group">
      <Tooltip :content="tooltipContent" placement="bottom">
        <CustomSelect
          class="eb-select"
          pop-class="env-pop"
          :model-value="store.activeEnvId ?? ''"
          :options="options"
          placeholder="环境：无"
          size="sm"
          @change="onChange"
        >
          <template #display="{ selected }">
            <span class="edot" :class="`ed-${colorClass(selected?.label ?? '')}`"></span>
            <span class="env-display-name">{{ selected?.label ?? '无环境' }}</span>
          </template>
          <template #option="{ option }">
            <template v-if="option.value === MANAGE_VALUE">
              <span class="env-manage">管理环境…</span>
            </template>
            <template v-else>
              <span class="env-opt-name">
                <span class="edot" :class="`ed-${colorClass(option.label)}`"></span>
                <span class="env-opt-name-text">{{ option.label }}</span>
              </span>
              <span v-if="envBaseUrl(envByValue(option.value))" class="env-opt-url">
                {{ envBaseUrl(envByValue(option.value)) }}
              </span>
            </template>
          </template>
        </CustomSelect>
      </Tooltip>
      <span class="eb-sep" aria-hidden="true"></span>
      <Tooltip :content="activeEnv ? '查看当前环境变量' : '没有激活的环境'" placement="bottom">
        <IconButton
          class="eb-eye"
          name="eye"
          :size="14"
          :disabled="!activeEnv"
          title="环境变量速览"
          @click="showQuick = !showQuick"
        />
      </Tooltip>
    </div>

    <EnvironmentQuickView
      v-if="showQuick && activeEnv"
      :anchor="barEl"
      @close="showQuick = false"
      @manage="showQuick = false; showManager = true"
    />
    <EnvironmentManager v-model:open="showManager" />
  </div>
</template>

<style scoped>
.env-bar {
  display: inline-flex;
  align-items: center;
  flex-shrink: 0;
}

/* ---- 统一 32px 边线相连组控：[下拉 | 👁️] ---- */
.eb-group {
  display: inline-flex;
  align-items: center;
  height: 32px;
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 8px;
  background: var(--bg-hover);
  overflow: hidden;
  transition:
    border-color var(--dur) var(--ease),
    box-shadow var(--dur) var(--ease);
}
.eb-group:hover {
  border-color: rgba(255, 255, 255, 0.2);
}
.eb-group:focus-within {
  border-color: var(--accent);
  box-shadow: 0 0 0 2px var(--accent-tint);
}

.eb-select {
  width: 168px;
  flex-shrink: 0;
}
.eb-select :deep(.cs-trigger) {
  height: 32px;
  border: none;
  background: transparent;
  border-radius: 0;
  font-family: var(--font-ui);
  font-weight: 500;
}
.eb-select :deep(.cs-trigger:hover:not(:disabled)) {
  background: var(--bg-hover);
}
.eb-select :deep(.cs-trigger:focus-visible) {
  outline: none;
}
.eb-select :deep(.cs.open .cs-trigger) {
  border: none;
  box-shadow: none;
}

.eb-sep {
  width: 1px;
  height: 18px;
  flex-shrink: 0;
  background: rgba(255, 255, 255, 0.1);
}

.eb-eye {
  width: 30px;
  height: 30px;
  border-radius: 0;
  background: transparent;
}
.eb-eye:hover:not(:disabled) {
  background: transparent;
}

/* ---- 环境色点（映射 utils/environment.ts 的 envColorClass） ---- */
.edot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
  background: var(--text-3);
}
.ed-dev {
  background: var(--success);
}
.ed-test {
  background: var(--info);
}
.ed-staging {
  background: var(--warning);
}
.ed-prod {
  background: #f97316;
}
.ed-global {
  background: var(--accent);
}

/* ---- 触发区展示：仅 状态点 + 环境名 ---- */
.env-display-name {
  font-size: 12px;
  font-weight: 500;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* ---- 下拉选项：第一行 点 + 名称，第二行 小字 Base URL ---- */
.env-opt-name {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  min-width: 0;
}
.env-opt-name-text {
  font-size: 12.5px;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.env-opt-url {
  font-family: var(--font-mono);
  font-size: 10.5px;
  color: var(--text-3);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* ---- 末项「管理环境…」：整行居中，顶部以分隔线隔开 ---- */
.env-manage {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  width: 100%;
  margin-right: 14px;
  font-size: 12px;
  color: var(--text-2);
}

/* ---- 浮层（Teleport 到 body，需全局兜底） ---- */
:global(.env-pop .cs-pop) {
  min-width: 240px;
}
:global(.env-pop .cs-opt) {
  height: auto;
  min-height: 30px;
  padding: 6px 10px 6px 8px;
}
:global(.env-pop .cs-opt-label) {
  display: flex;
  flex-direction: column;
  gap: 2px;
  white-space: normal;
  overflow: visible;
}
:global(.env-pop .cs-opt:last-child) {
  margin-top: 4px;
  padding-top: 8px;
  border-top: 1px solid var(--border);
}
</style>