<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'

const router = useRouter()
const ready = ref(false)

onMounted(() => {
  ready.value = true
})
</script>

<template>
  <div class="rf-panel home">
    <h1 class="rf-heading">RustFox</h1>
    <p class="rf-hint">RustFox 已迁移至 Tauri 2 + Vue 3。</p>
    <div class="home-actions">
      <button class="rf-btn rf-btn-primary" :disabled="!ready" @click="router.push('/projects')">
        项目列表
      </button>
      <button class="rf-btn" :disabled="!ready" @click="router.push('/graphql')">
        GraphQL 控制台
      </button>
      <button class="rf-btn" :disabled="!ready" @click="router.push('/settings')">设置</button>
    </div>
  </div>
</template>

<style scoped>
.home {
  gap: 14px;
}

.rf-panel {
  max-width: 640px;
  margin: 0 auto;
  padding: 20px;
  border-radius: 10px;
  background: var(--rf-bg-panel-2, #111827);
  border: 1px solid var(--rf-border, #1f2937);
}

.rf-heading {
  margin: 0 0 12px;
  font-size: 18px;
  font-weight: 600;
  color: var(--rf-text, #f9fafb);
}

.rf-hint {
  margin: 0 0 12px;
  font-size: 12.5px;
  color: var(--rf-text-secondary, #9ca3af);
}

.home-actions {
  display: flex;
  gap: 8px;
}
</style>
