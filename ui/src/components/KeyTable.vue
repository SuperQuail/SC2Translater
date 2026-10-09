<script setup lang="ts">
// 中栏：键表。这是工具的主工作面。
//
// 关键做法：
//  * **窗口化渲染** —— 大模组的键可以上万条，全量渲染会卡死；只渲染视口附近的行。
//  * **行内编辑** —— 译文直接改，改完立刻反映到状态与进度，不需要点保存。
//  * 表格用 grid 而不是 table，行高由密度决定（紧凑 30 / 标准 36 / 舒适 44）。

import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";

import { STATUS_META, statusLabel } from "../api";
import { entryId } from "../composables/useProject";
import type { Density, Project } from "../composables/useProject";
import type { Entry } from "../api/types";

const props = defineProps<{ project: Project }>();
const s = props.project.state;

const scroller = ref<HTMLElement | null>(null);
const scrollTop = ref(0);
const viewportH = ref(600);
const OVERSCAN = 10;

const rowHeight = computed(() => {
  const map: Record<Density, number> = { compact: 30, standard: 36, comfortable: 44 };
  return map[s.density.value];
});

const total = computed(() => props.project.filtered.value.length);
const start = computed(() => Math.max(0, Math.floor(scrollTop.value / rowHeight.value) - OVERSCAN));
const end = computed(() =>
  Math.min(total.value, Math.ceil((scrollTop.value + viewportH.value) / rowHeight.value) + OVERSCAN),
);
const visible = computed(() => props.project.filtered.value.slice(start.value, end.value));
const padTop = computed(() => start.value * rowHeight.value);
const padBottom = computed(() => Math.max(0, (total.value - end.value) * rowHeight.value));

function measure() {
  if (scroller.value) viewportH.value = scroller.value.clientHeight;
}

function onScroll() {
  scrollTop.value = scroller.value?.scrollTop ?? 0;
}

onMounted(() => {
  measure();
  window.addEventListener("resize", measure);
});
onBeforeUnmount(() => window.removeEventListener("resize", measure));

// 过滤条件或密度变化后，回到顶部，避免看到一个空白区域
watch([() => s.statusFilter.value, () => s.fileFilter.value, () => s.query.value, rowHeight], () => {
  if (scroller.value) scroller.value.scrollTop = 0;
  scrollTop.value = 0;
});

/** 选中行时把它滚进视野（键盘上下移动时用）。 */
function ensureVisible(id: string) {
  const index = props.project.filtered.value.findIndex((e) => entryId(e) === id);
  if (index < 0 || !scroller.value) return;
  const top = index * rowHeight.value;
  const bottom = top + rowHeight.value;
  const viewTop = scroller.value.scrollTop;
  const viewBottom = viewTop + viewportH.value;
  if (top < viewTop) scroller.value.scrollTop = top;
  else if (bottom > viewBottom) scroller.value.scrollTop = bottom - viewportH.value;
}

watch(
  () => s.selected.value,
  (id) => { if (id) ensureVisible(id); },
);

function toneOf(entry: Entry): string {
  return STATUS_META[entry.status]?.tone ?? "muted";
}

function select(entry: Entry) {
  s.selected.value = entryId(entry);
}

function onTargetInput(entry: Entry, event: Event) {
  const value = (event.target as HTMLInputElement).value;
  props.project.editTarget(entry, value);
}

const bulkScope = ref<"filtered" | "all">("filtered");

function fillFromSource() {
  props.project.fillFromSource(bulkScope.value);
}

function clearTargets() {
  props.project.clearTargets(bulkScope.value);
}

const searchInput = ref<HTMLInputElement | null>(null);
defineExpose({ focusSearch: () => searchInput.value?.focus() });
</script>

<template>
  <section class="table-wrap">
    <div class="toolbar">
      <div class="search">
        <input
          ref="searchInput"
          v-model="s.query.value"
          type="search"
          placeholder="搜索键、原文或译文…（Ctrl+F）"
          spellcheck="false"
        />
        <span v-if="s.query.value" class="clear" @click="s.query.value = ''">×</span>
      </div>

      <div class="density" role="group" aria-label="行高密度">
        <button
          v-for="d in (['compact', 'standard', 'comfortable'] as Density[])"
          :key="d"
          :class="{ active: s.density.value === d }"
          :title="d === 'compact' ? '紧凑' : d === 'standard' ? '标准' : '舒适'"
          @click="s.density.value = d"
        >
          <span class="lines" :class="d"></span>
        </button>
      </div>

      <span class="grow"></span>

      <span class="counts">
        <b>{{ total }}</b> / {{ props.project.counts.value.all }} 条
        <template v-if="s.checked.value.size">· 已选 {{ s.checked.value.size }}</template>
      </span>
    </div>

    <div class="bulk">
      <button class="ghost mini" @click="props.project.checkAllFiltered">全选当前结果</button>
      <button class="ghost mini" :disabled="!s.checked.value.size" @click="props.project.clearChecked">
        取消选择
      </button>
      <span class="sep"></span>
      <label class="scope">
        作用范围
        <select v-model="bulkScope">
          <option value="filtered">当前筛选结果</option>
          <option value="all">全部条目</option>
        </select>
      </label>
      <button class="mini" @click="fillFromSource">原文填入空译文</button>
      <button class="mini danger" @click="clearTargets">清空译文</button>
    </div>

    <div class="head grid" :style="{ height: rowHeight + 'px' }">
      <span class="cell check">
        <input
          type="checkbox"
          :checked="total > 0 && s.checked.value.size === total"
          :indeterminate.prop="s.checked.value.size > 0 && s.checked.value.size < total"
          title="全选 / 取消全选当前筛选结果"
          @change="s.checked.value.size === total ? props.project.clearChecked() : props.project.checkAllFiltered()"
        />
      </span>
      <span class="cell">状态</span>
      <span class="cell">键</span>
      <span class="cell">原文（{{ s.source.value }}）</span>
      <span class="cell">译文（{{ s.target.value }}）</span>
    </div>

    <div ref="scroller" class="scroller" @scroll="onScroll">
      <div v-if="!total" class="empty">
        <template v-if="!s.doc.value">先在左上角打开一张 .SC2Map / .SC2Mod。</template>
        <template v-else>当前筛选条件下没有条目。换一个状态或清空搜索再试。</template>
      </div>

      <div :style="{ height: padTop + 'px' }"></div>

      <div
        v-for="entry in visible"
        :key="entryId(entry)"
        class="row grid"
        :class="{ selected: s.selected.value === entryId(entry), checked: s.checked.value.has(entryId(entry)) }"
        :style="{ height: rowHeight + 'px' }"
        @click="select(entry)"
      >
        <span class="cell check" @click.stop>
          <input
            type="checkbox"
            :checked="s.checked.value.has(entryId(entry))"
            @change="props.project.toggleCheck(entryId(entry))"
          />
        </span>

        <span class="cell">
          <span class="status">
            <span class="tone" :class="toneOf(entry)"></span>
            <span class="status-text">{{ statusLabel(entry.status) }}</span>
          </span>
        </span>

        <span class="cell key mono" :title="entry.key">{{ entry.key }}</span>

        <span class="cell source" :title="entry.source ?? ''">{{ entry.source ?? "—" }}</span>

        <span class="cell target" @click.stop>
          <input
            class="target-input"
            :class="{ filled: (entry.target ?? '').trim().length > 0 }"
            :value="entry.target ?? ''"
            :placeholder="entry.source ? '待翻译' : '—'"
            spellcheck="false"
            @focus="select(entry)"
            @input="onTargetInput(entry, $event)"
          />
        </span>
      </div>

      <div :style="{ height: padBottom + 'px' }"></div>
    </div>
  </section>
</template>

<style scoped>
.table-wrap {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  background: var(--bg-app);
}

.toolbar {
  display: flex;
  align-items: center;
  gap: var(--s4);
  padding: var(--s4) var(--s5);
  border-bottom: 1px solid var(--border-subtle);
}

.search { position: relative; flex: 1; max-width: 460px; }
.search input { height: 30px; padding-right: 26px; }
.clear {
  position: absolute;
  right: 8px;
  top: 50%;
  transform: translateY(-50%);
  color: var(--text-faint);
  cursor: pointer;
  font-size: 16px;
  line-height: 1;
}
.clear:hover { color: var(--text); }

.density { display: flex; gap: 2px; border: 1px solid var(--border-subtle); border-radius: var(--r2); padding: 2px; }
.density button { height: 24px; width: 26px; padding: 0; background: transparent; border-color: transparent; justify-content: center; }
.density button.active { background: var(--bg-active); border-color: var(--border); }

.lines { display: block; width: 12px; height: 8px; position: relative; }
.lines::before, .lines::after {
  content: "";
  position: absolute;
  left: 0;
  right: 0;
  height: 1px;
  background: currentColor;
  opacity: 0.85;
}
.lines::before { top: 0; }
.lines::after { bottom: 0; }
.lines.compact { height: 6px; }
.lines.comfortable { height: 11px; }

.grow { flex: 1; }
.counts { font-size: var(--fs-sm); color: var(--text-muted); font-variant-numeric: tabular-nums; }
.counts b { color: var(--text); }

.bulk {
  display: flex;
  align-items: center;
  gap: var(--s3);
  padding: var(--s3) var(--s5);
  background: var(--bg-panel);
  border-bottom: 1px solid var(--border-subtle);
  font-size: var(--fs-sm);
  color: var(--text-muted);
}

.mini { height: 24px; padding: 0 var(--s4); font-size: var(--fs-sm); }
.sep { width: 1px; height: 14px; background: var(--border-subtle); }

.scope { display: flex; align-items: center; gap: var(--s3); }
.scope select { width: auto; height: 24px; font-size: var(--fs-sm); padding: 0 var(--s4); }

.grid {
  display: grid;
  grid-template-columns: 34px 84px minmax(180px, 1.1fr) minmax(200px, 1.4fr) minmax(200px, 1.4fr);
  align-items: center;
  gap: 0;
}

.head {
  padding: 0 var(--s5);
  background: var(--bg-panel);
  border-bottom: 1px solid var(--border-subtle);
  font-size: var(--fs-xs);
  color: var(--text-faint);
  text-transform: none;
  position: sticky;
  top: 0;
  z-index: 2;
}

.scroller { flex: 1; overflow: auto; padding: 0 var(--s5) var(--s6); }

.row {
  border-bottom: 1px solid var(--border-subtle);
  cursor: default;
}

.row:hover { background: var(--bg-hover); }
.row.selected { background: var(--blue-4); }
.row.selected:hover { background: var(--blue-5); }

.cell { padding: 0 var(--s3); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: var(--fs-sm); }
.cell.check { display: flex; align-items: center; justify-content: center; padding: 0; }
.cell.check input { width: 13px; height: 13px; accent-color: var(--accent); cursor: pointer; }
.head .cell.check { cursor: pointer; }

.status { display: inline-flex; align-items: center; gap: var(--s3); }
.status-text { font-size: var(--fs-xs); color: var(--text-muted); }

.tone { width: 6px; height: 6px; border-radius: 50%; background: var(--slate-9); flex: 0 0 auto; }
.tone.err { background: var(--err); }
.tone.warn { background: var(--warn); }
.tone.ok { background: var(--ok); }
.tone.info { background: var(--accent); }

.key { color: var(--text); font-size: var(--fs-xs); }
.source { color: var(--text-muted); }
.target { padding-right: var(--s2); }

.target-input {
  height: 24px;
  font-size: var(--fs-sm);
  background: transparent;
  border-color: transparent;
}
.target-input:hover { border-color: var(--border-subtle); }
.target-input.filled { color: var(--text); }
.target-input:not(.filled) { color: var(--text-faint); }

.empty { padding: var(--s9) var(--s5); color: var(--text-faint); text-align: center; }
</style>
