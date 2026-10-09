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
import type { ContextMenuApi } from "../composables/useContextMenu";
import type { Density, Project } from "../composables/useProject";
import type { Entry } from "../api/types";

const props = defineProps<{ project: Project; menu: ContextMenuApi }>();
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

function readSelect(event: Event): string {
  return String((event.target as unknown as { value: string }).value ?? "");
}

function onTargetInput(entry: Entry, event: Event) {
  props.project.editTarget(entry, readSelect(event));
}

const bulkScope = ref<"filtered" | "all">("filtered");

function fillFromSource() {
  props.project.fillFromSource(bulkScope.value);
}

function clearTargets() {
  props.project.clearTargets(bulkScope.value);
}

const searchInput = ref<HTMLElement | null>(null);
defineExpose({
  focusSearch: () => {
    const el = searchInput.value as unknown as { focus?: () => void } | null;
    el?.focus?.();
  },
  /** 面板刚显示时量一次视口高度（窗口化渲染需要）。 */
  remeasure: () => measure(),
});

/** 行右键菜单：把这一条上最常做的动作都放进去。 */
function rowMenu(event: MouseEvent, entry: Entry) {
  select(entry);
  const id = entryId(entry);
  const checkedCount = s.checked.value.size;
  props.menu.openMenu(
    event,
    [
      { id: "fill", label: "用原文填入译文", hint: "Ctrl+D", disabled: !entry.source },
      { id: "clear", label: "清空这条译文", disabled: !(entry.target ?? "").trim() },
      { id: "fill-sel", label: "用原文填入选中的 " + checkedCount + " 条", separator: true, disabled: checkedCount === 0 },
      { id: "clear-sel", label: "清空选中的 " + checkedCount + " 条译文", disabled: checkedCount === 0 },
      { id: "copy-key", label: "复制键", separator: true },
      { id: "copy-source", label: "复制原文", disabled: !entry.source },
      { id: "copy-target", label: "复制译文", disabled: !(entry.target ?? "").trim() },
      { id: "only-file", label: "只显示 " + entry.file, separator: true },
      { id: "only-prefix", label: "只显示 " + prefixOf(entry.key) + " 开头的键" },
      { id: "only-status", label: "只显示「" + statusLabel(entry.status) + "」" },
      { id: "clear-filters", label: "重置所有过滤", separator: true },
    ],
    (picked) => {
      if (picked === "fill") props.project.editTarget(entry, entry.source ?? "");
      else if (picked === "clear") props.project.editTarget(entry, "");
      else if (picked === "fill-sel") fillChecked(true);
      else if (picked === "clear-sel") fillChecked(false);
      else if (picked === "copy-key") void copyText(entry.key, "键");
      else if (picked === "copy-source") void copyText(entry.source ?? "", "原文");
      else if (picked === "copy-target") void copyText(entry.target ?? "", "译文");
      else if (picked === "only-file") s.fileFilter.value = entry.file;
      else if (picked === "only-prefix") s.query.value = prefixOf(entry.key);
      else if (picked === "only-status") s.statusFilter.value = entry.status as typeof s.statusFilter.value;
      else if (picked === "clear-filters") {
        s.statusFilter.value = "all";
        s.fileFilter.value = "all";
        s.query.value = "";
      }
      void id;
    },
  );
}

/** 键的第一段（Button/Tooltip/X → Button）作为「前缀过滤」。 */
function prefixOf(key: string): string {
  const i = key.indexOf("/");
  return i > 0 ? key.slice(0, i + 1) : key;
}

function fillChecked(fill: boolean) {
  let n = 0;
  for (const e of props.project.filtered.value) {
    if (!s.checked.value.has(entryId(e))) continue;
    if (fill) {
      if (!(e.target ?? "").trim() && e.source) {
        props.project.editTarget(e, e.source);
        n += 1;
      }
    } else if ((e.target ?? "").trim()) {
      props.project.editTarget(e, "");
      n += 1;
    }
  }
  props.project.log((fill ? "用原文填入" : "清空") + "选中的 " + n + " 条");
}

async function copyText(text: string, what: string) {
  try {
    await navigator.clipboard.writeText(text);
    props.project.log("已复制" + what);
  } catch {
    props.project.log("复制失败（浏览器拒绝）");
  }
}
</script>

<template>
  <section class="table-wrap">
    <div class="toolbar">
      <div class="search">
        <!-- @ts-expect-error vscode-textfield 是自定义元素 -->
        <vscode-textfield
          ref="searchInput"
          :value="s.query.value"
          placeholder="搜索键、原文或译文…（Ctrl+F）"
          @input="s.query.value = ($event.target as unknown as { value: string }).value"
        >
          <!-- @ts-expect-error 自定义元素 -->
          <vscode-icon slot="content-before" name="search" :size="14" />
        </vscode-textfield>
        <span v-if="s.query.value" class="clear" @click="s.query.value = ''">×</span>
      </div>

      <!-- @ts-expect-error vscode-button-group 是自定义元素 -->
      <vscode-button-group class="density" role="group" aria-label="行高密度">
        <!-- @ts-expect-error 自定义元素 -->
        <vscode-button
          v-for="d in (['compact', 'standard', 'comfortable'] as Density[])"
          :key="d"
          secondary
          icon-only
          :icon="d === 'compact' ? 'list-flat' : d === 'standard' ? 'list-unordered' : 'list-ordered'"
          :title="d === 'compact' ? '紧凑' : d === 'standard' ? '标准' : '舒适'"
          @click="s.density.value = d"
        ></vscode-button>
      </vscode-button-group>

      <span class="grow"></span>

      <span class="counts">
        <b>{{ total }}</b> / {{ props.project.counts.value.all }} 条
        <template v-if="s.checked.value.size">· 已选 {{ s.checked.value.size }}</template>
      </span>
    </div>

    <div class="bulk">
      <!-- @ts-expect-error vscode-button 是自定义元素 -->
      <vscode-button secondary @click="props.project.checkAllFiltered">全选当前结果</vscode-button>
      <!-- @ts-expect-error 自定义元素 -->
      <vscode-button secondary :disabled="!s.checked.value.size" @click="props.project.clearChecked">
        取消选择
      </vscode-button>
      <span class="sep"></span>
      <!-- @ts-expect-error 自定义元素 -->
      <vscode-single-select class="scope" :value="bulkScope" @change="bulkScope = readSelect($event) as 'filtered' | 'all'">
        <vscode-option value="filtered">范围：当前筛选结果</vscode-option>
        <vscode-option value="all">范围：全部条目</vscode-option>
      </vscode-single-select>
      <!-- @ts-expect-error 自定义元素 -->
      <vscode-button secondary icon="arrow-down" @click="fillFromSource">原文填入空译文</vscode-button>
      <!-- @ts-expect-error 自定义元素 -->
      <vscode-button secondary icon="clear-all" @click="clearTargets">清空译文</vscode-button>
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
        @contextmenu="rowMenu($event, entry)"
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
          <!-- @ts-expect-error vscode-textfield 是自定义元素 -->
          <vscode-textfield
            class="target-input"
            :class="{ filled: (entry.target ?? '').trim().length > 0 }"
            :value="entry.target ?? ''"
            :placeholder="entry.source ? '待翻译' : '—'"
            @focus="select(entry)"
            @input="onTargetInput(entry, $event)"
          ></vscode-textfield>
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
  background: var(--bg-editor);
}

.toolbar {
  display: flex;
  align-items: center;
  gap: var(--s4);
  padding: var(--s3) var(--s5);
  border-bottom: 1px solid var(--border-subtle);
  background: var(--bg-editor);
}

.search { position: relative; flex: 1; max-width: 420px; }
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

.density { flex: 0 0 auto; }

.grow { flex: 1; }
.counts { font-size: var(--fs-sm); color: var(--text-muted); font-variant-numeric: tabular-nums; }
.counts b { color: var(--text); }

.bulk {
  display: flex;
  align-items: center;
  gap: var(--s3);
  padding: var(--s3) var(--s5);
  background: var(--bg-editor);
  border-bottom: 1px solid var(--border-subtle);
  font-size: var(--fs-sm);
  color: var(--text-muted);
}

.sep { width: 1px; height: 14px; background: var(--border-subtle); }
.scope { width: 210px; flex: 0 0 auto; }

.grid {
  display: grid;
  grid-template-columns: 34px 84px minmax(180px, 1.1fr) minmax(200px, 1.4fr) minmax(200px, 1.4fr);
  align-items: center;
  gap: 0;
}

/* 表头与行都按 VS Code 的列表来：无行分隔线，靠 hover / 选中底色区分。 */
.head {
  padding: 0 var(--s5);
  background: var(--bg-editor);
  border-bottom: 1px solid var(--border-subtle);
  font-size: var(--fs-xs);
  font-weight: 600;
  color: var(--text-muted);
  position: sticky;
  top: 0;
  z-index: 2;
}

.scroller { flex: 1; overflow: auto; padding: 0 var(--s5) var(--s6); background: var(--bg-editor); }

.row { cursor: default; }

.row:hover { background: var(--list-hover); }
.row.selected { background: var(--list-active-bg); color: var(--list-active-fg); }
.row.selected:hover { background: var(--list-active-bg); }

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

.target-input { width: 100%; }
.target-input:not(.filled) { opacity: 0.85; }

.empty { padding: var(--s9) var(--s5); color: var(--text-faint); text-align: center; }
</style>
