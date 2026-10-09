<script setup lang="ts">
// 图片查看器：适应窗口 / 原始大小两档，够看参考图就行。
//
// 图片数据由后端读成 data URL 回传（不走 Tauri 的 asset 协议 —— 那条路要配 scope，
// 而且没在真机上验过；一张参考图走 IPC 完全够用，少一个配置面）。

import { computed, onMounted, ref } from "vue";

import { api, formatBytes, isTauri } from "../api";
import type { OpenFile } from "../api/types";
import Icon from "./Icon.vue";

const props = defineProps<{ file: OpenFile; root: string }>();

const fit = ref(true);
const src = ref(props.file.url ?? "");
const error = ref("");

onMounted(async () => {
  if (src.value || !isTauri) return;
  try {
    src.value = await api.readImageDataUrl(props.root, props.file.path);
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  }
});

const sizeLabel = computed(() => formatBytes(props.file.size));
</script>

<template>
  <div class="image-viewer">
    <div class="bar">
      <Icon name="file-media" :size="14" />
      <span class="name mono">{{ props.file.path }}</span>
      <span class="dim">{{ sizeLabel }}</span>
      <span class="grow"></span>
      <button class="toggle" :class="{ on: fit }" @click="fit = !fit">{{ fit ? "适应窗口" : "原始大小" }}</button>
    </div>
    <div class="stage" :class="{ fit }">
      <img v-if="src" :src="src" :alt="props.file.path" />
      <p v-else class="err">{{ error || "正在读取图片…" }}</p>
    </div>
  </div>
</template>

<style scoped>
.image-viewer { flex: 1; min-width: 0; display: flex; flex-direction: column; background: var(--bg-editor); }
.bar {
  flex: 0 0 auto;
  display: flex; align-items: center; gap: var(--s4);
  padding: var(--s3) var(--s5);
  border-bottom: 1px solid var(--border-subtle);
  font-size: var(--fs-sm);
}
.name { color: var(--text); }
.dim { color: var(--text-faint); }
.grow { flex: 1; }
.toggle {
  height: 24px; padding: 0 var(--s4);
  border: 1px solid var(--border-subtle); border-radius: var(--r1);
  background: transparent; color: var(--text-muted); font-size: var(--fs-xs);
}
.toggle.on { border-color: var(--accent); color: var(--text); }

.stage { flex: 1; min-height: 0; overflow: auto; display: grid; place-items: center; padding: var(--s6); }
.stage.fit img { max-width: 100%; max-height: 100%; object-fit: contain; }
.stage img { display: block; }
.err { color: var(--text-faint); font-size: var(--fs-sm); }
</style>
