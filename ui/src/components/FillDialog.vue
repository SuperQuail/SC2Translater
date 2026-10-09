<script setup lang="ts">
// 「补全默认语言」对话框。
//
// 这是本工具最常用的一招：一张只带 enUS 的地图，在其余 11 种语言下全都会露出键路径。
// 这个操作一次把它们全部补上（默认不覆盖已有译文，只补缺的键）。

import { computed, ref } from "vue";

import { api } from "../api";
import { COMMON_LOCALES } from "../composables/useProject";
import type { Project } from "../composables/useProject";

const props = defineProps<{ project: Project }>();
const emit = defineEmits<{ (e: "close"): void }>();

const s = props.project.state;
const scope = ref<"all" | "explicit">("all");
const onlyMissing = ref(true);
const picked = ref<Set<string>>(new Set());

/** 归档里已经存在的语言（用来给用户标注「已有 / 缺失」）。 */
const present = computed(() => new Set((s.audit.value?.locales ?? []).map((l) => l.locale)));

const candidates = computed(() => COMMON_LOCALES.filter((l) => l.code !== s.source.value));

const selectedCount = computed(() => (scope.value === "all" ? candidates.value.length : picked.value.size));

function readValue(event: Event): string {
  return String((event.target as unknown as { value: string }).value ?? "");
}

function readChecked(event: Event): boolean {
  return Boolean((event.target as unknown as { checked: boolean }).checked);
}

function toggle(code: string) {
  const next = new Set(picked.value);
  if (next.has(code)) next.delete(code);
  else next.add(code);
  picked.value = next;
}

function selectAll() {
  picked.value = new Set(candidates.value.map((l) => l.code));
}

function selectMissing() {
  picked.value = new Set(candidates.value.filter((l) => !present.value.has(l.code)).map((l) => l.code));
}

async function run() {
  if (!s.archive.value) return;
  if (selectedCount.value === 0) return;
  const spec = scope.value === "all" ? "all" : Array.from(picked.value).join(",");
  const name = await api.defaultOutputName(s.archive.value, scope.value === "all" ? "all" : Array.from(picked.value)[0]);
  const out = await api.pickArchiveSave(name);
  if (!out) return;
  emit("close");
  await props.project.fillDefaultLanguages(spec, onlyMissing.value, out);
}
</script>

<template>
  <div class="overlay" @click.self="emit('close')">
    <div class="dialog">
      <header>
        <h2>补全默认语言</h2>
        <button class="ghost close" @click="emit('close')">×</button>
      </header>

      <p class="lead">
        用 <b>{{ s.source.value }}</b> 的文本，去填充那些<strong>没有语言文件</strong>的语言。
        游戏在这些语言下就不会再显示原始键路径了。
      </p>

      <section>
        <h3>范围</h3>
        <!-- @ts-expect-error vscode-radio-group 是自定义元素 -->
        <vscode-radio-group :value="scope" @change="scope = readValue($event) as 'all' | 'explicit'">
          <!-- @ts-expect-error 自定义元素 -->
          <vscode-radio value="all">全部语言（{{ candidates.length }} 种，含归档里出现过的非官方代码）</vscode-radio>
          <!-- @ts-expect-error 自定义元素 -->
          <vscode-radio value="explicit">指定语言</vscode-radio>
        </vscode-radio-group>

        <div v-if="scope === 'explicit'" class="picks">
          <div class="pick-actions">
            <button class="mini" @click="selectAll">全选</button>
            <button class="mini" @click="selectMissing">只选缺失的</button>
            <button class="mini ghost" @click="picked = new Set()">清空</button>
          </div>
          <div class="grid">
            <!-- @ts-expect-error vscode-checkbox 是自定义元素 -->
            <vscode-checkbox
              v-for="l in candidates"
              :key="l.code"
              :checked="picked.has(l.code)"
              @change="toggle(l.code)"
            >
              {{ l.code }} · {{ l.label }} · {{ present.has(l.code) ? "已有" : "缺失" }}
            </vscode-checkbox>
          </div>
        </div>
      </section>

      <section>
        <!-- @ts-expect-error 自定义元素 -->
        <vscode-checkbox
          :checked="onlyMissing"
          label="不覆盖已有译文（推荐：只补缺的键）"
          @change="onlyMissing = readChecked($event)"
        ></vscode-checkbox>
      </section>

      <footer>
        <span class="count">将处理 {{ selectedCount }} 种语言</span>
        <!-- @ts-expect-error 自定义元素 -->
        <vscode-button secondary @click="emit('close')">取消</vscode-button>
        <!-- @ts-expect-error 自定义元素 -->
        <vscode-button :disabled="selectedCount === 0 || s.busy.value" @click="run">选择输出并执行</vscode-button>
      </footer>
    </div>
  </div>
</template>

<style scoped>
.overlay {
  position: fixed;
  inset: 0;
  background: rgba(6, 8, 12, 0.62);
  display: grid;
  place-items: center;
  z-index: 60;
}

.dialog {
  width: 620px;
  max-height: 84vh;
  overflow-y: auto;
  background: var(--bg-panel);
  border: 1px solid var(--border);
  border-radius: var(--r4);
  box-shadow: var(--shadow-pop);
  padding: var(--s6);
  display: flex;
  flex-direction: column;
  gap: var(--s5);
}

header { display: flex; align-items: center; justify-content: space-between; }
header h2 { font-size: var(--fs-lg); }
.close { height: 26px; width: 26px; justify-content: center; padding: 0; font-size: 16px; }

.lead { color: var(--text-muted); font-size: var(--fs-sm); line-height: 1.7; margin: 0; }
.lead b { color: var(--text); }

section { display: flex; flex-direction: column; gap: var(--s3); }
section h3 { font-size: var(--fs-xs); }

.radio, .check { display: flex; align-items: center; gap: var(--s4); font-size: var(--fs-sm); cursor: pointer; }
.radio input, .check input { accent-color: var(--accent); cursor: pointer; }
.dim { color: var(--text-faint); font-size: var(--fs-xs); margin-left: var(--s3); }

.picks { display: flex; flex-direction: column; gap: var(--s4); margin-top: var(--s2); }
.pick-actions { display: flex; gap: var(--s3); }
.mini { height: 24px; padding: 0 var(--s4); font-size: var(--fs-xs); }

.grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: var(--s2);
  max-height: 240px;
  overflow-y: auto;
  padding: var(--s3);
  background: var(--bg-inset);
  border: 1px solid var(--border-subtle);
  border-radius: var(--r2);
}

.pick {
  display: flex;
  align-items: center;
  gap: var(--s3);
  padding: var(--s2) var(--s3);
  border-radius: var(--r2);
  cursor: pointer;
  font-size: var(--fs-sm);
}
.pick:hover { background: var(--bg-hover); }
.pick.on { background: var(--blue-4); }
.pick input { accent-color: var(--accent); cursor: pointer; }
.code { color: var(--text); }
.name { flex: 1; color: var(--text-muted); font-size: var(--fs-xs); }
.state { font-size: 10px; color: var(--warn-text); }
.state.has { color: var(--ok-text); }

footer { display: flex; align-items: center; gap: var(--s4); justify-content: flex-end; }
.count { margin-right: auto; font-size: var(--fs-sm); color: var(--text-faint); }
</style>
