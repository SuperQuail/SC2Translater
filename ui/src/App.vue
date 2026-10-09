<script setup lang="ts">
// 应用外壳：活动栏 / 顶栏 / 文件标签 / 三栏工作区 / 底栏，外加快捷键与右键菜单。
//
// 布局参考 VS Code：最左是活动栏，它决定第二栏显示什么；中间是编辑区（多标签），
// 右侧是详情。两栏都可收起、宽度可拖，且记在本地 —— 汉化是长时间盯着屏幕的活。

import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";

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
import GlossaryPane from "./components/GlossaryPane.vue";
import LocalePane from "./components/LocalePane.vue";
import MtSettingsDialog from "./components/MtSettingsDialog.vue";
import ShortcutsHelp from "./components/ShortcutsHelp.vue";
import SourceControlPane from "./components/SourceControlPane.vue";
import StatusBar from "./components/StatusBar.vue";
import TitleBar from "./components/TitleBar.vue";
import ProjectPane from "./components/ProjectPane.vue";
import WelcomeWindow from "./components/WelcomeWindow.vue";
import WorkspacePane from "./components/WorkspacePane.vue";
import { useContextMenu } from "./composables/useContextMenu";
import { useGlossary } from "./composables/useGlossary";
import { useMt } from "./composables/useMt";
import { useProject } from "./composables/useProject";
import { useVcs } from "./composables/useVcs";

const project = useProject();
const s = project.state;
const menu = useContextMenu();

// 三块新功能的共享状态：术语库 / 机翻参考 / 版本管理。
const projectFile = () => s.projectPath.value;
const activeDocument = () => s.archive.value;
const glossary = useGlossary(projectFile);
const mt = useMt(projectFile);
const vcs = useVcs(projectFile, activeDocument);
const glossaryPane = ref<InstanceType<typeof GlossaryPane> | null>(null);
const mtSettingsOpen = ref(false);

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
/** 皮肤：VS Code 官方控件那套，还是 JetBrains（Ring UI 令牌）那套。 */
type Skin = "vscode" | "jb";

interface ViewPrefs {
  left: boolean;
  right: boolean;
  leftWidth: number;
  rightWidth: number;
  density: string;
  activity: Activity;
  skin: Skin;
}
const prefs = ref<ViewPrefs>({
  left: true,
  right: true,
  leftWidth: 252,
  rightWidth: 336,
  density: "standard",
  activity: "explorer",
  skin: "vscode",
});

/** 小窗（没打开项目）与主窗（打开了）两套尺寸，照 JetBrains 换窗的做法。 */
const WELCOME_SIZE = { w: 1100, h: 760 };
const WORKSPACE_SIZE = { w: 1440, h: 900 };

async function applyWindowMode(hasDocument: boolean) {
  if (!isTauri) return;
  const { getCurrentWindow, LogicalSize } = await import("@tauri-apps/api/window");
  const win = getCurrentWindow();
  const size = hasDocument ? WORKSPACE_SIZE : WELCOME_SIZE;
  await win.setSize(new LogicalSize(size.w, size.h));
  await win.center();
}

/**
 * 窗口级操作（最小化 / 最大化 / 关闭）。
 * 只在桌面壳里有效；浏览器调试时静默忽略。
 */
async function winAction(action: "minimize" | "toggleMaximize" | "close") {
  if (!isTauri) return;
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  await getCurrentWindow()[action]();
}

/** 没打开任何东西时，左栏必须开着 —— 它就是整个舞台的左半。 */
const stageOpen = computed(() => (s.doc.value ? prefs.value.left : true));

/** 当前打开的工程（没打开就是 null），给「打开项目」那一组用。 */
const currentProject = computed(() => {
  const p = s.projectPath.value;
  if (!p) return null;
  const base = p.split(/[\\/]/).pop() ?? p;
  return { name: s.project.value.name || base, path: p };
});

/** 皮肤靠 <html data-skin> 切换；CSS 那边只换令牌，不换组件。 */
function applySkin() {
  document.documentElement.dataset.skin = prefs.value.skin;
}

function loadPrefs() {
  try {
    const raw = localStorage.getItem(PREFS_KEY);
    if (raw) prefs.value = { ...prefs.value, ...(JSON.parse(raw) as Partial<ViewPrefs>) };
  } catch {
    // 坏掉的偏好直接忽略
  }
  s.density.value = (prefs.value.density as typeof s.density.value) ?? "standard";
  activity.value = prefs.value.activity ?? "explorer";
  applySkin();
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
  await Promise.all([glossary.refresh(), mt.loadConfig()]);
  await vcs.ensureTool();

  // 切文件 / 打开项目后，术语库与仓库状态都要跟着走。
  // 注意监视的是 doc（每次载入都是新对象）而不是 archive 路径 ——
  // 在同一个项目里切换文件时路径有可能没变，只盯路径会漏掉刷新。
  watch(
    () => s.doc.value,
    async (doc) => {
      if (!doc) {
        // 回到「没打开项目」：左栏切成「项目」，窗口收回小窗
        activity.value = "projects";
        void applyWindowMode(false);
        return;
      }
      void glossary.refresh();
      void applyWindowMode(true);
      await vcs.refresh();
    },
  );
  watch(
    () => s.projectPath.value,
    () => {
      void glossary.refresh();
      void vcs.refresh();
    },
  );

  // 浏览器调试用的深链：?demo=1 进工作区、&project=1 开示例项目、&view=kv|json、&fill=1、&help=1
  if (!isTauri) {
    const params = new URLSearchParams(window.location.search);
    const view = params.get("view");
    if (view === "json" || view === "kv" || view === "table") initialView.value = view;
    if (params.get("skin") === "jb") {
      prefs.value.skin = "jb";
      applySkin();
    }
    const pane = params.get("pane");
    if (pane === "explorer" || pane === "filters" || pane === "locales" || pane === "glossary" || pane === "scm") {
      activity.value = pane;
    }
    if (params.has("help")) helpOpen.value = true;
    // 只加载「最近的项目」而不打开任何文件 —— 用来截开始页
    if (params.has("recent")) await project.refreshRecents();
    if (params.has("demo")) {
      await project.openPath(DEMO_ARCHIVE);
      if (params.has("project")) await project.openProject("D:/示例/斯托科夫战役.sc2transproj");
    }
    if (params.has("fill")) fillOpen.value = true;
    if (params.has("mt")) {
      // 演示：自动对当前条跑一次机翻参考，便于截图与自测
      window.setTimeout(() => void mt.translate(project.selectedEntry.value?.source), 600);
    }
    if (params.has("overwrite")) overwriteOpen.value = true;
  }

  // 空舞台时活动栏停在「项目」那一项，窗口是小窗
  if (!s.doc.value) {
    activity.value = "projects";
    void applyWindowMode(false);
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

/** 批量机翻当前筛选结果（只生成参考，不写进键）。 */
async function translateFiltered() {
  const texts = project.filtered.value
    .filter((e) => !(e.target ?? "").trim() && e.source)
    .slice(0, 50)
    .map((e) => e.source as string);
  if (!texts.length) {
    project.log("没有需要机翻的空译文（或当前筛选结果为空）");
    return;
  }
  const map = await mt.translateMany(texts);
  project.log("已生成 " + map.size + " 条机翻参考；在详情面板里逐条「用作译文」才会写进键");
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
    if (key === "m") {
      event.preventDefault();
      void translateFiltered();
      return;
    }
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
      case "m": {
        event.preventDefault();
        void mt.translate(project.selectedEntry.value?.source);
        return;
      }
      case "g": {
        event.preventDefault();
        activity.value = "glossary";
        prefs.value.left = true;
        savePrefs();
        requestAnimationFrame(() => glossaryPane.value?.focusSearch());
        return;
      }
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
        // Ctrl+F 按当前面板路由：术语库面板打开时搜术语，否则搜键表
        if (activity.value === "glossary" && prefs.value.left) {
          glossaryPane.value?.focusSearch();
        } else {
          pane.value?.focusSearch();
        }
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
    <!-- 打开文件时是主界面；什么都没打开时，整窗换成 JetBrains 式的小窗 -->
    <template v-if="s.doc.value">
    <TitleBar
      :window-controls="isTauri"
      @minimize="winAction('minimize')"
      @maximize="winAction('toggleMaximize')"
      @close="winAction('close')"
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
      <!-- 活动栏一直在：没打开任何东西时它只亮「项目」那一项 -->
      <ActivityBar
        v-model="activity"
        :left-open="stageOpen"
        :badges="{ scm: vcs.changeCount.value }"
      />

      <template v-if="s.doc.value">
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
            <LocalePane v-else-if="activity === 'locales'" :project="project" :menu="menu" />
            <GlossaryPane v-else-if="activity === 'glossary'" ref="glossaryPane" :glossary="glossary" :menu="menu" />
            <SourceControlPane
              v-else-if="activity === 'scm'"
              :vcs="vcs"
              :menu="menu"
              :document="s.archive.value"
            />
            <ProjectPane
              v-else
              :recents="s.recents.value"
              :current="currentProject"
              @open="project.openArchive"
              @create="project.openArchive"
              @pick="project.openPath"
              @forget="project.forgetRecent"
              @collapse="prefs.left = false; savePrefs()"
            />
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
                <DetailPanel
                  :project="project"
                  :mt="mt"
                  :glossary="glossary"
                  @settings="mtSettingsOpen = true"
                />
              </div>
            </vscode-split-layout>
          </div>
        </vscode-split-layout>
      </template>
    </div>

    <ContextMenu :menu="menu" />
    <FillDialog v-if="fillOpen" :project="project" @close="fillOpen = false" />
    <ShortcutsHelp v-if="helpOpen" @close="helpOpen = false" />
    <MtSettingsDialog v-if="mtSettingsOpen" :mt="mt" @close="mtSettingsOpen = false" />
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
    </template>

    <WelcomeWindow
      v-else
      :recents="s.recents.value"
      :dropping="dropping"
      :version="info?.version"
      @open="project.openArchive"
      @create="project.openArchive"
      @pick="project.openPath"
      @forget="project.forgetRecent"
      @settings="mtSettingsOpen = true"
      @help="helpOpen = true"
      @minimize="winAction('minimize')"
      @maximize="winAction('toggleMaximize')"
      @close="winAction('close')"
    />
  </div>
</template>

<style scoped>
.app { height: 100%; display: flex; flex-direction: column; }
.body { flex: 1; min-height: 0; display: flex; }

/* split-layout 自己画分隔条（1px，hover 变强调色），我们只管让它铺满。 */
.split { display: block; height: 100%; min-width: 0; }
.split-main { flex: 1; }
.split-detail { width: 100%; }

/* 空舞台：左边是项目工具窗，右边是说明 */
.welcome-rail { flex: 0 0 300px; width: 300px; border-right: 1px solid var(--border-subtle); }
.welcome-rail.dropping { outline: 2px dashed var(--accent); outline-offset: -6px; }

.empty-stage { flex: 1; align-items: center; justify-content: center; background: var(--bg-editor); }
.empty-stage .hint { max-width: 620px; padding: var(--s8); }
.empty-stage h1 { font-size: 24px; font-weight: 600; letter-spacing: -0.02em; line-height: 1.35; margin-bottom: var(--s5); }
.empty-stage h1 em { font-style: normal; color: var(--accent-text); }
.empty-stage p { color: var(--text-muted); font-size: var(--fs-md); line-height: 1.75; margin-bottom: var(--s5); }
.empty-stage p.dim { color: var(--text-faint); font-size: var(--fs-sm); margin: 0; }
.empty-stage code {
  background: var(--bg-raised);
  border: 1px solid var(--border-subtle);
  border-radius: var(--r1);
  padding: 1px 5px;
  color: var(--text);
}

.pane { height: 100%; min-width: 0; overflow: hidden; display: flex; flex-direction: column; }
.pane > * { flex: 1; min-height: 0; }
</style>
