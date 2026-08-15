import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import tailwindcss from '@tailwindcss/vite'

export default defineConfig({
  plugins: [vue(), tailwindcss()],
  // Tauri 开发模式固定端口：tauri.conf.json 的 devUrl 指向 5173
  server: {
    port: 5173,
    strictPort: true,
  },
  // Tauri 构建要求使用相对路径（产物经由 tauri://localhost 加载）
  base: './',
  clearScreen: false,
  envPrefix: ['VITE_', 'TAURI_ENV_'],
})
