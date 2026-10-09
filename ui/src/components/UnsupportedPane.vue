<script setup lang="ts">
// 打不开的文件：明说为什么，不给一个空白页让人猜。

import { formatBytes } from "../api";
import type { OpenFile } from "../api/types";
import Icon from "./Icon.vue";

const props = defineProps<{ file: OpenFile }>();
defineEmits<{ (e: "reveal"): void }>();
</script>

<template>
  <div class="unsupported">
    <div class="card">
      <Icon name="warn" :size="28" />
      <h2>不受支持的文件格式</h2>
      <p class="path mono">{{ props.file.path }}</p>
      <p class="reason">{{ props.file.reason ?? "这个文件不是本工具能打开的类型。" }}</p>
      <p class="dim">大小 {{ formatBytes(props.file.size) }}。<br />要改它得用专门的工具（模型、音频、贴图各有各的编辑器）。</p>
      <div class="row">
        <!-- @ts-expect-error vscode-button 是自定义元素 -->
        <vscode-button secondary icon="folder-opened" @click="$emit('reveal')">在资源管理器中显示</vscode-button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.unsupported { flex: 1; min-width: 0; display: grid; place-items: center; background: var(--bg-editor); padding: var(--s8); }
.card { max-width: 520px; display: flex; flex-direction: column; align-items: center; text-align: center; gap: var(--s4); }
h2 { font-size: var(--fs-lg); font-weight: 600; margin: 0; }
.path { color: var(--text); font-size: var(--fs-sm); margin: 0; word-break: break-all; }
.reason { color: var(--text-muted); font-size: var(--fs-md); line-height: 1.75; margin: 0; }
.dim { color: var(--text-faint); font-size: var(--fs-sm); line-height: 1.7; margin: 0; }
.row { margin-top: var(--s3); }
</style>
