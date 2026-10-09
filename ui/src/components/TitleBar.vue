<script setup lang="ts">
// 顶栏：项目 + 语言对 + 主操作。控件全部用 VS Code 官方的 <vscode-button> / <vscode-single-select>。

import { computed, watch } from "vue";

import { baseName, formatBytes } from "../api";
import type { AppInfo } from "../api/types";
import type { ContextMenuApi } from "../composables/useContextMenu";
import { COMMON_LOCALES } from "../composables/useProject";
import type { Project } from "../composables/useProject";
import Icon from "./Icon.vue";

const props = defineProps<{
  project: Project;
  info: AppInfo | null;
  menu: ContextMenuApi;
  leftOpen: boolean;
  rightOpen: boolean;
  /** 桌面壳里关掉了系统标题栏，窗口按钮由我们自己画。 */
  windowControls?: boolean;
}>();
const emit = defineEmits<{
  (e: "fill"): void;
  (e: "overwrite-all"): void;
  (e: "help"): void;
  (e: "settings"): void;
  (e: "toggle-left"): void;
  (e: "toggle-right"): void;
  (e: "minimize"): void;
  (e: "maximize"): void;
  (e: "close"): void;
}>();

const s = props.project.state;
const CUSTOM = "__custom__";

const sourceChoice = computed(() =>
  COMMON_LOCALES.some((l) => l.code === s.source.value) ? s.source.value : CUSTOM,
);
const targetChoice = computed(() =>
  COMMON_LOCALES.some((l) => l.code === s.target.value) ? s.target.value : CUSTOM,
);

const archiveLabel = computed(() =>
  s.project.value.files.length
    ? s.project.value.name
    : s.archive.value
      ? baseName(s.archive.value)
      : "未打开文件",
);

const fileMeta = computed(() => {
  const st = s.info.value;
  return st ? formatBytes(st.size) : "";
});

function readSelect(event: Event): string {
  return String((event.target as unknown as { value: string }).value ?? "");
}

/** 改语言对时同时写回当前项目文件 —— 每个文件可以有自己的语言对。 */
function applyLanguages(nextSource: string, nextTarget: string) {
  const file = s.project.value.files[s.project.value.active];
  if (file) {
    file.source = nextSource;
    file.target = nextTarget;
  }
  void props.project.changeLanguages(nextSource, nextTarget);
}

function onSourceChange(event: Event) {
  const v = readSelect(event);
  if (v === CUSTOM) return;
  applyLanguages(v, s.target.value);
}

function onTargetChange(event: Event) {
  const v = readSelect(event);
  if (v === CUSTOM) return;
  applyLanguages(s.source.value, v);
}

function onProjectMenu(event: MouseEvent) {
  props.menu.openMenu(
    event,
    [
      { id: "new", label: "新建项目", hint: "Ctrl+N" },
      { id: "open", label: "打开项目…", hint: "Ctrl+O" },
      { id: "save", label: "保存项目", hint: "Ctrl+S" },
      { id: "saveas", label: "另存为…", hint: "Ctrl+Shift+S", separator: true },
      { id: "addfiles", label: "添加文件…", hint: "Ctrl+Shift+O", separator: true },
      { id: "rename", label: "重命名项目" },
    ],
    (id) => {
      if (id === "new") props.project.newProject();
      else if (id === "open") void props.project.openProjectDialog();
      else if (id === "save") void props.project.saveProject(false);
      else if (id === "saveas") void props.project.saveProject(true);
      else if (id === "addfiles") void props.project.addFiles();
      else if (id === "rename") {
        const next = window.prompt("项目名称", s.project.value.name);
        if (next !== null) props.project.renameProject(next);
      }
    },
  );
}

function onFillMenu(event: MouseEvent) {
  props.menu.openMenu(
    event,
    [
      { id: "dialog", label: "补全默认语言…（可选范围）", hint: "Ctrl+Shift+F" },
      { id: "seed", label: "一键兜底：只补当前目标语言" },
    ],
    (id) => {
      if (id === "dialog") emit("fill");
      else void props.project.seedMissing();
    },
  );
}

function onMore(event: MouseEvent) {
  props.menu.openMenu(
    event,
    [
      { id: "verify", label: "容器自检", hint: "F5" },
      { id: "reveal", label: "打开产物所在目录", separator: true },
      { id: "settings", label: "设置…", hint: "Ctrl+,", separator: true },
      { id: "help", label: "快捷键与右键菜单", hint: "F1" },
    ],
    (id) => {
      if (id === "verify") void props.project.runVerify();
      else if (id === "reveal") void props.project.revealOutput();
      else if (id === "settings") emit("settings");
      else if (id === "help") emit("help");
    },
  );
}

watch([() => s.source.value, () => s.target.value], () => void 0);
</script>

<template>
  <header class="titlebar">
    <span class="mark">SC2</span>

    <button class="project-name" title="点开项目菜单" @click="onProjectMenu">
      {{ archiveLabel }}
      <span v-if="s.projectDirty.value" class="dirty" title="有未保存的项目改动">●</span>
      <Icon name="chevron-down" :size="12" />
    </button>

    <span v-if="s.project.value.files.length" class="counts">
      {{ s.project.value.files.length }} 个文件 · 待翻译
      <b>{{ props.project.projectTotals.value.todo }}</b>
    </span>

    <span class="spacer"></span>

    <div v-if="s.doc.value" class="langs">
      <!-- @ts-expect-error vscode-single-select 是自定义元素 -->
      <vscode-single-select
        class="lang"
        :value="sourceChoice"
        title="原文语言"
        @change="onSourceChange"
      >
        <vscode-option v-for="l in COMMON_LOCALES" :key="l.code" :value="l.code">{{ l.label }} · {{ l.code }}</vscode-option>
      </vscode-single-select>
      <Icon name="chevron" :size="12" />
      <!-- @ts-expect-error 自定义元素 -->
      <vscode-single-select
        class="lang"
        :value="targetChoice"
        title="目标语言"
        @change="onTargetChange"
      >
        <vscode-option v-for="l in COMMON_LOCALES" :key="l.code" :value="l.code">{{ l.label }} · {{ l.code }}</vscode-option>
      </vscode-single-select>
      <span v-if="fileMeta" class="meta">{{ fileMeta }}</span>
    </div>

    <span class="spacer"></span>

    <div class="panels">
      <!-- @ts-expect-error 自定义元素 -->
      <vscode-button
        secondary
        icon-only
        icon="layout-sidebar-left"
        :title="'收起/展开左栏（Ctrl+B）'"
        @click="emit('toggle-left')"
      ></vscode-button>
      <!-- @ts-expect-error 自定义元素 -->
      <vscode-button
        secondary
        icon-only
        icon="layout-sidebar-right"
        title="收起/展开右栏（Ctrl+J）"
        @click="emit('toggle-right')"
      ></vscode-button>
    </div>

    <div class="actions">
      <!-- @ts-expect-error 自定义元素 -->
      <vscode-button
        secondary
        icon="arrow-down"
        :disabled="!s.doc.value || s.busy.value"
        title="导入外部翻译 JSON（Ctrl+I）"
        @click="props.project.importJson"
      >导入译文</vscode-button>
      <!-- @ts-expect-error 自定义元素 -->
      <vscode-button
        secondary
        icon="export"
        :disabled="!s.doc.value || s.busy.value"
        title="导出翻译键 JSON（Ctrl+E）"
        @click="props.project.exportJson"
      >导出 JSON</vscode-button>
      <!-- @ts-expect-error 自定义元素 -->
      <vscode-button
        icon="save"
        :disabled="!s.doc.value || s.busy.value"
        title="把译文写回地图（Ctrl+Enter）"
        @click="props.project.applyToArchive"
      >写回地图</vscode-button>
      <!-- @ts-expect-error 自定义元素 -->
      <vscode-button
        secondary
        icon="globe"
        :disabled="!s.doc.value || s.busy.value"
        title="补全语言（Ctrl+Shift+F）"
        @click="onFillMenu"
      >补全语言</vscode-button>
      <!-- @ts-expect-error 自定义元素 -->
      <vscode-button
        secondary
        icon="wand"
        class="overwrite"
        :disabled="!s.doc.value || s.busy.value"
        title="一键全覆盖：用原文覆盖所有语言的每一个键（含已有译文）"
        @click="emit('overwrite-all')"
      >一键全覆盖</vscode-button>
      <!-- @ts-expect-error 自定义元素 -->
      <vscode-button
        secondary
        icon-only
        icon="ellipsis"
        title="更多"
        @click="onMore"
      ></vscode-button>
      <!-- @ts-expect-error 自定义元素 -->
      <vscode-button
        secondary
        icon-only
        icon="question"
        title="快捷键（F1）"
        @click="emit('help')"
      ></vscode-button>
    </div>

    <!-- 自绘窗口按钮（tauri.conf 里 decorations=false，由我们画） -->
    <div v-if="props.windowControls" class="win">
      <button class="win-btn" title="最小化" @click="emit('minimize')"><span class="glyph">—</span></button>
      <button class="win-btn" title="最大化" @click="emit('maximize')"><span class="glyph">▢</span></button>
      <button class="win-btn danger" title="关闭" @click="emit('close')"><span class="glyph">✕</span></button>
    </div>
  </header>
</template>

<style scoped>
.titlebar {
  height: var(--h-titlebar);
  flex: 0 0 auto;
  display: flex;
  align-items: center;
  gap: var(--s4);
  padding: 0 var(--s4) 0 var(--s3);
  background: var(--bg-titlebar);
  border-bottom: 1px solid var(--border-titlebar);
}

.mark {
  display: grid;
  place-items: center;
  width: 22px;
  height: 22px;
  border-radius: var(--r1);
  background: var(--accent);
  color: #fff;
  font-size: 10px;
  font-weight: 700;
}

.project-name {
  display: inline-flex;
  align-items: center;
  gap: var(--s3);
  height: 24px;
  padding: 0 var(--s3);
  font-size: var(--fs-md);
  font-weight: 600;
  color: var(--text);
  max-width: 300px;
}
.project-name:hover { background: var(--bg-hover); }

.dirty { color: var(--warn-text); font-size: 10px; }

.counts { font-size: var(--fs-xs); color: var(--text-faint); white-space: nowrap; }
.counts b { color: var(--warn-text); font-variant-numeric: tabular-nums; }

.spacer { flex: 1; }

.langs { display: flex; align-items: center; gap: var(--s3); }
.lang { width: 168px; }
.meta { font-size: var(--fs-xs); color: var(--text-faint); }

.panels, .actions { display: flex; align-items: center; gap: var(--s2); }

.win { display: flex; margin-left: var(--s3); align-self: stretch; }
.win-btn {
  width: 44px;
  padding: 0;
  justify-content: center;
  border-radius: 0;
  background: transparent;
  color: var(--text-muted);
}
.win-btn:hover { background: var(--list-hover); color: var(--text); }
.win-btn.danger:hover { background: #c42b1c; color: #fff; }
.glyph { font-size: 12px; line-height: 1; }

/* 「一键全覆盖」是破坏性操作，描边带点警示色 */
.overwrite::part(control) { border-color: var(--warn); }
</style>
