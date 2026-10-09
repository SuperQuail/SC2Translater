<script setup lang="ts">
// 中栏容器：在「键表」和「JSON 编辑器」之间切换。
// 两种视图操作的是同一份工作表数据（表格是主，JSON 是进出的另一条路）。

import { ref } from "vue";

import JsonEditor from "./JsonEditor.vue";
import KeyTable from "./KeyTable.vue";
import type { Project } from "../composables/useProject";

const props = defineProps<{ project: Project; initialView?: "table" | "json" }>();
const view = ref<"table" | "json">(props.initialView ?? "table");
const table = ref<InstanceType<typeof KeyTable> | null>(null);

function focusSearch() {
  view.value = "table";
  table.value?.focusSearch();
}

defineExpose({ focusSearch });
</script>

<template>
  <div class="center">
    <div class="tabs">
      <button :class="{ active: view === 'table' }" @click="view = 'table'">键表</button>
      <button :class="{ active: view === 'json' }" @click="view = 'json'">JSON 编辑器</button>
    </div>
    <KeyTable v-show="view === 'table'" ref="table" :project="props.project" />
    <JsonEditor v-show="view === 'json'" :project="props.project" />
  </div>
</template>

<style scoped>
.center { flex: 1; min-width: 0; display: flex; flex-direction: column; }

.tabs {
  display: flex;
  gap: var(--s2);
  padding: var(--s3) var(--s5) 0;
  background: var(--bg-panel);
}

.tabs button {
  height: 28px;
  padding: 0 var(--s5);
  background: transparent;
  border-color: transparent;
  border-bottom: 2px solid transparent;
  border-radius: var(--r2) var(--r2) 0 0;
  color: var(--text-faint);
  font-size: var(--fs-sm);
}

.tabs button:hover { color: var(--text); background: var(--bg-hover); }
.tabs button.active { color: var(--text); border-bottom-color: var(--accent); background: transparent; }
</style>
