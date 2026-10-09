<script setup lang="ts">
// 底栏：进度 + 统计 + 脏标记 + 最近一条结果/错误。

import { computed } from "vue";

import { formatBytes } from "../api";
import type { Project } from "../composables/useProject";

const props = defineProps<{ project: Project }>();
const s = props.project.state;

const percent = computed(() => props.project.progress.value.percent);
const shown = computed(() => props.project.filtered.value.length);

const tone = computed(() => {
  if (s.error.value) return "err";
  if (percent.value >= 100) return "ok";
  if (percent.value > 0) return "warn";
  return "err";
});

const resultLine = computed(() => {
  const r = s.result.value;
  if (!r) return "";
  return "产物 " + formatBytes(r.bytes_out) + "（原件 " + formatBytes(r.bytes_in) + "） · 回读校验" + (r.ok ? "通过" : "失败");
});
</script>

<template>
  <footer class="statusbar">
    <div class="progress" :class="tone">
      <div class="track"><div class="fill" :style="{ width: percent + '%' }"></div></div>
      <span class="value">{{ percent.toFixed(1) }}%</span>
    </div>

    <span class="sep"></span>
    <span class="item">键 <b>{{ props.project.counts.value.all }}</b></span>
    <span class="item">待翻译 <b class="warn">{{ props.project.counts.value.todo }}</b></span>
    <span class="item">已翻译 <b class="ok">{{ props.project.counts.value.translated }}</b></span>
    <span class="item">当前筛选 <b>{{ shown }}</b></span>

    <span v-if="props.project.dirtyCount.value" class="chip-dirty">
      未保存 {{ props.project.dirtyCount.value }} 处
      <button class="mini" @click="props.project.revertEdits">放弃</button>
    </span>

    <span class="spacer"></span>

    <span v-if="s.busy.value" class="item busy">
      <span class="spinner"></span>{{ s.busyText.value }}
    </span>
    <span v-else-if="s.error.value" class="item err" :title="s.error.value">{{ s.error.value }}</span>
    <span v-else-if="resultLine" class="item ok">{{ resultLine }}</span>
    <span v-else class="item faint">{{ s.logs.value[0] ?? "就绪" }}</span>
  </footer>
</template>

<style scoped>
.statusbar {
  height: var(--h-statusbar);
  flex: 0 0 auto;
  display: flex;
  align-items: center;
  gap: var(--s5);
  padding: 0 var(--s5);
  background: var(--bg-panel);
  border-top: 1px solid var(--border-subtle);
  font-size: var(--fs-sm);
  color: var(--text-muted);
  overflow: hidden;
}

.progress { display: flex; align-items: center; gap: var(--s3); }
.track { width: 148px; height: 5px; border-radius: 999px; background: var(--slate-5); overflow: hidden; }
.fill { height: 100%; transition: width var(--t-base); background: var(--slate-9); }
.progress.ok .fill { background: var(--ok); }
.progress.warn .fill { background: var(--warn); }
.progress.err .fill { background: var(--err); }
.value { font-variant-numeric: tabular-nums; color: var(--text); }

.sep { width: 1px; height: 16px; background: var(--border-subtle); }

.item { white-space: nowrap; }
.item b { color: var(--text); font-weight: 600; font-variant-numeric: tabular-nums; }
.item b.warn { color: var(--warn-text); }
.item b.ok { color: var(--ok-text); }
.item.err { color: var(--err-text); overflow: hidden; text-overflow: ellipsis; max-width: 46ch; }
.item.ok { color: var(--ok-text); }
.item.faint { color: var(--text-faint); overflow: hidden; text-overflow: ellipsis; max-width: 52ch; }

.spacer { flex: 1; }

.chip-dirty {
  display: inline-flex;
  align-items: center;
  gap: var(--s3);
  padding: 1px var(--s3);
  border-radius: var(--r2);
  background: var(--blue-4);
  color: var(--accent-text);
}

.mini {
  height: 20px;
  padding: 0 var(--s3);
  font-size: var(--fs-xs);
  background: transparent;
  border-color: transparent;
  color: var(--accent-text);
}
.mini:hover { background: var(--blue-5); border-color: transparent; }

.busy { display: inline-flex; align-items: center; gap: var(--s3); color: var(--accent-text); }

.spinner {
  width: 11px;
  height: 11px;
  border-radius: 50%;
  border: 2px solid var(--blue-7);
  border-top-color: transparent;
  animation: spin 700ms linear infinite;
}

@keyframes spin { to { transform: rotate(360deg); } }
</style>
