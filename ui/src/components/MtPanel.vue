<script setup lang="ts">
// 详情面板里的「机翻参考」一块。
//
// 刻意与译文框分开：机翻只在这里出现，用户点「用作译文」才会进译文框。

import { computed } from "vue";

import type { MtApi } from "../composables/useMt";
import type { Entry } from "../api/types";
import Icon from "./Icon.vue";

const props = defineProps<{ mt: MtApi; entry: Entry | null }>();
const emit = defineEmits<{ (e: "use", text: string): void; (e: "settings"): void }>();

const providerLabel = computed(() =>
  props.mt.config.value.provider === "offline" ? "离线（术语拼装）" : "在线 " + props.mt.config.value.model,
);
</script>

<template>
  <section class="mt">
    <h3>
      机翻参考
      <span class="dim">{{ providerLabel }}<template v-if="mt.suggestion.value?.cached"> · 缓存</template></span>
    </h3>

    <div class="row">
      <!-- @ts-expect-error vscode-button 是自定义元素 -->
      <vscode-button
        secondary
        icon="wand"
        :disabled="mt.busy.value || !entry?.source"
        @click="mt.translate(entry?.source)"
      >机翻这条</vscode-button>
      <!-- @ts-expect-error 自定义元素 -->
      <vscode-button secondary icon-only icon="settings-gear" title="机翻设置" @click="emit('settings')"></vscode-button>
    </div>

    <div v-if="mt.error.value" class="err">{{ mt.error.value }}</div>
    <div v-if="mt.busy.value" class="dim">正在生成参考…</div>

    <template v-if="mt.suggestion.value">
      <div class="box">{{ mt.suggestion.value.text }}</div>
      <div class="note"><Icon name="info" :size="12" />{{ mt.suggestion.value.note }}</div>

      <div v-if="mt.suggestion.value.terms.length" class="terms">
        <span class="termsTitle">命中术语</span>
        <span v-for="t in mt.suggestion.value.terms" :key="t.id" class="term mono">{{ t.source }} → {{ t.target }}</span>
      </div>

      <div class="row">
        <!-- @ts-expect-error 自定义元素 -->
        <vscode-button @click="emit('use', mt.suggestion.value.text)">用作译文</vscode-button>
        <!-- @ts-expect-error 自定义元素 -->
        <vscode-button secondary @click="mt.clear()">清空</vscode-button>
      </div>
      <p class="warn">机翻仅供参考 —— 点「用作译文」才会写进译文框，写回地图前请自行校对。</p>
    </template>
  </section>
</template>

<style scoped>
.mt { display: flex; flex-direction: column; gap: var(--s3); border-top: 1px solid var(--border-subtle); padding-top: var(--s4); }

h3 { display: flex; align-items: baseline; gap: var(--s3); }
.dim { font-size: var(--fs-xs); color: var(--text-faint); font-weight: 400; text-transform: none; letter-spacing: 0; }

.row { display: flex; gap: var(--s3); align-items: center; flex-wrap: wrap; }

.box {
  padding: var(--s4);
  background: var(--bg-inset);
  border: 1px solid var(--border-subtle);
  border-left: 2px solid var(--warn);
  border-radius: var(--r2);
  font-size: var(--fs-sm);
  line-height: 1.6;
  white-space: pre-wrap;
  word-break: break-word;
  max-height: 160px;
  overflow-y: auto;
}

.note { display: flex; align-items: flex-start; gap: var(--s3); font-size: var(--fs-xs); color: var(--warn-text); }
.err { font-size: var(--fs-xs); color: var(--err-text); }

.terms { display: flex; flex-wrap: wrap; gap: var(--s2); align-items: center; }
.termsTitle { width: 100%; font-size: var(--fs-xs); color: var(--text-faint); }
.term {
  padding: 1px var(--s3);
  border-radius: var(--r1);
  background: var(--bg-raised);
  border: 1px solid var(--border-subtle);
  font-size: 11px;
  color: var(--text-muted);
}

.warn { margin: 0; font-size: var(--fs-xs); color: var(--text-faint); line-height: 1.6; }
</style>
