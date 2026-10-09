<script setup lang="ts">
// 应用外壳：活动栏 / 顶栏 / 文件标签 / 三栏工作区 / 底栏，外加快捷键与右键菜单。
//
// 布局参考 VS Code：最左是活动栏，它决定第二栏显示什么；中间是编辑区（多标签），
// 右侧是详情。两栏都可收起、宽度可拖，且记在本地 —— 汉化是长时间盯着屏幕的活。

import { onBeforeUnmount, onMounted, ref, watch } from "vue";

import { api, isTauri } from "./api";
import { DEMO_ARCHIVE } from "./api/bridge";
import type { AppInfo } from "./api/types";
import ActivityBar, { type Activity } from "./components/ActivityBar.vue";
import ConfirmDialog from "./components/ConfirmDialog.vue";
import ContextMenu from "./components/ContextMenu.vue";
import DetailPanel from "./components/DetailPanel.vue";
import ExplorerPane from "./components/ExplorerPane.vue";
import FileTabs from "./components/FileTabs.vue";
import FillDialog from "./components/FillDialog.vue";
import FilterPane from "./components/FilterPane.vue";
import LocalePane from "./components/LocalePane.vue";
import ShortcutsHelp from "./components/ShortcutsHelp.vue";
import StatusBar from "./components/StatusBar.vue";
import TitleBar from "./components/TitleBar.vue";
import WelcomePane from "./components/WelcomePane.vue";
import WorkspacePane from "./components/WorkspacePane.vue";
import { useContextMenu } from "./composables/useContextMenu";
import { useProject } from "./composables/useProject";

const project = useProject();
const s = project.state;
const menu = useContextMenu();

const info = ref<AppInfo | null>(null);
const pane = ref<InstanceType<typeof WorkspacePane> | null>(null);
const dropping = ref(false);
const fillOpen = ref(false);
const helpOpen = ref(false);
const overwriteOpen = ref(false);
const activity = ref<Activity>("explorer");
const initialView = ref<"table" | "kv" | "json">("table");

/** 视图偏好（栏宽、开合、活动栏、密度）—— 记在本地。 */
const PREFS_KEY = "sc2trans.view";
interface ViewPrefs {
  left: boolean;
  right: boolean;
  leftWidth: number;
  rightWidth: number;
  density: string;
  activity: Activity;
}
const prefs = ref<ViewPrefs>({
  left: true,
  right: true,
  leftWidth: 252,
  rightWidth: 336,
  density: "standard",
  activity: "explorer",
});

function loadPrefs() {
  try {
    const raw = localStorage.getItem(PREFS_KEY);
    if (raw) prefs.value = { ...prefs.value, ...(JSON.parse(raw) as Partial<ViewPrefs>) };
  } catch {
    // 坏掉的偏好直接忽略
  }
  s.density.value = (prefs.value.density as typeof s.density.value) ?? "standard";
  activity.value = prefs.value.activity ?? "explorer";
}

function savePrefs() {
  prefs.value.density = s.density.value;
  prefs.value.activity = activity.value;
  try {
    localStorage.setItem(PREFS_KEY, JSON.stringify(prefs.value));
  } catch {
    // 存不下就算了
  }
}

/**
 * 栏宽交给官方 <vscode-split-layout> 管，我们只在拖完后把它当前的位置记下来。
 * 组件没有专门的拖拽事件，所以监听容器上的 pointerup 读 handlePosition。
 */
function captureWidth(which: "left" | "right", event: PointerEvent) {
  const el = event.currentTarget as unknown as { handlePosition?: number } | null;
  const pos = el?.handlePosition;
  if (typeof pos !== "number" || Number.isNaN(pos)) return;
  if (which === "left") {
    if (pos < 100) return; // 收起状态，不记
    prefs.value.leftWidth = Math.min(620, Math.max(190, Math.round(pos)));
  } else {
    if (pos < 100) return;
    prefs.value.rightWidth = Math.min(620, Math.max(220, Math.round(pos)));
  }
  savePrefs();
}

watch([() => s.density.value, activity], savePrefs);

onMounted(async () => {
  loadPrefs();
  info.value = await api.appInfo();
  await project.refreshRecents();

  // 浏览器调试用的深链：?demo=1 进工作区、&project=1 开示例项目、&view=kv|json、&fill=1、&help=1
  if (!isTauri) {
    const params = new URLSearchParams(window.location.search);
    const view = params.get("view");
    if (view === "json" || view === "kv" || view === "table") initialView.value = view;
    if (params.has("help")) helpOpen.value = true;
    if (params.has("demo")) {
      await project.openPath(DEMO_ARCHIVE);
      if (params.has("project")) await project.openProject("D:/示例/斯托科夫战役.sc2transproj");
    }
    if (params.has("fill")) fillOpen.value = true;
    if (params.has("overwrite")) overwriteOpen.value = true;
  }

  window.addEventListener("keydown", onKeydown);
});

onBeforeUnmount(() => {
  window.removeEventListener("keydown", onKeydown);
  savePrefs();
});

function inTextField(target: EventTarget | null): boolean {
  const el = target as HTMLElement | null;
  if (!el) return false;
  const tag = el.tagName;
  return tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT" || el.isContentEditable;
}

/** 一键全覆盖：用原文覆盖所有语言的所有键。破坏性，所以先确认。 */
async function runOverwriteAll() {
  overwriteOpen.value = false;
  if (!s.archive.value) return;
  const name = await api.defaultOutputName(s.archive.value, "all");
  const out = await api.pickArchiveSave(name);
  if (!out) return;
  await project.fillDefaultLanguages("all", false, out);
}

/** 全局快捷键。完整列表见 ShortcutsHelp（F1）。 */
function onKeydown(event: KeyboardEvent) {
  const mod = event.ctrlKey || event.metaKey;
  const key = event.key.toLowerCase();

  if (event.key === "F1") {
    event.preventDefault();
    helpOpen.value = !helpOpen.value;
    return;
  }
  if (event.key === "F5") {
    event.preventDefault();
    void project.runVerify();
    return;
  }
  if (event.key === "Escape") {
    if (menu.menu.value) menu.close();
    (document.activeElement as HTMLElement | null)?.blur?.();
    return;
  }

  if (mod && event.shiftKey) {
    if (key === "s") {
      event.preventDefault();
      void project.saveProject(true);
      return;
    }
    if (key === "o") {
      event.preventDefault();
      void project.addFiles();
      return;
    }
    if (key === "f") {
      event.preventDefault();
      fillOpen.value = true;
      return;
    }
    if (key === "d") {
      event.preventDefault();
      project.fillFromSource("all");
      return;
    }
    if (event.key === "Tab") {
      event.preventDefault();
      if (project.hasProject.value) {
        const n = s.project.value.files.length;
        void project.selectFile((s.project.value.active - 1 + n) % n);
      }
      return;
    }
  }

  if (mod) {
    switch (key) {
      case "n":
        event.preventDefault();
        project.newProject();
        return;
      case "o":
        event.preventDefault();
        void project.openProjectDialog();
        return;
      case "s":
        event.preventDefault();
        void project.saveProject(false);
        return;
      case "e":
        event.preventDefault();
        void project.exportJson();
        return;
      case "i":
        event.preventDefault();
        void project.importJson();
        return;
      case "f":
        event.preventDefault();
        pane.value?.focusSearch();
        return;
      case "b":
        event.preventDefault();
        prefs.value.left = !prefs.value.left;
        savePrefs();
        return;
      case "j":
        event.preventDefault();
        prefs.value.right = !prefs.value.right;
        savePrefs();
        return;
      case "w":
        event.preventDefault();
        if (project.hasProject.value) project.removeFile(s.project.value.active);
        return;
      case "d":
        event.preventDefault();
        project.fillFromSource("selected");
        return;
      case "enter":
        event.preventDefault();
        void project.applyToArchive();
        return;
      default:
        break;
    }
    if (event.key === "Tab") {
      event.preventDefault();
      if (project.hasProject.value) {
        const n = s.project.value.files.length;
        void project.selectFile((s.project.value.active + 1) % n);
      }
      return;
    }
    if (/^[1-9]$/.test(event.key) && project.hasProject.value) {
      event.preventDefault();
      const index = Number(event.key) - 1;
      if (index < s.project.value.files.length) void project.selectFile(index);
      return;
    }
  }

  if (!s.doc.value) return;
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
    <TitleBar
      :project="project"
      :info="info"
      :menu="menu"
      :left-open="prefs.left"
      :right-open="prefs.right"
      @fill="fillOpen = true"
      @overwrite-all="overwriteOpen = true"
      @help="helpOpen = true"
      @toggle-left="prefs.left = !prefs.left; savePrefs()"
      @toggle-right="prefs.right = !prefs.right; savePrefs()"
    />

    <FileTabs v-if="project.hasProject.value || s.doc.value" :project="project" :menu="menu" />

    <div class="body">
      <template v-if="s.doc.value">
        <ActivityBar v-model="activity" :left-open="prefs.left" />

        <!-- 嵌套两个官方 split-layout：外层分侧栏，内层分详情栏。 -->
        <!-- @ts-expect-error vscode-split-layout 是自定义元素 -->
        <vscode-split-layout
          class="split split-main"
          split="vertical"
          :handle-position="prefs.left ? prefs.leftWidth : 0"
          :min-start="prefs.left ? 190 : 0"
          @pointerup="captureWidth('left', $event)"
        >
          <div slot="start" v-show="prefs.left" class="pane">
            <ExplorerPane v-if="activity === 'explorer'" :project="project" :menu="menu" />
            <FilterPane v-else-if="activity === 'filters'" :project="project" :menu="menu" />
            <LocalePane v-else :project="project" :menu="menu" />
          </div>

          <div slot="end" class="pane">
            <!-- @ts-expect-error 自定义元素 -->
            <vscode-split-layout
              class="split split-detail"
              split="vertical"
              fixed-pane="end"
              :handle-position="prefs.right ? prefs.rightWidth : 0"
              :min-end="prefs.right ? 220 : 0"
              @pointerup="captureWidth('right', $event)"
            >
              <div slot="start" class="pane">
                <WorkspacePane ref="pane" :project="project" :initial-view="initialView" :menu="menu" />
              </div>
              <div slot="end" v-show="prefs.right" class="pane">
                <DetailPanel :project="project" />
              </div>
            </vscode-split-layout>
          </div>
        </vscode-split-layout>
      </template>
      <WelcomePane v-else :dropping="dropping" @open="project.openArchive" />
    </div>

    <ContextMenu :menu="menu" />
    <FillDialog v-if="fillOpen" :project="project" @close="fillOpen = false" />
    <ShortcutsHelp v-if="helpOpen" @close="helpOpen = false" />
    <ConfirmDialog
      v-if="overwriteOpen"
      title="一键全覆盖"
      message="用当前原文，覆盖所有语言目录下的每一个键。"
      detail="已有译文会被原文替换掉（含 zhCN 里已经翻好的部分）。产物会写成新文件，原件不动。若只想补缺失的键，请改用「补全语言」。"
      confirm-text="选择输出文件并覆盖"
      danger
      @cancel="overwriteOpen = false"
      @confirm="runOverwriteAll"
    />
    <StatusBar :project="project" />
  </div>
</template>

<style scoped>
.app { height: 100%; display: flex; flex-direction: column; }
.body { flex: 1; min-height: 0; display: flex; }

/* split-layout 自己画分隔条（1px，hover 变强调色），我们只管让它铺满。 */
.split { display: block; height: 100%; min-width: 0; }
.split-main { flex: 1; }
.split-detail { width: 100%; }

.pane { height: 100%; min-width: 0; overflow: hidden; display: flex; flex-direction: column; }
.pane > * { flex: 1; min-height: 0; }
</style>
