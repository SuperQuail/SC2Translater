<script setup lang="ts">
// 项目内文件标签条：在一个项目里的多张地图/模组之间秒切。
//
// 每个标签显示：名称 + 待翻译数量徽标 + 语言对；右键出菜单（重命名 / 移出 / 定位文件）。

import { computed, nextTick, ref } from "vue";

import { baseName } from "../api";
import type { ContextMenuApi } from "../composables/useContextMenu";
import type { Project } from "../composables/useProject";

const props = defineProps<{
  project: Project;
  menu: ContextMenuApi;
  /** 只在这一版里保留标签条上的「添加文件」（三处重复，选一处）。 */
  showAdd?: boolean;
}>();
const s = props.project.state;

const editingIndex = ref(-1);
const editingText = ref("");
const inputRef = ref<HTMLInputElement | null>(null);

async function startRename(index: number) {
  editingIndex.value = index;
  editingText.value = s.project.value.files[index]?.label ?? "";
  await nextTick();
  inputRef.value?.focus();
  inputRef.value?.select();
}

function commitRename() {
  if (editingIndex.value >= 0) {
    props.project.renameFileLabel(editingIndex.value, editingText.value);
  }
  editingIndex.value = -1;
}

function todoOf(index: number): number {
  const st = s.project.value.files[index]?.stats;
  return st ? st.missing + st.identical : 0;
}

function subtitle(file: { source: string; target: string }): string {
  return file.source + "→" + file.target;
}

function onContext(event: MouseEvent, index: number) {
  props.menu.openMenu(
    event,
    [
      { id: "rename", label: "重命名标签" },
      { id: "reload", label: "重新扫描这个文件" },
      { id: "reveal", label: "在资源管理器中显示", separator: true },
      { id: "remove", label: "从项目中移出", danger: true, separator: true },
    ],
    (id) => {
      if (id === "rename") void startRename(index);
      else if (id === "reload") void props.project.selectFile(index);
      else if (id === "reveal") void props.project.revealOutput();
      else if (id === "remove") props.project.removeFile(index);
    },
  );
}

const total = computed(() => s.project.value.files.length);
</script>

<template>
  <div class="tabs">
    <div class="strip">
      <div
        v-for="(file, index) in s.project.value.files"
        :key="file.path"
        class="tab"
        :class="{ active: index === s.project.value.active }"
        :title="file.path + '  ·  ' + subtitle(file)"
        @click="props.project.selectFile(index)"
        @dblclick="startRename(index)"
        @contextmenu="onContext($event, index)"
      >
        <input
          v-if="editingIndex === index"
          ref="inputRef"
          v-model="editingText"
          class="rename"
          @click.stop
          @blur="commitRename"
          @keyup.enter="commitRename"
          @keyup.esc="editingIndex = -1"
        />
        <template v-else>
          <span class="name">{{ file.label || baseName(file.path) }}</span>
          <span class="langs">{{ subtitle(file) }}</span>
          <!-- @ts-expect-error vscode-badge 是自定义元素 -->
          <vscode-badge v-if="todoOf(index)" class="badge">{{ todoOf(index) }}</vscode-badge>
          <!-- @ts-expect-error 自定义元素 -->
          <vscode-badge v-else-if="file.stats" class="badge done">✓</vscode-badge>
          <span class="close" title="从项目中移出" @click.stop="props.project.removeFile(index)">×</span>
        </template>
      </div>
    </div>

    <button
      v-if="props.showAdd"
      class="add"
      title="添加文件（Ctrl+Shift+O）"
      @click="props.project.addFiles"
    >
      + 添加文件
    </button>
    <span v-if="total > 1" class="hint">Ctrl+Tab 切换 · Ctrl+1…9 直跳</span>
  </div>
</template>

<style scoped>
.tabs {
  display: flex;
  align-items: center;
  gap: var(--s4);
  height: var(--h-tabbar);
  flex: 0 0 auto;
  padding: 0 var(--s4) 0 0;
  background: var(--bg-tabbar);
  border-bottom: 1px solid var(--border-tabbar);
}

.strip {
  display: flex;
  align-items: stretch;
  gap: 2px;
  overflow-x: auto;
  overflow-y: hidden;
  flex: 1;
  min-width: 0;
  height: 100%;
  scrollbar-width: thin;
}
.strip::-webkit-scrollbar { height: 4px; }

/* VS Code 式标签：方角、彼此用细线分隔、活动标签顶部一条强调线。 */
.tab {
  display: flex;
  align-items: center;
  gap: var(--s3);
  padding: 0 var(--s3) 0 var(--s5);
  min-width: 130px;
  max-width: 280px;
  height: 100%;
  border: none;
  border-right: 1px solid var(--tab-border);
  border-radius: 0;
  background: var(--tab-inactive-bg);
  color: var(--tab-inactive-fg);
  font-size: var(--fs-md);
  cursor: default;
  user-select: none;
  white-space: nowrap;
  position: relative;
}

.tab:first-child { border-left: 1px solid var(--tab-border); }
.tab:hover { background: var(--tab-hover-bg); color: var(--text); }
.tab.active { background: var(--tab-active-bg); color: var(--tab-active-fg); }
.tab.active::before {
  content: "";
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  height: 1px;
  background: var(--tab-active-border-top);
}

.name { overflow: hidden; text-overflow: ellipsis; }
.langs { font-size: 10px; color: var(--text-faint); font-family: var(--font-mono); }

.badge { flex: 0 0 auto; }
.badge.done { opacity: 0.8; }

.close {
  opacity: 0;
  color: var(--text-faint);
  font-size: 14px;
  line-height: 1;
  padding: 0 2px;
  border-radius: var(--r1);
}
.tab:hover .close { opacity: 1; }
.close:hover { color: var(--err-text); background: var(--bg-active); }

.rename {
  height: 22px;
  font-size: var(--fs-sm);
  padding: 0 var(--s2);
  width: 100%;
}

.add { height: 22px; flex: 0 0 auto; font-size: var(--fs-sm); color: var(--tab-inactive-fg); }
.add:hover { color: var(--text); }
.hint { font-size: var(--fs-xs); color: var(--text-faint); flex: 0 0 auto; }
</style>
