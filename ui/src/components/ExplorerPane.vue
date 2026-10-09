<script setup lang="ts">
// 资源管理器：直接用 VS Code 官方的 <vscode-tree> / <vscode-tree-item>。
//
// 展开箭头、缩进参考线、选中态、键盘导航都由组件负责 —— VS Code 里长什么样，这里就长什么样。

import { computed } from "vue";

import { baseName, formatBytes } from "../api";
import type { FileEntry, FileKind } from "../api/types";
import type { ContextMenuApi } from "../composables/useContextMenu";
import type { Project } from "../composables/useProject";
import Icon from "./Icon.vue";

const props = defineProps<{
  project: Project;
  menu: ContextMenuApi;
  /** 扫出来的工程文件（类 VS Code 的文件夹视图）。 */
  tree?: FileEntry[];
  /** 当前打开的文件。 */
  activePath?: string;
}>();
const emit = defineEmits<{ (e: "open", path: string): void }>();
const s = props.project.state;

/** 按目录分组的文件树。 */
const groups = computed(() => {
  const map = new Map<string, FileEntry[]>();
  for (const f of props.tree ?? []) {
    const i = f.path.lastIndexOf("/");
    const dir = i < 0 ? "" : f.path.slice(0, i);
    const list = map.get(dir) ?? [];
    list.push(f);
    map.set(dir, list);
  }
  return [...map.entries()].sort((a, b) => a[0].localeCompare(b[0]));
});

/** 文件类型 -> codicon。 */
function kindIcon(kind: FileKind): string {
  if (kind === "sc2") return "map";
  if (kind === "image") return "file-media";
  if (kind === "binary") return "file-binary";
  return "file";
}

/** 树里的名字：取最后一段。 */
function leafName(p: string): string {
  const i = p.lastIndexOf("/");
  return i < 0 ? p : p.slice(i + 1);
}

const currentInfo = computed(() => {
  const i = s.info.value;
  if (!i) return null;
  return { size: formatBytes(i.size), components: i.container.components, blocks: i.container.blocks };
});

function todoOf(index: number): number {
  const st = s.project.value.files[index]?.stats;
  return st ? st.missing + st.identical : 0;
}

/** 树选中：detail 是被选中的 tree-item 元素数组，用 id 反查下标。 */
function onTreeSelect(event: Event) {
  const detail = (event as CustomEvent).detail as Array<HTMLElement> | undefined;
  const id = detail?.[0]?.id ?? "";
  const m = /^file-(\d+)$/.exec(id);
  if (m) void props.project.selectFile(Number(m[1]));
}

function fileMenu(event: MouseEvent, index: number) {
  const file = s.project.value.files[index];
  if (!file) return;
  props.menu.openMenu(
    event,
    [
      { id: "open", label: "打开" },
      { id: "rescan", label: "重新扫描" },
      { id: "copy", label: "复制路径" },
      { id: "remove", label: "从项目中移出", hint: "Ctrl+W", danger: true, separator: true },
    ],
    (id) => {
      if (id === "open" || id === "rescan") void props.project.selectFile(index);
      else if (id === "copy") void navigator.clipboard.writeText(file.path);
      else if (id === "remove") props.project.removeFile(index);
    },
  );
}

function projectMenu(event: MouseEvent) {
  props.menu.openMenu(
    event,
    [
      { id: "new", label: "新建项目", hint: "Ctrl+N" },
      { id: "open", label: "打开项目…", hint: "Ctrl+O" },
      { id: "save", label: "保存项目", hint: "Ctrl+S" },
      { id: "add", label: "添加文件…", hint: "Ctrl+Shift+O", separator: true },
      { id: "rename", label: "重命名项目" },
      { id: "clearStats", label: "清空扫描统计", separator: true },
    ],
    (id) => {
      if (id === "new") props.project.newProject();
      else if (id === "open") void props.project.openProjectDialog();
      else if (id === "save") void props.project.saveProject(false);
      else if (id === "add") void props.project.addFiles();
      else if (id === "rename") {
        const next = window.prompt("项目名称", s.project.value.name);
        if (next !== null) props.project.renameProject(next);
      } else if (id === "clearStats") {
        for (const f of s.project.value.files) f.stats = null;
        props.project.log("已清空扫描统计");
      }
    },
  );
}
</script>

<template>
  <div class="explorer">
    <header class="section" @contextmenu="projectMenu">
      <span class="title">资源管理器</span>
      <!-- @ts-expect-error vscode-toolbar-container 是自定义元素 -->
      <vscode-toolbar-container class="actions">
        <!-- @ts-expect-error 自定义元素 -->
        <vscode-toolbar-button icon="new-file" label="添加文件（Ctrl+Shift+O）" @click="props.project.addFiles"></vscode-toolbar-button>
        <!-- @ts-expect-error 自定义元素 -->
        <vscode-toolbar-button icon="refresh" label="重新扫描当前文件" @click="props.project.reload"></vscode-toolbar-button>
        <!-- @ts-expect-error 自定义元素 -->
        <vscode-toolbar-button icon="save" label="保存项目（Ctrl+S）" @click="props.project.saveProject(false)"></vscode-toolbar-button>
      </vscode-toolbar-container>
    </header>

    <div class="tree-host" @contextmenu.self="projectMenu">
      <!-- @ts-expect-error vscode-tree 是自定义元素 -->
      <vscode-tree expand-mode="singleClick" indent-guides="onHover" @vsc-tree-select="onTreeSelect">
        <!-- @ts-expect-error vscode-tree-item 是自定义元素 -->
        <vscode-tree-item id="project-root" branch open>
          {{ s.project.value.name }}（{{ s.project.value.files.length }}）
          <!-- @ts-expect-error 自定义元素 -->
          <vscode-icon slot="icon-branch" name="folder" :size="16" />
          <!-- @ts-expect-error 自定义元素 -->
          <vscode-icon slot="icon-branch-opened" name="folder-opened" :size="16" />
        </vscode-tree-item>

        <!-- @ts-expect-error 自定义元素 -->
        <vscode-tree-item
          v-for="(file, index) in s.project.value.files"
          :id="'file-' + index"
          :key="file.path"
          :selected="index === s.project.value.active"
          @contextmenu.stop="fileMenu($event, index)"
        >
          {{ file.label || baseName(file.path) }}
          <span slot="description" class="desc">
            {{ file.source }}→{{ file.target }}<template v-if="todoOf(index)"> · {{ todoOf(index) }} 待翻译</template>
          </span>
          <!-- @ts-expect-error 自定义元素 -->
          <vscode-icon slot="icon-leaf" name="map" :size="16" />
        </vscode-tree-item>
      </vscode-tree>
    </div>

    <div class="actions-row">
      <button class="add" @click="props.project.addFiles"><Icon name="plus" :size="12" />添加文件</button>
    </div>

    <div v-if="props.tree?.length" class="files">
      <h3>工程文件</h3>
      <template v-for="[dir, list] in groups" :key="dir">
        <div v-if="dir" class="dir"><Icon name="folder" :size="13" />{{ dir }}</div>
        <button
          v-for="f in list"
          :key="f.path"
          class="fileRow"
          :class="{ active: f.path === props.activePath, dim: f.kind === 'binary' }"
          :title="f.path + ' · ' + formatBytes(f.size)"
          @click="emit('open', f.path)"
        >
          <Icon :name="kindIcon(f.kind)" :size="13" />
          <span class="fname">{{ leafName(f.path) }}</span>
          <span v-if="f.linked_json" class="linked" title="已关联同名 JSON">JSON</span>
        </button>
      </template>
    </div>

    <div v-if="currentInfo" class="current">
      <h3>当前文件</h3>
      <div class="kv"><span>大小</span><b>{{ currentInfo.size }}</b></div>
      <div class="kv"><span>组件 / 块</span><b>{{ currentInfo.components }} / {{ currentInfo.blocks }}</b></div>
      <div class="path mono">{{ s.archive.value }}</div>
    </div>
  </div>
</template>

<style scoped>
.explorer { display: flex; flex-direction: column; height: 100%; overflow: hidden; background: var(--bg-sidebar); }

.section {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 24px;
  padding: 0 var(--s2) 0 var(--s5);
  flex: 0 0 auto;
  cursor: default;
}
.title { font-size: var(--fs-xs); font-weight: 700; letter-spacing: 0.04em; color: var(--text-muted); text-transform: uppercase; }
.actions { display: flex; }

.tree-host { flex: 1; min-height: 0; overflow: auto; padding-bottom: var(--s4); }
vscode-tree { display: block; width: 100%; }

.actions-row { padding: 0 var(--s4) var(--s4); flex: 0 0 auto; }
.add {
  display: flex;
  align-items: center;
  gap: var(--s3);
  width: 100%;
  height: 22px;
  padding: 0 var(--s3);
  justify-content: flex-start;
  color: var(--text-muted);
  font-size: var(--fs-sm);
}
.add:hover { background: var(--bg-hover); color: var(--text); }

.current {
  flex: 0 0 auto;
  padding: var(--s4) var(--s5);
  border-top: 1px solid var(--border-subtle);
  display: flex;
  flex-direction: column;
  gap: var(--s2);
}
.kv { display: flex; justify-content: space-between; font-size: var(--fs-xs); color: var(--text-muted); }
.kv b { color: var(--text); font-weight: 500; font-variant-numeric: tabular-nums; }
.path { font-size: 10px; color: var(--text-faint); word-break: break-all; }
.desc { font-size: 10px; opacity: 0.75; }

.files { flex: 1; min-height: 0; overflow-y: auto; border-top: 1px solid var(--border-subtle); padding: var(--s3) var(--s3) var(--s5); }
.files h3 { padding: 0 var(--s2) var(--s3); }
.dir {
  display: flex; align-items: center; gap: var(--s3);
  padding: 2px var(--s3); margin-top: var(--s3);
  font-size: var(--fs-xs); color: var(--text-muted);
}
.fileRow {
  display: flex; align-items: center; gap: var(--s3);
  width: 100%; height: 22px; padding: 0 var(--s3) 0 var(--s5);
  border: none; border-radius: var(--r1); background: transparent;
  color: var(--text); font-size: var(--fs-sm); justify-content: flex-start;
  /* button 默认 text-align:center，不重置的话文件名会飘到中间 */
  text-align: left;
  cursor: default;
}
.fileRow:hover { background: var(--list-hover); }
.fileRow.active { background: var(--list-active-bg); color: var(--list-active-fg); }
.fileRow.dim { color: var(--text-faint); }
.fname { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.linked {
  flex: 0 0 auto; font-size: 9px; padding: 0 4px; border-radius: var(--r1);
  background: var(--accent); color: #fff;
}
</style>
