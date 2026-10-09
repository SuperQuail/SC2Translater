<script setup lang="ts">
// 专有名词库面板（左栏之一）。
//
// 交互要点：打开本面板后 **Ctrl+F** 直接把光标放到搜索框（App 里按活动栏路由）。
// 词条分全局 / 项目两个作用域，项目覆盖全局；列表里可以直接改译名，改完自动存。

import { computed, ref } from "vue";

import type { ContextMenuApi } from "../composables/useContextMenu";
import type { GlossaryApi } from "../composables/useGlossary";
import type { Term, TermScope } from "../api/types";
import Icon from "./Icon.vue";

const props = defineProps<{ glossary: GlossaryApi; menu: ContextMenuApi }>();
const g = props.glossary;

const searchInput = ref<HTMLElement | null>(null);
const draftSource = ref("");
const draftTarget = ref("");
const draftScope = ref<TermScope>("global");

defineExpose({
  focusSearch: () => {
    const el = searchInput.value as unknown as { focus?: () => void } | null;
    el?.focus?.();
  },
});

const scopeOptions = [
  { value: "all", label: "全部作用域" },
  { value: "global", label: "全局" },
  { value: "project", label: "项目" },
];

function readValue(event: Event): string {
  return String((event.target as unknown as { value: string }).value ?? "");
}

function onSearchInput(event: Event) {
  g.query.value = readValue(event);
}

function onScopeChange(event: Event) {
  g.scopeFilter.value = readValue(event) as "all" | TermScope;
}

function editTarget(term: Term, event: Event) {
  g.upsert({ ...term, target: readValue(event) });
}

function addDraft() {
  const source = draftSource.value.trim();
  const target = draftTarget.value.trim();
  if (!source || !target) return;
  g.upsert({ source, target, scope: draftScope.value, case_sensitive: true });
  draftSource.value = "";
  draftTarget.value = "";
}

function termMenu(event: MouseEvent, term: Term) {
  props.menu.openMenu(
    event,
    [
      { id: "copy-src", label: "复制原文" },
      { id: "copy-tgt", label: "复制译名" },
      { id: "move-global", label: "移到全局术语库", disabled: term.scope === "global", separator: true },
      { id: "move-project", label: "移到项目术语库", disabled: term.scope === "project" },
      { id: "delete", label: "删除词条", danger: true, separator: true },
    ],
    (id) => {
      if (id === "copy-src") void navigator.clipboard.writeText(term.source);
      else if (id === "copy-tgt") void navigator.clipboard.writeText(term.target);
      else if (id === "move-global") {
        g.remove(term.id);
        g.upsert({ ...term, scope: "global" });
      } else if (id === "move-project") {
        g.remove(term.id);
        g.upsert({ ...term, scope: "project" });
      } else if (id === "delete") g.remove(term.id);
    },
  );
}

const headerTip = computed(
  () => "全局 " + g.counts.value.global + " 条 · 项目 " + g.counts.value.project + " 条 · 合并后 " + g.counts.value.merged + " 条",
);
</script>

<template>
  <div class="glossary">
    <header class="section">
      <span class="title">专有名词库</span>
      <!-- @ts-expect-error vscode-toolbar-container 是自定义元素 -->
      <vscode-toolbar-container class="actions">
        <!-- @ts-expect-error 自定义元素 -->
        <vscode-toolbar-button icon="add" label="新增词条" @click="draftSource = draftTarget = ''"></vscode-toolbar-button>
        <!-- @ts-expect-error 自定义元素 -->
        <vscode-toolbar-button icon="cloud-download" label="导入术语库（json/csv）" @click="g.importFile('global')"></vscode-toolbar-button>
        <!-- @ts-expect-error 自定义元素 -->
        <vscode-toolbar-button icon="export" label="导出全局术语库" @click="g.exportFile('global', true)"></vscode-toolbar-button>
        <!-- @ts-expect-error 自定义元素 -->
        <vscode-toolbar-button icon="refresh" label="重新载入" @click="g.refresh"></vscode-toolbar-button>
      </vscode-toolbar-container>
    </header>

    <div class="filters">
      <!-- @ts-expect-error vscode-textfield 是自定义元素 -->
      <vscode-textfield
        ref="searchInput"
        class="search"
        :value="g.query.value"
        placeholder="搜索术语（Ctrl+F）"
        @input="onSearchInput"
      >
        <!-- @ts-expect-error 自定义元素 -->
        <vscode-icon slot="content-before" name="search" :size="14" />
      </vscode-textfield>
      <!-- @ts-expect-error vscode-single-select 是自定义元素 -->
      <vscode-single-select :value="g.scopeFilter.value" @change="onScopeChange">
        <vscode-option v-for="o in scopeOptions" :key="o.value" :value="o.value">{{ o.label }}</vscode-option>
      </vscode-single-select>
    </div>

    <div class="meta">
      <span>{{ headerTip }}</span>
      <span v-if="g.query.value">· 命中 {{ g.filtered.value.length }}</span>
    </div>
    <div v-if="g.status.value" class="status">{{ g.status.value }}</div>
    <div v-if="g.error.value" class="status err">{{ g.error.value }}</div>

    <div class="list">
      <div
        v-for="term in g.filtered.value"
        :key="term.id"
        class="row"
        @contextmenu="termMenu($event, term)"
      >
        <div class="line1">
          <Icon name="text" :size="12" />
          <span class="src mono" :title="term.source">{{ term.source }}</span>
          <span class="scope" :class="term.scope">{{ term.scope === "global" ? "全局" : "项目" }}</span>
          <button class="del" title="删除" @click="g.remove(term.id)"><Icon name="trash" :size="12" /></button>
        </div>
        <div class="line2">
          <!-- @ts-expect-error vscode-textfield 是自定义元素 -->
          <vscode-textfield
            class="tgt"
            :value="term.target"
            placeholder="译名"
            @change="editTarget(term, $event)"
          ></vscode-textfield>
        </div>
      </div>

      <div v-if="!g.filtered.value.length" class="empty">
        <template v-if="g.query.value">没有匹配的术语。</template>
        <template v-else>还没有词条。在下面添加，或点工具栏的「导入术语库」。</template>
      </div>
    </div>

    <div class="add">
      <h3>新增</h3>
      <!-- @ts-expect-error 自定义元素 -->
      <vscode-textfield
        :value="draftSource"
        placeholder="原文，如 Infested Terran"
        @input="draftSource = readValue($event)"
      ></vscode-textfield>
      <!-- @ts-expect-error 自定义元素 -->
      <vscode-textfield
        :value="draftTarget"
        placeholder="译名，如 被感染的陆战队员"
        @input="draftTarget = readValue($event)"
      ></vscode-textfield>
      <div class="addRow">
        <!-- @ts-expect-error 自定义元素 -->
        <vscode-single-select :value="draftScope" @change="draftScope = readValue($event) as TermScope">
          <vscode-option value="global">加到全局</vscode-option>
          <vscode-option value="project">加到项目</vscode-option>
        </vscode-single-select>
        <!-- @ts-expect-error 自定义元素 -->
        <vscode-button :disabled="!draftSource.trim() || !draftTarget.trim()" @click="addDraft">添加词条</vscode-button>
      </div>
      <p class="hint">
        项目术语库保存在项目文件旁边；全局术语库在应用数据目录，所有项目共用。
        查表时**项目覆盖全局**。
      </p>
    </div>
  </div>
</template>

<style scoped>
.glossary { display: flex; flex-direction: column; height: 100%; overflow: hidden; background: var(--bg-sidebar); }

.section {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 24px;
  padding: 0 var(--s2) 0 var(--s5);
  flex: 0 0 auto;
}
.title { font-size: var(--fs-xs); font-weight: 700; letter-spacing: 0.04em; color: var(--text-muted); text-transform: uppercase; }
.actions { display: flex; }

.filters { display: flex; gap: var(--s2); padding: var(--s2) var(--s4); flex: 0 0 auto; }
.search { flex: 1; min-width: 0; }

.meta { padding: 0 var(--s5) var(--s2); font-size: var(--fs-xs); color: var(--text-faint); }
.status { padding: 0 var(--s5) var(--s2); font-size: var(--fs-xs); color: var(--ok-text); }
.status.err { color: var(--err-text); }

.list { flex: 1; min-height: 0; overflow-y: auto; padding: 0 var(--s4) var(--s4); display: flex; flex-direction: column; gap: var(--s2); }

.row { padding: var(--s2) var(--s3); border-radius: var(--r2); }
.row:hover { background: var(--list-hover); }

.line1 { display: flex; align-items: center; gap: var(--s3); font-size: var(--fs-xs); }
.src { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--text); }
.scope {
  flex: 0 0 auto;
  font-size: 10px;
  padding: 0 4px;
  border-radius: var(--r1);
  background: var(--badge-background, #616161);
  color: var(--badge-foreground, #fff);
}
.scope.project { background: var(--accent); }
.del {
  width: 18px; height: 18px; padding: 0; justify-content: center;
  background: transparent; color: var(--text-faint); opacity: 0;
}
.row:hover .del { opacity: 1; }
.del:hover { color: var(--err-text); background: transparent; }

.line2 { margin-top: 2px; }
.tgt { width: 100%; }

.empty { padding: var(--s6) var(--s4); font-size: var(--fs-sm); color: var(--text-faint); }

.add {
  flex: 0 0 auto;
  border-top: 1px solid var(--border-sidebar);
  padding: var(--s4) var(--s4) var(--s5);
  display: flex;
  flex-direction: column;
  gap: var(--s3);
}
.add h3 { font-size: var(--fs-xs); }
.addRow { display: flex; gap: var(--s3); align-items: center; }
.hint { font-size: var(--fs-xs); color: var(--text-faint); margin: 0; line-height: 1.6; }
</style>
