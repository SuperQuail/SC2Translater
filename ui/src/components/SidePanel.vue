<script setup lang="ts">
// 左栏：概览 + 两个过滤器 + 语言覆盖。
// 过滤器的每一个条目都带实时计数，用户一眼就知道「还有多少要翻」。

import { computed } from "vue";

import { formatBytes, formatCoverage } from "../api";
import type { Project, StatusFilter } from "../composables/useProject";

const props = defineProps<{ project: Project }>();
const s = props.project.state;

const STATUS_ROWS: Array<{ key: StatusFilter; label: string; tone: string }> = [
  { key: "all", label: "全部", tone: "muted" },
  { key: "todo", label: "待翻译（缺失 + 未翻译）", tone: "warn" },
  { key: "missing", label: "缺失（会显示键路径）", tone: "err" },
  { key: "identical", label: "与原文相同", tone: "warn" },
  { key: "translated", label: "已翻译", tone: "ok" },
  { key: "extra", label: "仅目标语言有", tone: "info" },
];

const overview = computed(() => {
  const i = s.info.value;
  if (!i) return null;
  return {
    size: formatBytes(i.size),
    components: i.container.components,
    blocks: i.container.blocks,
    locales: i.locales.map((l) => l.locale),
    rawChunk: i.container.raw_chunk_size,
  };
});

const auditLocales = computed(() => s.audit.value?.locales ?? []);

function countOf(key: StatusFilter): number {
  const c = props.project.counts.value;
  if (key === "all") return c.all;
  if (key === "todo") return c.todo;
  return c[key] ?? 0;
}
</script>

<template>
  <aside class="side">
    <section v-if="overview" class="block">
      <h3>概览</h3>
      <div class="kv"><span>大小</span><b>{{ overview.size }}</b></div>
      <div class="kv"><span>组件 / 块</span><b>{{ overview.components }} / {{ overview.blocks }}</b></div>
      <div class="kv"><span>语言目录</span><b>{{ overview.locales.length }}</b></div>
      <div class="locales mono">{{ overview.locales.join(" ") || "—" }}</div>
      <div class="divider"></div>
      <div class="kv"><span>已完成</span><b>{{ props.project.progress.value.done }} / {{ props.project.progress.value.total }}</b></div>
      <div class="bar"><div class="bar-fill" :style="{ width: props.project.progress.value.percent + '%' }"></div></div>
    </section>

    <section class="block">
      <h3>状态</h3>
      <button
        v-for="row in STATUS_ROWS"
        :key="row.key"
        class="filter"
        :class="{ active: s.statusFilter.value === row.key }"
        @click="s.statusFilter.value = row.key"
      >
        <span class="tone" :class="row.tone"></span>
        <span class="label">{{ row.label }}</span>
        <span class="count">{{ countOf(row.key) }}</span>
      </button>
    </section>

    <section class="block">
      <h3>字符串表</h3>
      <button class="filter" :class="{ active: s.fileFilter.value === 'all' }" @click="s.fileFilter.value = 'all'">
        <span class="label">全部</span>
        <span class="count">{{ props.project.counts.value.all }}</span>
      </button>
      <button
        v-for="f in props.project.files.value"
        :key="f"
        class="filter"
        :class="{ active: s.fileFilter.value === f }"
        @click="s.fileFilter.value = f"
      >
        <span class="label mono">{{ f.replace('.txt', '') }}</span>
        <span class="tag" :class="{ editor: !(s.audit.value?.by_file?.[f]?.player_facing ?? true) }">
          {{ s.audit.value?.by_file?.[f]?.player_facing === false ? "编辑器" : "玩家" }}
        </span>
        <span class="count">{{ s.doc.value?.stats.by_file?.[f] ?? 0 }}</span>
      </button>
    </section>

    <section v-if="auditLocales.length" class="block">
      <h3>语言覆盖</h3>
      <div v-for="row in auditLocales" :key="row.locale" class="locale-row">
        <div class="locale-head">
          <span class="mono">{{ row.locale }}</span>
          <span class="pct" :class="{ bad: row.coverage < 1 }">{{ formatCoverage(row.coverage) }}</span>
        </div>
        <div class="bar small"><div class="bar-fill" :class="{ bad: row.coverage < 1 }" :style="{ width: row.coverage * 100 + '%' }"></div></div>
      </div>
    </section>
  </aside>
</template>

<style scoped>
.side {
  width: var(--w-side);
  flex: 0 0 auto;
  overflow-y: auto;
  background: var(--bg-panel);
  border-right: 1px solid var(--border-subtle);
  padding: var(--s5) var(--s4) var(--s8);
  display: flex;
  flex-direction: column;
  gap: var(--s6);
}

.block { display: flex; flex-direction: column; gap: var(--s2); }

.block h3 { padding: 0 var(--s3) var(--s2); }

.kv { display: flex; justify-content: space-between; padding: var(--s1) var(--s3); font-size: var(--fs-sm); color: var(--text-muted); }
.kv b { color: var(--text); font-weight: 600; font-variant-numeric: tabular-nums; }

.locales { padding: var(--s2) var(--s3) 0; font-size: var(--fs-xs); color: var(--text-faint); word-break: break-all; }

.bar { height: 4px; border-radius: 999px; background: var(--slate-5); overflow: hidden; margin: var(--s2) var(--s3) 0; }
.bar.small { margin: 3px 0 0; }
.bar-fill { height: 100%; background: var(--accent); transition: width var(--t-base); }
.bar-fill.bad { background: var(--warn); }

.filter {
  display: flex;
  align-items: center;
  gap: var(--s3);
  width: 100%;
  height: 28px;
  padding: 0 var(--s3);
  background: transparent;
  border-color: transparent;
  color: var(--text-muted);
  font-size: var(--fs-sm);
  text-align: left;
}

.filter:hover { background: var(--bg-hover); border-color: transparent; }
.filter.active { background: var(--blue-4); color: var(--text); border-color: transparent; }

.filter .label { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.filter .count { font-size: var(--fs-xs); color: var(--text-faint); font-variant-numeric: tabular-nums; }
.filter.active .count { color: var(--accent-text); }

.tone { width: 6px; height: 6px; border-radius: 50%; flex: 0 0 auto; background: var(--slate-9); }
.tone.err { background: var(--err); }
.tone.warn { background: var(--warn); }
.tone.ok { background: var(--ok); }
.tone.info { background: var(--accent); }

.tag {
  font-size: 10px;
  padding: 0 5px;
  border-radius: var(--r1);
  background: var(--green-9);
  color: #04120b;
  flex: 0 0 auto;
}
.tag.editor { background: var(--slate-7); color: var(--text); }

.locale-row { padding: var(--s2) var(--s3); }
.locale-head { display: flex; justify-content: space-between; font-size: var(--fs-xs); color: var(--text-muted); }
.pct { font-variant-numeric: tabular-nums; }
.pct.bad { color: var(--warn-text); }
</style>
