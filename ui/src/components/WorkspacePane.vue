<script setup lang="ts">
// 中间工作区：<vscode-tabs panel> 直接给出 VS Code 的标签条 + 面板。
// 三种视图操作的是同一份数据：键表（逐条）/ 键值文本 / JSON。

import { ref } from "vue";

import Icon from "./Icon.vue";
import JsonEditor from "./JsonEditor.vue";
import KeyTable from "./KeyTable.vue";
import KeyValueEditor from "./KeyValueEditor.vue";
import type { ContextMenuApi } from "../composables/useContextMenu";
import type { Project } from "../composables/useProject";

const props = defineProps<{
  project: Project;
  menu: ContextMenuApi;
  initialView?: "table" | "kv" | "json";
}>();

const INDEX: Record<string, number> = { table: 0, kv: 1, json: 2 };
const selected = ref(INDEX[props.initialView ?? "table"] ?? 0);
const table = ref<InstanceType<typeof KeyTable> | null>(null);

function onTabSelect(event: Event) {
  const detail = (event as CustomEvent).detail as { selectedIndex?: number } | undefined;
  if (typeof detail?.selectedIndex === "number") selected.value = detail.selectedIndex;
  // 键表是窗口化渲染的：面板刚显示时视口高度还是 0，要重新量一次
  if (selected.value === 0) requestAnimationFrame(() => table.value?.remeasure());
}

function focusSearch() {
  selected.value = 0;
  requestAnimationFrame(() => table.value?.focusSearch());
}

defineExpose({ focusSearch });
</script>

<template>
  <div class="center">
    <!-- @ts-expect-error vscode-tabs 是自定义元素 -->
    <vscode-tabs panel :selected-index="selected" @vsc-tabs-select="onTabSelect">
      <!-- @ts-expect-error 自定义元素 -->
      <vscode-tab-header slot="header">
        <span class="tab-label"><Icon name="list" :size="14" />键表</span>
      </vscode-tab-header>
      <!-- @ts-expect-error 自定义元素 -->
      <vscode-tab-panel>
        <div class="panel-body">
          <KeyTable ref="table" :project="props.project" :menu="props.menu" />
        </div>
      </vscode-tab-panel>

      <!-- @ts-expect-error 自定义元素 -->
      <vscode-tab-header slot="header">
        <span class="tab-label"><Icon name="text" :size="14" />键值文本</span>
      </vscode-tab-header>
      <!-- @ts-expect-error 自定义元素 -->
      <vscode-tab-panel>
        <div class="panel-body"><KeyValueEditor :project="props.project" /></div>
      </vscode-tab-panel>

      <!-- @ts-expect-error 自定义元素 -->
      <vscode-tab-header slot="header">
        <span class="tab-label"><Icon name="code" :size="14" />JSON</span>
      </vscode-tab-header>
      <!-- @ts-expect-error 自定义元素 -->
      <vscode-tab-panel>
        <div class="panel-body"><JsonEditor :project="props.project" /></div>
      </vscode-tab-panel>
    </vscode-tabs>
  </div>
</template>

<style scoped>
.center {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  background: var(--bg-app);
  overflow: hidden;
}

vscode-tabs { flex: 1; min-height: 0; display: flex; flex-direction: column; }

.tab-label { display: inline-flex; align-items: center; gap: var(--s3); }

.panel-body { flex: 1; min-height: 0; display: flex; height: 100%; }
</style>
