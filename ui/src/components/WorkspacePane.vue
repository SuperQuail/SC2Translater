<script setup lang="ts">
// SC2 文档的工作区：翻译键视图 + 原始文本 + JSON。
//
// **A 定稿**：单标签内切换 —— 一个视图区，右上角三段切换（键表 / 原始文本 / JSON）。
// 另外两版（并排、三个独立标签）在最初的设计对比里被否了，代码不再留。
// 三者操作同一份内存文档（见开发文档第 6 节）。

import { computed, ref } from "vue";

import Icon from "./Icon.vue";
import JsonEditor from "./JsonEditor.vue";
import KeyTable from "./KeyTable.vue";
import KeyValueEditor from "./KeyValueEditor.vue";
import type { ContextMenuApi } from "../composables/useContextMenu";
import { COMMON_LOCALES } from "../composables/useProject";
import type { Project } from "../composables/useProject";

const props = defineProps<{
  project: Project;
  menu: ContextMenuApi;
  initialView?: "table" | "kv" | "json";
}>();

const s = props.project.state;
const view = ref<"table" | "kv" | "json">(props.initialView ?? "table");
const table = ref<InstanceType<typeof KeyTable> | null>(null);

const VIEWS = [
  { id: "table" as const, label: "键表", icon: "list" },
  { id: "kv" as const, label: "原始文本", icon: "text" },
  { id: "json" as const, label: "JSON", icon: "code" },
];

function pick(v: "table" | "kv" | "json") {
  view.value = v;
  if (v === "table") requestAnimationFrame(() => table.value?.remeasure());
}

function readSelect(event: Event): string {
  return String((event.target as unknown as { value: string }).value ?? "");
}

/** 改语言对只影响当前文件 —— 它是项目配置，不是全局设置。 */
function onSource(event: Event) {
  void props.project.changeLanguages(readSelect(event), s.target.value);
}

function onTarget(event: Event) {
  void props.project.changeLanguages(s.source.value, readSelect(event));
}

/** 关联上的同名 JSON（没有就是 null）。 */
const linked = computed(() => {
  const p = s.archive.value;
  if (!p) return null;
  return p.replace(/\.[^.]+$/, ".json").split(/[\\/]/).pop() ?? null;
});

defineExpose({
  focusSearch: () => {
    view.value = "table";
    requestAnimationFrame(() => table.value?.focusSearch());
  },
});
</script>

<template>
  <div class="center">
    <!-- SC2 工具条：项目配置（语言对）+ 关联状态 + 提取 / 关联 / 写回 + 视图切换 -->
    <div class="sc2bar">
      <!-- 语言对是「这个文件的配置」，不是全局设置（见开发文档第 3 节） -->
      <!-- @ts-expect-error vscode-single-select 是自定义元素 -->
      <vscode-single-select class="lang" :value="s.source.value" title="原文语言（当前文件的配置）" @change="onSource">
        <vscode-option v-for="l in COMMON_LOCALES" :key="l.code" :value="l.code">{{ l.label }} · {{ l.code }}</vscode-option>
      </vscode-single-select>
      <Icon name="chevron" :size="12" />
      <!-- @ts-expect-error 自定义元素 -->
      <vscode-single-select class="lang" :value="s.target.value" title="目标语言（当前文件的配置）" @change="onTarget">
        <vscode-option v-for="l in COMMON_LOCALES" :key="l.code" :value="l.code">{{ l.label }} · {{ l.code }}</vscode-option>
      </vscode-single-select>

      <span class="divider"></span>

      <span
        class="badge"
        :class="{ on: Boolean(linked) }"
        :title="linked ? '已关联同名 JSON：' + linked : '同目录没有同名 JSON'"
      >
        <Icon :name="linked ? 'check' : 'info'" :size="12" />
        {{ linked ?? "无同名 JSON" }}
      </span>
      <!-- @ts-expect-error vscode-button 是自定义元素 -->
      <vscode-button secondary icon="export" title="从归档导出翻译键 JSON" @click="props.project.exportJson">提取</vscode-button>
      <!-- @ts-expect-error 自定义元素 -->
      <vscode-button secondary icon="arrow-down" title="换一份 JSON 关联" @click="props.project.importJson">关联</vscode-button>
      <!-- @ts-expect-error 自定义元素 -->
      <vscode-button icon="save" title="把译文写回归档（不覆盖原件）" @click="props.project.applyToArchive">写回</vscode-button>

      <span class="grow"></span>

      <div class="seg">
        <button
          v-for="v in VIEWS"
          :key="v.id"
          class="segBtn"
          :class="{ on: view === v.id }"
          @click="pick(v.id)"
        >
          <Icon :name="v.icon" :size="12" />{{ v.label }}
        </button>
      </div>
    </div>

    <div class="panel-body">
      <KeyTable v-if="view === 'table'" ref="table" :project="props.project" :menu="props.menu" />
      <KeyValueEditor v-else-if="view === 'kv'" :project="props.project" />
      <JsonEditor v-else :project="props.project" />
    </div>
  </div>
</template>

<style scoped>
.center {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  background: var(--bg-editor);
  overflow: hidden;
}

.sc2bar {
  flex: 0 0 auto;
  display: flex;
  align-items: center;
  gap: var(--s3);
  padding: var(--s2) var(--s5);
  border-bottom: 1px solid var(--border-subtle);
  flex-wrap: nowrap;
  overflow: hidden;
}
.grow { flex: 1; }

.divider { width: 1px; height: 16px; background: var(--border-subtle); margin: 0 var(--s2); }
.lang { width: 132px; flex: 0 0 auto; }

.badge {
  display: inline-flex; align-items: center; gap: var(--s3);
  min-width: 0; overflow: hidden; white-space: nowrap; text-overflow: ellipsis;
  height: 22px; padding: 0 var(--s4);
  border-radius: 999px;
  background: var(--bg-raised);
  border: 1px solid var(--border-subtle);
  color: var(--text-faint);
  font-size: var(--fs-xs);
}
.badge.on { border-color: var(--accent); color: var(--text); }

.seg { flex: 0 0 auto; display: flex; border: 1px solid var(--border-subtle); border-radius: var(--r2); overflow: hidden; }
.segBtn {
  display: inline-flex; align-items: center; gap: var(--s3);
  height: 24px; padding: 0 var(--s4);
  border: none; border-radius: 0; background: transparent;
  color: var(--text-muted); font-size: var(--fs-xs);
}
.segBtn:hover { background: var(--list-hover); color: var(--text); }
.segBtn.on { background: var(--list-active-bg); color: var(--list-active-fg); }

.panel-body { flex: 1; min-height: 0; display: flex; }
</style>
