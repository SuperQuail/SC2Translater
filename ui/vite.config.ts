// 从 vitest/config 引入：这样同一份配置里既能写 vite 选项也能写 test 选项。
import { defineConfig } from "vitest/config";
import vue from "@vitejs/plugin-vue";

// SC2Translater 前端构建配置。
// - 固定端口，便于 Tauri 的 devUrl 与浏览器调试模式共用（与参考项目 SC2Miyin 的 5183 错开）
// - 构建目标对齐 WebView2（Chromium 110+）
export default defineConfig({
  plugins: [vue()],
  clearScreen: false,
  server: {
    port: 5184,
    strictPort: true,
  },
  build: {
    outDir: "dist",
    emptyOutDir: true,
    target: "chrome110",
    sourcemap: false,
  },
  test: {
    environment: "node",
    include: ["tests/**/*.spec.ts"],
  },
});
