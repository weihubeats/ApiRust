<script setup lang="ts">
import { useRouter } from 'vue-router'

const router = useRouter()

const entries = [
  {
    to: '/projects',
    title: '项目列表',
    desc: '创建、导入与管理接口项目，调试请求，查看历史',
    accent: '#6366f1',
    icon: 'folder',
  },
  {
    to: '/graphql',
    title: 'GraphQL 控制台',
    desc: '连接 GraphQL 服务，发送查询、变更与订阅',
    accent: '#e879f9',
    icon: 'graphql',
  },
  {
    to: '/settings',
    title: '设置',
    desc: '环境变量、Mock 服务、备份恢复与应用配置',
    accent: '#34d399',
    icon: 'settings',
  },
]
</script>

<template>
  <div class="home">
    <div class="home-inner">
      <header class="brand">
        <div class="logo" aria-hidden="true">
          <svg width="30" height="30" viewBox="0 0 24 24" fill="none">
            <path
              d="M13.2 2 4.4 13.6h6.2L9.1 22l8.9-11.6h-6.3L13.2 2z"
              fill="currentColor"
            />
          </svg>
        </div>
        <h1 class="title">RustFox</h1>
        <p class="tagline">本地优先的 API 设计、调试、Mock 与测试工具</p>
      </header>

      <nav class="cards" aria-label="功能导航">
        <button
          v-for="e in entries"
          :key="e.to"
          type="button"
          class="card"
          @click="router.push(e.to)"
        >
          <span class="card-icon" :style="{ color: e.accent, background: e.accent + '1f' }">
            <svg
              v-if="e.icon === 'folder'"
              width="22"
              height="22"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="1.8"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V7z" />
            </svg>
            <svg
              v-else-if="e.icon === 'graphql'"
              width="22"
              height="22"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="1.8"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <circle cx="12" cy="4.2" r="2.2" />
              <circle cx="4.6" cy="18.5" r="2.2" />
              <circle cx="19.4" cy="18.5" r="2.2" />
              <path d="M12 6.4 5.4 17.2m6.6-10.8 6.6 10.8M6.8 17h10.4" />
            </svg>
            <svg
              v-else
              width="22"
              height="22"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="1.8"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <circle cx="12" cy="12" r="3" />
              <path
                d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 1 1-4 0v-.09a1.65 1.65 0 0 0-1-1.51 1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 1 1 0-4h.09a1.65 1.65 0 0 0 1.51-1 1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33h.09a1.65 1.65 0 0 0 1-1.51V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 1 1.51h.09a1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82v.09a1.65 1.65 0 0 0 1.51 1H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"
              />
            </svg>
          </span>
          <span class="card-body">
            <span class="card-title">{{ e.title }}</span>
            <span class="card-desc">{{ e.desc }}</span>
          </span>
          <svg
            class="card-arrow"
            width="18"
            height="18"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
          >
            <path d="m9 18 6-6-6-6" />
          </svg>
        </button>
      </nav>

      <footer class="foot">v0.0.1 · Tauri 2 · 数据保存在本机</footer>
    </div>
  </div>
</template>

<style scoped>
.home {
  position: relative;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  overflow: hidden;
  background:
    radial-gradient(55% 45% at 50% 0%, rgba(99, 102, 241, 0.14), transparent 65%),
    radial-gradient(38% 32% at 88% 88%, rgba(59, 130, 246, 0.1), transparent 62%),
    radial-gradient(30% 26% at 8% 82%, rgba(52, 211, 153, 0.06), transparent 60%),
    var(--rf-bg);
}

.home::before {
  content: '';
  position: absolute;
  inset: 0;
  background-image:
    linear-gradient(rgba(148, 163, 184, 0.055) 1px, transparent 1px),
    linear-gradient(90deg, rgba(148, 163, 184, 0.055) 1px, transparent 1px);
  background-size: 44px 44px;
  -webkit-mask-image: radial-gradient(72% 62% at 50% 42%, #000 0%, transparent 78%);
  mask-image: radial-gradient(72% 62% at 50% 42%, #000 0%, transparent 78%);
  pointer-events: none;
}

.home-inner {
  position: relative;
  z-index: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 34px;
  padding: 24px;
  animation: rise 0.5s ease both;
}

@keyframes rise {
  from {
    opacity: 0;
    transform: translateY(14px);
  }
  to {
    opacity: 1;
    transform: none;
  }
}

.brand {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 10px;
}

.logo {
  width: 64px;
  height: 64px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #fff;
  border-radius: 18px;
  background: linear-gradient(135deg, #6366f1 0%, #3b82f6 100%);
  box-shadow:
    0 12px 32px rgba(99, 102, 241, 0.35),
    inset 0 1px 0 rgba(255, 255, 255, 0.25);
}

.title {
  margin: 0;
  font-size: 30px;
  font-weight: 700;
  letter-spacing: -0.02em;
  background: linear-gradient(100deg, #f9fafb 30%, #a5b4fc 90%);
  -webkit-background-clip: text;
  background-clip: text;
  color: transparent;
}

.tagline {
  margin: 0;
  font-size: 13.5px;
  color: var(--rf-text-secondary);
}

.cards {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 14px;
  width: min(720px, 92vw);
}

.card {
  display: flex;
  align-items: flex-start;
  gap: 12px;
  padding: 16px 16px 16px 14px;
  text-align: left;
  border: 1px solid var(--rf-border);
  border-radius: 14px;
  background: var(--rf-bg-panel);
  color: var(--rf-text);
  cursor: pointer;
  transition:
    transform 0.18s ease,
    border-color 0.18s ease,
    box-shadow 0.18s ease,
    background 0.18s ease;
  animation: rise 0.5s ease both;
}

.card:nth-child(2) {
  animation-delay: 0.07s;
}
.card:nth-child(3) {
  animation-delay: 0.14s;
}

.card:hover {
  transform: translateY(-3px);
  border-color: var(--rf-accent);
  box-shadow: 0 10px 28px rgba(0, 0, 0, 0.35);
}

.card:focus-visible {
  outline: 2px solid var(--rf-accent);
  outline-offset: 2px;
}

.card-icon {
  flex: none;
  width: 42px;
  height: 42px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 12px;
}

.card-body {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 3px;
  min-width: 0;
}

.card-title {
  font-size: 14px;
  font-weight: 600;
}

.card-desc {
  font-size: 12px;
  line-height: 1.5;
  color: var(--rf-text-secondary);
}

.card-arrow {
  flex: none;
  margin-top: 2px;
  color: var(--rf-text-muted);
  transition:
    color 0.18s ease,
    transform 0.18s ease;
}

.card:hover .card-arrow {
  color: var(--rf-accent);
  transform: translateX(2px);
}

.foot {
  font-size: 11.5px;
  color: var(--rf-text-muted);
}

@media (max-width: 760px) {
  .cards {
    grid-template-columns: 1fr;
    width: min(420px, 92vw);
  }
}
</style>
