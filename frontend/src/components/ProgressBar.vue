<script setup lang="ts">
/**
 * ProgressBar：全局顶部加载进度条（配合 composables/useProgress.ts）。
 *
 * 固定悬浮在窗口顶部，2px 高，rf- 品牌蓝渐变 + 前进流光动画（NProgress 风格）。
 * 在 App 根组件挂载一次：
 * ```html
 * <ProgressBar />
 * ```
 * useFoxApi 的 `run()` 已自动 start/done，无需业务代码介入。
 */
import { useProgress } from '../composables/useProgress'

const { visible, progress } = useProgress()
</script>

<template>
  <Transition name="rf-progress">
    <div v-if="visible" class="rf-progress" role="progressbar" aria-label="加载中">
      <div class="rf-progress-bar" :style="{ width: `${progress}%` }"></div>
    </div>
  </Transition>
</template>

<style scoped>
.rf-progress {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  height: 2px;
  z-index: 2000;
  pointer-events: none;
}

.rf-progress-bar {
  height: 100%;
  border-radius: 0 2px 2px 0;
  background: linear-gradient(90deg, var(--rf-accent-weak), var(--rf-accent) 60%, var(--rf-info));
  box-shadow: 0 0 8px rgba(59, 130, 246, 0.7);
  transition: width 0.2s ease;
}

.rf-progress-bar::after {
  content: '';
  position: absolute;
  top: 0;
  right: 0;
  bottom: 0;
  width: 96px;
  background: linear-gradient(90deg, transparent, rgba(255, 255, 255, 0.35), transparent);
  animation: rf-progress-shine 1s linear infinite;
}

@keyframes rf-progress-shine {
  from {
    transform: translateX(-96px);
  }
  to {
    transform: translateX(100vw);
  }
}

.rf-progress-enter-active,
.rf-progress-leave-active {
  transition: opacity 0.24s ease;
}

.rf-progress-enter-from,
.rf-progress-leave-to {
  opacity: 0;
}
</style>