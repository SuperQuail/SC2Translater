<script setup lang="ts">
// 键值文本视图：所见即所得地编辑「键=译文」。
//
// 为什么要有这一屏：表格适合逐条过，JSON 适合程序处理，但很多翻译者最顺手的是
// 直接看一坨文本 —— 这也正是 SC2 本地化文件本身的格式（key=value）。
// 格式与 GameStrings.txt 一致，按字符串表分段（# 开头的行为分段注释，解析时忽略）。

import { computed, ref, watch } from "vue";

import type { Project } from "../composables/useProject";

const props = defineProps<{ project: Project }>();
const s = props.project.state;

const text = ref("");
const info = ref("");
const dirty = ref(false);
const onlyTodo = ref(true);

/** 这一屏处理哪些条目：待翻译，还是全部（受当前筛选影响）。 */
const scope = computed(() =>
  onlyTodo.value
    ? props.project.filtered.value.filter((e) => e.status === "missing" || e.status === "identical")
    : props.project.filtered.value,
);

function render(): string {
  const lines: string[] = [];
  lines.push("# SC2Translater 键值编辑 —— 每行格式为 键=译文");
  lines.push("# 以 # 开头的行会被忽略；留空表示未翻译");
  lines.push("");
  let lastFile = "";
  for (const e of scope.value) {
    if (e.file !== lastFile) {
      if (lastFile) lines.push("");
      lines.push("# ── " + e.file + " ──");
      lastFile = e.file;
    }
    lines.push(e.key + "=" + (e.target ?? ""));
  }
  return lines.join("\n") + "\n";
}

function reload() {
  text.value = render();
  info.value = "";
  dirty.value = false;
}

watch(
  () => [s.doc.value, s.statusFilter.value, s.fileFilter.value, s.query.value, onlyTodo.value, s.entries.value.length],
  () => {
    if (!dirty.value) reload();
  },
  { immediate: true },
);

function onInput() {
  dirty.value = true;
  info.value = "";
}

/** 解析文本 -> (file, key, value)。 */
function parse(): Array<{ file: string; key: string; value: string }> {
  const out: Array<{ file: string; key: string; value: string }> = [];
  let file = scope.value[0]?.file ?? "GameStrings.txt";
  for (const raw of text.value.split(/\r?\n/)) {
    const line = raw.trim();
    if (!line) continue;
    if (line.startsWith("#")) {
      const m = line.match(/^#\s*──\s*(.+?)\s*──$/);
      if (m) file = m[1];
      continue;
    }
    const eq = raw.indexOf("=");
    if (eq <= 0) continue;
    out.push({ file, key: raw.slice(0, eq).trim(), value: raw.slice(eq + 1) });
  }
  return out;
}

const pendingChanges = computed(() => {
  if (!dirty.value) return 0;
  let n = 0;
  for (const item of parse()) {
    const hit = s.entries.value.find((e) => e.file === item.file && e.key === item.key);
    if (hit && (hit.target ?? "") !== item.value) n += 1;
  }
  return n;
});

function applyToTable() {
  let applied = 0;
  let skipped = 0;
  for (const item of parse()) {
    const hit = s.entries.value.find((e) => e.file === item.file && e.key === item.key);
    if (!hit) {
      skipped += 1;
      continue;
    }
    if ((hit.target ?? "") !== item.value) {
      props.project.editTarget(hit, item.value);
      applied += 1;
    }
  }
  dirty.value = false;
  info.value = "已应用到键表：" + applied + " 条" + (skipped ? "，未匹配 " + skipped + " 条" : "");
  props.project.log("键值编辑：应用 " + applied + " 条");
}

/** 把这一屏里空着的译文用原文补上（只动当前范围）。 */
function fillEmptyHere() {
  let n = 0;
  for (const item of parse()) {
    const hit = s.entries.value.find((e) => e.file === item.file && e.key === item.key);
    if (hit && !(hit.target ?? "").trim() && hit.source) {
      props.project.editTarget(hit, hit.source);
      n += 1;
    }
  }
  dirty.value = false;
  reload();
  info.value = "用原文补了 " + n + " 条空译文";
}

async function copyAll() {
  try {
    await navigator.clipboard.writeText(text.value);
    info.value = "已复制到剪贴板";
  } catch {
    info.value = "复制失败（浏览器拒绝）";
  }
}

const statusLine = computed(() => {
  const total = parse().length;
  const todo = scope.value.filter((e) => e.status === "missing" || e.status === "identical").length;
  return total + " 行 · 其中待翻译 " + todo;
});
</script>

<template>
  <section class="kv">
    <div class="bar">
      <!-- @ts-expect-error vscode-checkbox 是自定义元素 -->
      <vscode-checkbox :checked="onlyTodo" label="只看待翻译" @change="onlyTodo = ($event.target as unknown as { checked: boolean }).checked"></vscode-checkbox>
      <span class="divider"></span>
      <!-- @ts-expect-error 自定义元素 -->
      <vscode-button secondary icon="refresh" @click="reload">重新载入</vscode-button>
      <!-- @ts-expect-error 自定义元素 -->
      <vscode-button secondary icon="wand" @click="fillEmptyHere">原文填空译文</vscode-button>
      <!-- @ts-expect-error 自定义元素 -->
      <vscode-button secondary icon="copy" @click="copyAll">复制</vscode-button>
      <span class="grow"></span>
      <span class="stat">{{ statusLine }}</span>
      <span v-if="dirty" class="pending">{{ pendingChanges }} 处未应用</span>
      <!-- @ts-expect-error 自定义元素 -->
      <vscode-button icon="check" :disabled="!dirty" @click="applyToTable">应用到键表</vscode-button>
    </div>

    <div v-if="info" class="alert ok">{{ info }}</div>
    <div v-else-if="dirty" class="alert warn">文本有改动，点右侧「应用到键表」才会生效</div>
    <div v-else class="alert hint">
      格式与 SC2 的 GameStrings.txt 一致：<code>键=译文</code>。<code>#</code> 开头的行是注释。
    </div>

    <textarea class="editor" :value="text" spellcheck="false" wrap="off" @input="onInput"></textarea>
  </section>
</template>

<style scoped>
.kv { flex: 1; min-width: 0; display: flex; flex-direction: column; background: var(--bg-editor); }

.bar {
  display: flex;
  align-items: center;
  gap: var(--s4);
  padding: var(--s3) var(--s5);
  border-bottom: 1px solid var(--border-subtle);
}

.check { display: flex; align-items: center; gap: var(--s3); font-size: var(--fs-sm); color: var(--text-muted); cursor: pointer; }
.check input { accent-color: var(--accent); cursor: pointer; }
.divider { width: 1px; height: 14px; background: var(--border-subtle); }

.mini { height: 26px; padding: 0 var(--s4); font-size: var(--fs-sm); gap: var(--s2); }
.grow { flex: 1; }
.stat { font-size: var(--fs-xs); color: var(--text-faint); font-variant-numeric: tabular-nums; }
.pending { font-size: var(--fs-xs); color: var(--warn-text); }

.alert { padding: var(--s3) var(--s5); font-size: var(--fs-sm); border-bottom: 1px solid var(--border-subtle); }
.alert.ok { color: var(--ok-text); background: rgba(48, 164, 108, 0.08); }
.alert.warn { color: var(--warn-text); background: rgba(255, 197, 61, 0.08); }
.alert.hint { color: var(--text-faint); }
.alert code { background: var(--bg-raised); border: 1px solid var(--border-subtle); border-radius: var(--r1); padding: 0 4px; color: var(--text); }

.editor {
  flex: 1;
  min-height: 0;
  border: none;
  border-radius: 0;
  background: var(--bg-inset);
  color: var(--text);
  font-family: var(--font-mono);
  font-size: var(--fs-sm);
  line-height: 1.7;
  padding: var(--s5);
  resize: none;
  white-space: pre;
  tab-size: 2;
}
.editor:focus { box-shadow: none; border: none; }
</style>
