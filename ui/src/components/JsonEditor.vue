<script setup lang="ts">
// 内置 JSON 编辑器。
//
// 表格适合逐条过；但很多翻译者习惯直接改 JSON（或者用外部工具批量处理）。
// 这个视图把「当前工作表」原样呈现成 sc2trans.keys.v1 文档，改完点「应用到表格」即可，
// 不需要导出再导入一轮。解析失败会就地报错，不会污染表格数据。

import { ref, watch } from "vue";

import type { Entry } from "../api/types";
import type { Project } from "../composables/useProject";

const props = defineProps<{ project: Project }>();
const s = props.project.state;

const text = ref("");
const error = ref("");
const info = ref("");
const dirty = ref(false);

/** 把当前工作表渲染成 JSON 文本。 */
function render(): string {
  const doc = s.doc.value;
  if (!doc) return "{}";
  const payload = {
    schema: doc.schema,
    tool: doc.tool,
    source_locale: s.source.value,
    target_locale: s.target.value,
    archive: doc.archive,
    files: doc.files,
    stats: { entries: s.entries.value.length, translatable: props.project.counts.value.todo },
    entries: s.entries.value.map((e) => ({
      key: e.key,
      file: e.file,
      source: e.source ?? null,
      target: e.target ?? null,
      status: e.status,
    })),
  };
  return JSON.stringify(payload, null, 2);
}

function reload() {
  text.value = render();
  error.value = "";
  info.value = "";
  dirty.value = false;
}

watch(
  () => [s.doc.value, s.selected.value, s.entries.value.length, dirty.value],
  () => {
    // 表格里改过就别覆盖用户正在编辑的文本
    if (!dirty.value) text.value = render();
  },
  { immediate: true },
);

function onInput() {
  dirty.value = true;
  info.value = "";
  try {
    JSON.parse(text.value);
    error.value = "";
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  }
}

function format() {
  try {
    text.value = JSON.stringify(JSON.parse(text.value), null, 2);
    error.value = "";
    info.value = "已格式化";
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  }
}

function sortByKey() {
  try {
    const parsed = JSON.parse(text.value) as { entries?: Array<Record<string, unknown>> };
    if (Array.isArray(parsed.entries)) {
      parsed.entries.sort((a, b) => {
        const fa = String(a.file ?? "");
        const fb = String(b.file ?? "");
        if (fa !== fb) return fa < fb ? -1 : 1;
        const ka = String(a.key ?? "");
        const kb = String(b.key ?? "");
        return ka < kb ? -1 : ka > kb ? 1 : 0;
      });
      text.value = JSON.stringify(parsed, null, 2);
      info.value = "已按 (字符串表, 键) 排序";
      error.value = "";
    }
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  }
}

/** 把 JSON 里的 target 合并回表格。 */
function applyToTable() {
  let parsed: { entries?: Array<Record<string, unknown>> };
  try {
    parsed = JSON.parse(text.value);
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
    return;
  }
  if (!Array.isArray(parsed.entries)) {
    error.value = "缺少 entries 数组";
    return;
  }
  let applied = 0;
  let skipped = 0;
  for (const raw of parsed.entries) {
    const key = typeof raw.key === "string" ? raw.key : null;
    if (!key) continue;
    const file = typeof raw.file === "string" ? raw.file : null;
    const target = typeof raw.target === "string" ? raw.target : "";
    const hit: Entry | undefined = file
      ? s.entries.value.find((e) => e.file === file && e.key === key)
      : s.entries.value.find((e) => e.key === key);
    if (!hit) {
      skipped += 1;
      continue;
    }
    if ((hit.target ?? "") !== target) {
      props.project.editTarget(hit, target);
      applied += 1;
    }
  }
  dirty.value = false;
  info.value = "已应用到表格：" + applied + " 条" + (skipped ? "，未匹配 " + skipped + " 条" : "");
  props.project.log("JSON 编辑器：应用 " + applied + " 条到工作表");
}

async function copyAll() {
  try {
    await navigator.clipboard.writeText(text.value);
    info.value = "已复制到剪贴板";
  } catch {
    info.value = "复制失败（浏览器拒绝）";
  }
}
</script>

<template>
  <section class="json">
    <div class="bar">
      <!-- @ts-expect-error vscode-button 是自定义元素 -->
      <vscode-button secondary icon="wand" @click="format">格式化</vscode-button>
      <!-- @ts-expect-error 自定义元素 -->
      <vscode-button secondary icon="list-ordered" @click="sortByKey">按 key 排序</vscode-button>
      <!-- @ts-expect-error 自定义元素 -->
      <vscode-button secondary icon="refresh" @click="reload">重新载入</vscode-button>
      <!-- @ts-expect-error 自定义元素 -->
      <vscode-button secondary icon="copy" @click="copyAll">复制</vscode-button>
      <span class="grow"></span>
      <span class="hint">sc2trans.keys.v1</span>
      <!-- @ts-expect-error 自定义元素 -->
      <vscode-button icon="check" :disabled="Boolean(error)" @click="applyToTable">应用到表格</vscode-button>
    </div>

    <div v-if="error" class="alert err">JSON 解析失败：{{ error }}</div>
    <div v-else-if="info" class="alert ok">{{ info }}</div>
    <div v-else-if="dirty" class="alert warn">有未应用的改动，点右侧「应用到表格」生效</div>

    <textarea
      class="editor"
      :value="text"
      spellcheck="false"
      wrap="off"
      @input="onInput"
    ></textarea>
  </section>
</template>

<style scoped>
.json { flex: 1; min-width: 0; display: flex; flex-direction: column; background: var(--bg-editor); }

.bar {
  display: flex;
  align-items: center;
  gap: var(--s3);
  padding: var(--s4) var(--s5);
  border-bottom: 1px solid var(--border-subtle);
}

.mini { height: 26px; padding: 0 var(--s4); font-size: var(--fs-sm); }
.grow { flex: 1; }
.hint { font-size: var(--fs-xs); color: var(--text-faint); }

.alert { padding: var(--s3) var(--s5); font-size: var(--fs-sm); border-bottom: 1px solid var(--border-subtle); }
.alert.err { color: var(--err-text); background: rgba(229, 72, 77, 0.08); }
.alert.ok { color: var(--ok-text); background: rgba(48, 164, 108, 0.08); }
.alert.warn { color: var(--warn-text); background: rgba(255, 197, 61, 0.08); }

.editor {
  flex: 1;
  min-height: 0;
  border: none;
  border-radius: 0;
  background: var(--bg-inset);
  color: var(--text);
  font-family: var(--font-mono);
  font-size: var(--fs-sm);
  line-height: 1.6;
  padding: var(--s5);
  resize: none;
  white-space: pre;
  tab-size: 2;
}

.editor:focus { box-shadow: none; border: none; }
</style>
