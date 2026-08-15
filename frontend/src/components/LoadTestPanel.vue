<script setup lang="ts">
/**
 * LoadTestPanel：压测（并发基准）面板。
 * - 实时面积图（Chart.js）：x=已耗时(s)，y=累计成功/失败请求数，压测中逐事件刷新；
 * - 3 列 KPI 指标网格：成功率 / 平均耗时 / RPS / P50 / P90 / P99。
 */
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import {
  Chart as ChartJS,
  Filler,
  Legend,
  LinearScale,
  LineElement,
  PointElement,
  Tooltip,
} from 'chart.js'
import { Line } from 'vue-chartjs'
import type { ChartOptions } from 'chart.js'
import { useFoxApi } from '../composables/useFoxApi'
import { useToast } from '../composables/useToast'
import { useWorkspaceStore } from '../stores/workspace'
import CustomNumberInput from './ui/CustomNumberInput.vue'
import Icon from './ui/Icon.vue'
import { formatDuration } from '../utils/format'
import type { Endpoint, LoadResult } from '../types/foxApi'

ChartJS.register(LinearScale, PointElement, LineElement, Filler, Tooltip, Legend)

const props = defineProps<{
  draft: Endpoint | null
  url: string
}>()

const api = useFoxApi()
const toast = useToast()
const store = useWorkspaceStore()

// ---------- 运行控制 ----------
const loadConcurrency = ref('20')
const loadTotal = ref('200')
const loading = ref(false)
const loadResult = ref<LoadResult | null>(null)

const progress = ref<{ done: number; total: number; ok: number; failed: number } | null>(null)
const running = computed(() => loading.value || progress.value !== null)

// ---------- 实时图表 ----------
interface ChartPoint {
  x: number
  ok: number
  failed: number
}

const points = ref<ChartPoint[]>([])
let startTs: number | null = null

function cssVar(name: string, fallback: string): string {
  const v = getComputedStyle(document.documentElement).getPropertyValue(name).trim()
  return v || fallback
}

const okColor = cssVar('--success', '#22c55e')
const errColor = cssVar('--danger', '#ef4444')

const chartData = computed(() => ({
  datasets: [
    {
      label: '成功',
      data: points.value.map((p) => ({ x: p.x, y: p.ok })),
      borderColor: okColor,
      backgroundColor: `${okColor}2e`,
      fill: true,
      tension: 0.35,
      borderWidth: 1.5,
      pointRadius: 0,
    },
    {
      label: '失败',
      data: points.value.map((p) => ({ x: p.x, y: p.failed })),
      borderColor: errColor,
      backgroundColor: `${errColor}2e`,
      fill: true,
      tension: 0.35,
      borderWidth: 1.5,
      pointRadius: 0,
    },
  ],
}))

const chartOptions: ChartOptions<'line'> = {
  responsive: true,
  maintainAspectRatio: false,
  animation: false,
  interaction: { mode: 'index', intersect: false },
  scales: {
    x: {
      type: 'linear',
      title: { display: true, text: '耗时（秒）', font: { size: 11 } },
      ticks: { precision: 0 },
      grid: { color: 'rgba(128,128,128,0.12)' },
    },
    y: {
      beginAtZero: true,
      title: { display: true, text: '已完成请求数', font: { size: 11 } },
      ticks: { precision: 0 },
      grid: { color: 'rgba(128,128,128,0.12)' },
    },
  },
  plugins: {
    legend: { labels: { boxWidth: 12, boxHeight: 12, font: { size: 11 } } },
  },
}

function onProgress(p: { done: number; total: number; ok: number; failed: number }): void {
  progress.value = p
  const now = performance.now()
  if (startTs === null) startTs = now
  const x = Math.round(((now - startTs) / 1000) * 10) / 10
  const last = points.value[points.value.length - 1]
  if (last && Math.abs(x - last.x) < 0.05) {
    last.ok = p.ok
    last.failed = p.failed
  } else {
    points.value.push({ x, ok: p.ok, failed: p.failed })
  }
  if (points.value.length > 400) points.value.splice(0, points.value.length - 400)
}

let unlistenLoad: UnlistenFn | null = null

onMounted(async () => {
  unlistenLoad = await listen<{ done: number; total: number; ok: number; failed: number }>(
    'fox:load-progress',
    (event) => onProgress(event.payload),
  )
})

onUnmounted(() => {
  unlistenLoad?.()
})

// ---------- 启动压测 ----------
async function runLoadTest(): Promise<void> {
  if (!props.draft) return
  const concurrency = Number(loadConcurrency.value)
  const total = Number(loadTotal.value)
  if (!Number.isFinite(concurrency) || !Number.isFinite(total) || total < 1) {
    toast.error('请输入合法的并发数与总数')
    return
  }
  loading.value = true
  progress.value = null
  loadResult.value = null
  points.value = []
  startTs = null
  try {
    loadResult.value = await api.loadTest({
      url: props.url,
      method: props.draft.method,
      spec: props.draft.request,
      environment_id: store.activeEnvId,
      concurrency,
      total,
    })
  } catch (err) {
    toast.error('压测失败', { message: err instanceof Error ? err.message : String(err) })
  } finally {
    loading.value = false
    progress.value = null
  }
}

// ---------- 指标 ----------
interface Metric {
  label: string
  value: string
  tone?: 'ok' | 'warn' | 'err'
}

const metrics = computed<Metric[]>(() => {
  const r = loadResult.value
  if (!r) {
    const p = progress.value
    return [
      { label: '已完成', value: p ? `${p.done}/${p.total}` : '—' },
      { label: '成功', value: p ? String(p.ok) : '—', tone: 'ok' },
      { label: '失败', value: p ? String(p.failed) : '—', tone: p && p.failed > 0 ? 'err' : undefined },
      { label: '平均耗时', value: '—' },
      { label: 'RPS', value: '—' },
      { label: 'P50', value: '—' },
    ]
  }
  const rate = ((r.ok / Math.max(r.total, 1)) * 100).toFixed(1)
  return [
    {
      label: '成功率',
      value: `${rate}%`,
      tone: Number(rate) === 100 ? 'ok' : Number(rate) >= 95 ? 'warn' : 'err',
    },
    { label: '平均耗时', value: formatDuration(r.avg_ms) },
    { label: 'RPS', value: r.rps.toFixed(1) },
    { label: 'P50', value: formatDuration(r.p50_ms) },
    { label: 'P90', value: formatDuration(r.p90_ms) },
    { label: 'P99', value: formatDuration(r.p99_ms) },
  ]
})
</script>

<template>
  <div class="load-panel">
    <div class="load-controls">
      <CustomNumberInput
        :model-value="loadConcurrency"
        size="sm"
        :min="1"
        placeholder="并发"
        class="load-num"
        :disabled="running"
        @update:model-value="loadConcurrency = String($event)"
      />
      <CustomNumberInput
        :model-value="loadTotal"
        size="sm"
        :min="1"
        placeholder="总请求数"
        class="load-num"
        :disabled="running"
        @update:model-value="loadTotal = String($event)"
      />
      <button class="rf-btn rf-btn-sm" type="button" :disabled="!draft || running" @click="runLoadTest">
        <Icon name="gauge" :size="13" />
        {{ loading ? `压测中… ${progress ? `${progress.done}/${progress.total}` : ''}` : '开始压测' }}
      </button>
    </div>

    <div v-if="running || loadResult" class="chart-wrap">
      <Line :data="chartData" :options="chartOptions" />
    </div>

    <div v-if="running || loadResult" class="metrics">
      <div
        v-for="m in metrics"
        :key="m.label"
        class="metric"
        :class="{ [`tone-${m.tone}`]: m.tone }"
      >
        <span class="metric-label">{{ m.label }}</span>
        <span class="metric-value">{{ m.value }}</span>
      </div>
    </div>

    <ul v-if="loadResult?.errors.length" class="load-errors">
      <li v-for="(e, i) in loadResult.errors" :key="i">{{ e }}</li>
    </ul>

    <p v-if="!running && !loadResult" class="load-hint">
      设置并发与总请求数后开始压测；运行中实时绘制成功/失败曲线，结束后展示分位耗时。
    </p>
  </div>
</template>

<style scoped>
.load-panel {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.load-controls {
  display: flex;
  gap: 8px;
  align-items: center;
}

.load-num {
  width: 110px;
}

.chart-wrap {
  position: relative;
  height: 220px;
  padding: 4px 2px 0;
}

.metrics {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 8px;
}

.metric {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 10px 12px;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--bg-card);
}

.metric-label {
  font-size: 11px;
  color: var(--text-3);
}

.metric-value {
  font-size: 15px;
  font-weight: 700;
  font-family: var(--font-mono);
  color: var(--text-1);
}

.metric.tone-ok .metric-value {
  color: var(--success);
}

.metric.tone-warn .metric-value {
  color: var(--warning);
}

.metric.tone-err .metric-value {
  color: var(--danger);
}

.load-errors {
  margin: 0;
  padding: 8px 12px 8px 28px;
  border: 1px solid var(--danger-border);
  border-radius: var(--radius);
  background: var(--danger-tint);
  color: var(--danger);
  font-size: 12px;
  word-break: break-all;
}

.load-hint {
  margin: 0;
  font-size: 12px;
  color: var(--text-3);
}
</style>