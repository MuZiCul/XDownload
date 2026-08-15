/// <reference types="vitest" />
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: {
      ignored: ["../src-tauri/**", "../config/**", "../downloads/**", "../bin/**"],
    },
  },
  test: {
    // 纯函数测试用 node 环境；组件测试（.test.tsx）用 jsdom 环境。
    environment: "node",
    environmentMatchGlobs: [["src/**/*.test.tsx", "jsdom"]],
    include: ["src/**/*.test.ts", "src/**/*.test.tsx"],
  },
});
