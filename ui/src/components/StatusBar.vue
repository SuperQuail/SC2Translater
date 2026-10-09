<script setup lang="ts">
// 底栏：VS Code 式的一条细状态条 —— 进度 + 统计 + 当前动作。

import { computed } from "vue";

import { formatBytes } from "../api";
import type { Project } from "../composables/useProject";
import Icon from "./Icon.vue";

const props = defineProps<{ project: Project }>();
const s = props.project.state;

const percent = computed(() => props.project.progress.value.percent);
const shown = computed(() => props.project.filtered.value.length);

const resultLine = computed(() => {
  const r = s.result.value;
  if (!r) return "";
  return "产物 " + formatBytes(r.bytes_out) + " · 回读" + (r.ok ? "通过" : "失败");
});
</script>

<template>
  <footer class="statusbar">
    <span class="brand">
      <Icon name="files" :size="12" />
      项目 {{ s.project.value.files.length }}
    </span>

    <span class="item">键 <b>{{ props.project.counts.value.all }}</b></span>
    <span class="item">待翻译 <b class="warn">{{ props.project.counts.value.todo }}</b></span>
    <span class="item">已翻译 <b class="ok">{{ props.project.counts.value.translated }}</b></span>
    <span class="item">当前筛选 <b>{{ shown }}</b></span>

    <!-- @ts-expect-error vscode-progress-bar 是自定义元素 -->
    <vscode-progress-bar class="progress" :value="percent" :max="100" />
    <span class="value">{{ percent.toFixed(1) }}%</span>

    <span v-if="props.project.dirtyCount.value" class="item warn">
      未保存 {{ props.project.dirtyCount.value }} 处
      <button class="mini" :disabled="!props.project.canUndo.value" title="撤销（Ctrl+Z）" @click="props.project.undo">撤销</button>
      <button class="mini" @click="props.project.revertEdits">全部放弃</button>
    </span>
    <span v-if="props.project.canUndo.value" class="item faint" title="可撤销的步数（Ctrl+Z 撤回，Ctrl+Y 重做）">
      记忆 {{ props.project.undoDepth.value }} 步
    </span>

    <span class="spacer"></span>

    <span v-if="s.busy.value" class="item busy">
      <Icon name="refresh" :size="12" :spin="true" />{{ s.busyText.value }}
    </span>
    <span v-else-if="s.error.value" class="item err" :title="s.error.value">
      <Icon name="error" :size="12" />{{ s.error.value }}
    </span>
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
  padding: 0 var(--s3);
  background: var(--bg-statusbar);
  border-top: 1px solid var(--border-subtle);
  font-size: var(--fs-sm);
  color: var(--vscode-statusBar-foreground, #cccccc);
  overflow: hidden;
}

.brand {
  display: inline-flex;
  align-items: center;
  gap: var(--s3);
  padding: 0 var(--s4);
  height: 100%;
  background: var(--vscode-statusBarItem-remoteBackground, #0078d4);
  color: #fff;
}

.item { display: inline-flex; align-items: center; gap: var(--s3); white-space: nowrap; }
.item b { color: #fff; font-weight: 600; font-variant-numeric: tabular-nums; }
.item b.warn { color: var(--warn-text); }
.item b.ok { color: var(--ok-text); }
.item.warn { color: var(--warn-text); }
.item.err { color: var(--err-text); overflow: hidden; text-overflow: ellipsis; max-width: 46ch; }
.item.ok { color: var(--ok-text); }
.item.faint { color: var(--vscode-statusBar-foreground, #cccccc); opacity: 0.75; overflow: hidden; text-overflow: ellipsis; max-width: 60ch; }
.item.busy { color: var(--accent-text); }

.progress { width: 120px; height: 3px; }
.value { font-variant-numeric: tabular-nums; color: #fff; }

.spacer { flex: 1; }

.mini {
  height: 16px;
  padding: 0 var(--s3);
  font-size: var(--fs-xs);
  background: rgba(255, 255, 255, 0.12);
  color: inherit;
}
.mini:hover:not(:disabled) { background: rgba(255, 255, 255, 0.22); }
.mini:disabled { opacity: 0.4; }
</style>
