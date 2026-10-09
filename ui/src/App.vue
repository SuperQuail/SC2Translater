<script setup lang="ts">
// 应用外壳：顶栏 + 三栏工作区 + 底栏，外加全局快捷键。
//
// 快捷键（桌面工具的基本礼貌）：
//   ↑ / ↓      上一条 / 下一条
//   Ctrl+F     搜索
//   Ctrl+S     导出 JSON
//   Ctrl+Enter 写回地图
//   Esc        退出输入框

import { onBeforeUnmount, onMounted, ref } from "vue";

import { api, isTauri } from "./api";
import { DEMO_ARCHIVE } from "./api/bridge";
import type { AppInfo } from "./api/types";
import DetailPanel from "./components/DetailPanel.vue";
import FillDialog from "./components/FillDialog.vue";
import SidePanel from "./components/SidePanel.vue";
import WorkspacePane from "./components/WorkspacePane.vue";
import StatusBar from "./components/StatusBar.vue";
import TitleBar from "./components/TitleBar.vue";
import WelcomePane from "./components/WelcomePane.vue";
import { useProject } from "./composables/useProject";

const project = useProject();
const info = ref<AppInfo | null>(null);
const pane = ref<InstanceType<typeof WorkspacePane> | null>(null);
const dropping = ref(false);
const fillOpen = ref(false);
const initialView = ref<"table" | "json">("table");

onMounted(async () => {
  info.value = await api.appInfo();
  // 浏览器调试用的深链：?demo=1 直接进工作区、&view=json 开 JSON 视图、&fill=1 开补全对话框。
  // 桌面壳里这几个参数没有影响。
  if (!isTauri) {
    const params = new URLSearchParams(window.location.search);
    if (params.has("view")) initialView.value = params.get("view") === "json" ? "json" : "table";
    if (params.has("fill")) fillOpen.value = true;
    if (params.has("demo")) await project.openPath(DEMO_ARCHIVE);
  }
  window.addEventListener("keydown", onKeydown);
  if (isTauri) {
    try {
      const { getCurrentWebview } = await import("@tauri-apps/api/webview");
      await getCurrentWebview().onDragDropEvent((event) => {
        const payload = event.payload as { type: string; paths?: string[] };
        if (payload.type === "over") dropping.value = true;
        else if (payload.type === "leave") dropping.value = false;
        else if (payload.type === "drop") {
          dropping.value = false;
          const path = payload.paths?.[0];
          if (path) void project.openPath(path);
        }
      });
    } catch {
      // 拖放不可用不影响主流程
    }
  }
});

onBeforeUnmount(() => window.removeEventListener("keydown", onKeydown));

function inTextField(target: EventTarget | null): boolean {
  const el = target as HTMLElement | null;
  if (!el) return false;
  const tag = el.tagName;
  return tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT" || el.isContentEditable;
}

function onKeydown(event: KeyboardEvent) {
  const mod = event.ctrlKey || event.metaKey;
  if (mod && event.key.toLowerCase() === "f") {
    event.preventDefault();
    pane.value?.focusSearch();
    return;
  }
  if (mod && event.key.toLowerCase() === "s") {
    event.preventDefault();
    void project.exportJson();
    return;
  }
  if (mod && event.key === "Enter") {
    event.preventDefault();
    void project.applyToArchive();
    return;
  }
  if (event.key === "Escape") {
    (document.activeElement as HTMLElement | null)?.blur?.();
    return;
  }
  if (!project.hasArchive.value) return;
  if (inTextField(event.target)) return;
  if (event.key === "ArrowDown") {
    event.preventDefault();
    project.moveSelection(1);
  } else if (event.key === "ArrowUp") {
    event.preventDefault();
    project.moveSelection(-1);
  }
}
</script>

<template>
  <div class="app">
    <TitleBar :project="project" :info="info" @fill="fillOpen = true" />

    <div class="body">
      <template v-if="project.hasArchive.value">
        <SidePanel :project="project" />
        <WorkspacePane ref="pane" :project="project" :initial-view="initialView" />
        <DetailPanel :project="project" />
      </template>
      <WelcomePane v-else :dropping="dropping" @open="project.openArchive" />
    </div>

    <FillDialog v-if="fillOpen" :project="project" @close="fillOpen = false" />
    <StatusBar :project="project" />
  </div>
</template>

<style scoped>
.app { height: 100%; display: flex; flex-direction: column; }
.body { flex: 1; min-height: 0; display: flex; }
</style>
