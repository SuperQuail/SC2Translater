<script setup lang="ts">
// 右栏：当前条目的完整信息与长文本编辑。
// 表格里的输入框适合快速过一遍；需要看清整段文本（带内联标记的那种）就在这里改。

import { computed, ref, watch } from "vue";

import { STATUS_META, statusLabel } from "../api";
import type { Project } from "../composables/useProject";

const props = defineProps<{ project: Project }>();
const s = props.project.state;

const entry = computed(() => props.project.selectedEntry.value);
const targetDraft = ref("");
const copied = ref("");

watch(
  () => [s.selected.value, entry.value?.target],
  () => {
    targetDraft.value = entry.value?.target ?? "";
  },
  { immediate: true },
);

function commit() {
  if (entry.value) props.project.editTarget(entry.value, targetDraft.value);
}

function useSource() {
  if (!entry.value) return;
  targetDraft.value = entry.value.source ?? "";
  props.project.editTarget(entry.value, targetDraft.value);
}

function clearTarget() {
  targetDraft.value = "";
  if (entry.value) props.project.editTarget(entry.value, "");
}

async function copy(text: string, what: string) {
  try {
    await navigator.clipboard.writeText(text);
    copied.value = what;
    window.setTimeout(() => (copied.value = ""), 1200);
  } catch {
    copied.value = "";
  }
}

const tone = computed(() => STATUS_META[entry.value?.status ?? ""]?.tone ?? "muted");
const label = computed(() => statusLabel(entry.value?.status ?? ""));
const sourceLength = computed(() => (entry.value?.source ?? "").length);
const targetLength = computed(() => (targetDraft.value ?? "").length);
const tokens = computed(() => props.project.selectedMarkup.value);
</script>

<template>
  <aside class="detail">
    <template v-if="entry">
      <div class="head">
        <span class="badge" :class="tone"><span class="dot"></span>{{ label }}</span>
        <span class="file mono">{{ entry.file }}</span>
      </div>

      <section class="section">
        <h3>键</h3>
        <div class="keyrow">
          <code class="key mono">{{ entry.key }}</code>
          <!-- @ts-expect-error vscode-button 是自定义元素 -->
          <vscode-button secondary @click="copy(entry.key, 'key')">{{ copied === "key" ? "已复制" : "复制" }}</vscode-button>
        </div>
        <div v-if="entry.origin?.length" class="origin">
          来源：<span class="mono">{{ entry.origin.join("、") }}</span>
        </div>
      </section>

      <section class="section">
        <h3>原文 <span class="len">{{ sourceLength }} 字符</span></h3>
        <div class="source-box">{{ entry.source || "（空）" }}</div>
        <div v-if="tokens.length" class="tokens">
          <div class="tokens-title">内联标记（回填时必须整体保留）</div>
          <code v-for="(t, i) in tokens" :key="i" class="token mono">{{ t }}</code>
        </div>
      </section>

      <section class="section grow">
        <h3>
          译文 <span class="len">{{ targetLength }} 字符</span>
        </h3>
        <!-- @ts-expect-error vscode-textarea 是自定义元素 -->
        <vscode-textarea
          class="target-area"
          :value="targetDraft"
          resize="vertical"
          :placeholder="entry.source ? '在这里写译文…（Ctrl+D 用原文填入）' : '这条没有原文'"
          @input="targetDraft = ($event.target as unknown as { value: string }).value"
          @change="commit"
        ></vscode-textarea>
        <div class="row-actions">
          <!-- @ts-expect-error 自定义元素 -->
          <vscode-button secondary icon="arrow-down" @click="useSource">用原文填入</vscode-button>
          <!-- @ts-expect-error 自定义元素 -->
          <vscode-button secondary icon="clear-all" @click="clearTarget">清空</vscode-button>
          <!-- @ts-expect-error 自定义元素 -->
          <vscode-button secondary icon="copy" @click="copy(targetDraft, 'target')">
            {{ copied === "target" ? "已复制" : "复制译文" }}
          </vscode-button>
        </div>
      </section>
    </template>

    <div v-else class="placeholder">
      在左侧表格里选一条键，这里会显示它的完整信息。
    </div>
  </aside>
</template>

<style scoped>
.detail {
  width: 100%;
  display: flex;
  flex-direction: column;
  gap: var(--s5);
  padding: var(--s5);
  background: var(--bg-sidebar);
  border-left: 1px solid var(--border-sidebar);
  overflow-y: auto;
}

.head { display: flex; align-items: center; justify-content: space-between; gap: var(--s3); }

.badge {
  display: inline-flex;
  align-items: center;
  gap: var(--s3);
  padding: 2px var(--s4);
  border-radius: 999px;
  font-size: var(--fs-xs);
  background: var(--bg-raised);
  border: 1px solid var(--border-subtle);
  color: var(--text-muted);
}
.badge .dot { width: 6px; height: 6px; border-radius: 50%; background: var(--slate-9); }
.badge.err .dot { background: var(--err); }
.badge.warn .dot { background: var(--warn); }
.badge.ok .dot { background: var(--ok); }
.badge.info .dot { background: var(--accent); }

.file { font-size: var(--fs-xs); color: var(--text-faint); }

.section { display: flex; flex-direction: column; gap: var(--s3); }
.section.grow { flex: 1; min-height: 0; }

.section h3 { display: flex; align-items: baseline; gap: var(--s3); }
.len { font-size: var(--fs-xs); color: var(--text-faint); font-weight: 400; text-transform: none; letter-spacing: 0; }

.keyrow { display: flex; align-items: flex-start; gap: var(--s3); }
.key {
  flex: 1;
  padding: var(--s3) var(--s4);
  background: var(--bg-inset);
  border: 1px solid var(--border-subtle);
  border-radius: var(--r2);
  word-break: break-all;
  line-height: 1.45;
}



.origin { font-size: var(--fs-xs); color: var(--text-faint); word-break: break-all; }

.source-box {
  padding: var(--s4);
  background: var(--bg-inset);
  border-left: 2px solid var(--blue-7);
  border: 1px solid var(--border-subtle);
  border-radius: var(--r2);
  font-size: var(--fs-sm);
  color: var(--text-muted);
  white-space: pre-wrap;
  word-break: break-word;
  max-height: 190px;
  overflow-y: auto;
  line-height: 1.55;
}

.tokens { display: flex; flex-wrap: wrap; gap: var(--s2); align-items: center; }
.tokens-title { width: 100%; font-size: var(--fs-xs); color: var(--warn-text); }
.token {
  padding: 1px var(--s3);
  border-radius: var(--r1);
  background: var(--bg-raised);
  border: 1px solid var(--border-subtle);
  font-size: var(--fs-xs);
  color: var(--text-muted);
}

.target-area { flex: 1; min-height: 140px; width: 100%; }

.row-actions { display: flex; gap: var(--s3); flex-wrap: wrap; align-items: center; }

.placeholder { color: var(--text-faint); font-size: var(--fs-sm); line-height: 1.7; padding: var(--s6) 0; }
</style>
