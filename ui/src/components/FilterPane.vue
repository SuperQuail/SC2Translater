<script setup lang="ts">
// 过滤器面板：状态 + 字符串表。每个条目都带实时计数。

import type { ContextMenuApi } from "../composables/useContextMenu";
import type { Project, StatusFilter } from "../composables/useProject";
import Icon from "./Icon.vue";

const props = defineProps<{ project: Project; menu: ContextMenuApi }>();
const s = props.project.state;

const STATUS_ROWS: Array<{ key: StatusFilter; label: string; short: string; tone: string }> = [
  { key: "all", label: "全部条目", short: "全部", tone: "muted" },
  { key: "todo", label: "待翻译（缺失 + 未翻译）", short: "待翻译", tone: "warn" },
  { key: "missing", label: "缺失（会显示键路径）", short: "缺失", tone: "err" },
  { key: "identical", label: "与原文相同", short: "未翻译", tone: "warn" },
  { key: "translated", label: "已翻译", short: "已翻译", tone: "ok" },
  { key: "extra", label: "仅目标语言有", short: "多余", tone: "info" },
];

function countOf(key: StatusFilter): number {
  const c = props.project.counts.value;
  if (key === "all") return c.all;
  if (key === "todo") return c.todo;
  return c[key] ?? 0;
}

function resetAll() {
  s.statusFilter.value = "all";
  s.fileFilter.value = "all";
  s.query.value = "";
}

function statusMenu(event: MouseEvent, key: StatusFilter, label: string) {
  props.menu.openMenu(
    event,
    [
      { id: "only", label: "只显示：" + label },
      { id: "reset", label: "重置所有过滤", separator: true },
    ],
    (id) => (id === "only" ? (s.statusFilter.value = key) : resetAll()),
  );
}

function tableMenu(event: MouseEvent, file: string | null) {
  props.menu.openMenu(
    event,
    [
      { id: "only", label: file ? "只显示 " + file : "显示全部字符串表", disabled: !file },
      { id: "reset", label: "重置所有过滤", separator: true },
    ],
    (id) => (id === "only" && file ? (s.fileFilter.value = file) : resetAll()),
  );
}
</script>

<template>
  <div class="filters">
    <header class="section">
      <span class="title">过滤器</span>
      <button class="icon" title="重置所有过滤" @click="resetAll"><Icon name="refresh" :size="14" /></button>
    </header>

    <section class="block">
      <h3>状态</h3>
      <button
        v-for="row in STATUS_ROWS"
        :key="row.key"
        class="row"
        :class="{ active: s.statusFilter.value === row.key }"
        @click="s.statusFilter.value = row.key"
        @contextmenu="statusMenu($event, row.key, row.label)"
      >
        <span class="tone" :class="row.tone"></span>
        <span class="label">{{ row.short }}</span>
        <span class="count">{{ countOf(row.key) }}</span>
      </button>
    </section>

    <section class="block">
      <h3>字符串表</h3>
      <button
        class="row"
        :class="{ active: s.fileFilter.value === 'all' }"
        @click="s.fileFilter.value = 'all'"
        @contextmenu="tableMenu($event, null)"
      >
        <Icon name="layers" :size="13" />
        <span class="label">全部</span>
        <span class="count">{{ props.project.counts.value.all }}</span>
      </button>
      <button
        v-for="f in props.project.files.value"
        :key="f"
        class="row"
        :class="{ active: s.fileFilter.value === f }"
        @click="s.fileFilter.value = f"
        @contextmenu="tableMenu($event, f)"
      >
        <Icon name="text" :size="13" />
        <span class="label mono">{{ f.replace('.txt', '') }}</span>
        <span class="tag" :class="{ editor: !(s.audit.value?.by_file?.[f]?.player_facing ?? true) }">
          {{ s.audit.value?.by_file?.[f]?.player_facing === false ? "编辑器" : "玩家" }}
        </span>
        <span class="count">{{ s.doc.value?.stats.by_file?.[f] ?? 0 }}</span>
      </button>
    </section>
  </div>
</template>

<style scoped>
.filters { display: flex; flex-direction: column; height: 100%; overflow-y: auto; background: var(--bg-sidebar); }

.section {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--s3) var(--s3) var(--s3) var(--s5);
  position: sticky;
  top: 0;
  background: var(--bg-sidebar);
  z-index: 2;
}
.title { font-size: var(--fs-xs); font-weight: 600; letter-spacing: 0.06em; color: var(--text-muted); text-transform: uppercase; }
.icon { width: 22px; height: 22px; padding: 0; justify-content: center; background: transparent; border-color: transparent; color: var(--text-faint); }
.icon:hover { background: var(--bg-hover); border-color: transparent; color: var(--text); }

.block { padding: var(--s2) var(--s3) var(--s5); display: flex; flex-direction: column; gap: 1px; }
.block h3 { padding: var(--s3) var(--s3) var(--s2); }

.row {
  display: flex;
  align-items: center;
  gap: var(--s3);
  width: 100%;
  height: 26px;
  padding: 0 var(--s3);
  background: transparent;
  border-color: transparent;
  color: var(--text-muted);
  font-size: var(--fs-sm);
  text-align: left;
}
.row:hover { background: var(--list-hover); border-color: transparent; color: var(--text); }
.row.active { background: var(--list-active-bg); color: var(--list-active-fg); }

.label { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.count { font-size: var(--fs-xs); color: var(--text-faint); font-variant-numeric: tabular-nums; }
.row.active .count { color: var(--accent-text); }

.tone { width: 7px; height: 7px; border-radius: 50%; flex: 0 0 auto; background: var(--slate-9); }
.tone.err { background: var(--err); }
.tone.warn { background: var(--warn); }
.tone.ok { background: var(--ok); }
.tone.info { background: var(--accent); }

.tag { font-size: 10px; padding: 0 5px; border-radius: var(--r1); background: var(--green-9); color: #04120b; flex: 0 0 auto; }
.tag.editor { background: var(--slate-7); color: var(--text); }
</style>
