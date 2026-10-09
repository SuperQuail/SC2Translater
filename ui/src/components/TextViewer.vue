<script setup lang="ts">
// 文本文件查看器。**不是完整编辑器** —— 没有高亮、没有多光标（见开发文档第 8 节）。

import { computed, ref, watch } from "vue";

import { formatBytes } from "../api";
import type { OpenFile } from "../api/types";
import Icon from "./Icon.vue";

const props = defineProps<{ file: OpenFile; dirty?: boolean }>();
const emit = defineEmits<{ (e: "edit"): void }>();

const draft = ref(props.file.text ?? "");
watch(
  () => props.file,
  (f) => {
    draft.value = f.text ?? "";
  },
);

const lines = computed(() => draft.value.split("\n").length);

function onInput(event: Event) {
  draft.value = (event.target as HTMLTextAreaElement).value;
  emit("edit");
}
</script>

<template>
  <div class="text-viewer">
    <div class="bar">
      <Icon name="file" :size="14" />
      <span class="name mono">{{ props.file.path }}</span>
      <span class="dim">{{ formatBytes(props.file.size) }} · {{ lines }} 行</span>
      <span v-if="props.dirty" class="dirty">● 未保存</span>
      <span class="grow"></span>
      <span class="hint">看 + 小改用；不做高亮与补全</span>
    </div>
    <textarea class="editor mono" :value="draft" spellcheck="false" @input="onInput"></textarea>
  </div>
</template>

<style scoped>
.text-viewer { flex: 1; min-width: 0; display: flex; flex-direction: column; background: var(--bg-editor); }
.bar {
  flex: 0 0 auto;
  display: flex; align-items: center; gap: var(--s4);
  padding: var(--s3) var(--s5);
  border-bottom: 1px solid var(--border-subtle);
  font-size: var(--fs-sm);
}
.name { color: var(--text); }
.dim { color: var(--text-faint); }
.dirty { color: var(--warn-text); }
.grow { flex: 1; }
.hint { color: var(--text-faint); font-size: var(--fs-xs); }

.editor {
  flex: 1; min-height: 0;
  width: 100%;
  padding: var(--s5);
  border: none; outline: none; resize: none;
  background: var(--bg-editor);
  color: var(--text);
  font-size: var(--fs-md);
  line-height: 1.6;
  tab-size: 4;
}
</style>
