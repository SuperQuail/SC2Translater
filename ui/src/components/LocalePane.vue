<script setup lang="ts">
// 语言覆盖面板：项目进度 + 每种语言的覆盖率（右键可设为目标语言或补全它）。

import { formatCoverage } from "../api";
import type { ContextMenuApi } from "../composables/useContextMenu";
import type { Project } from "../composables/useProject";
import Icon from "./Icon.vue";

const props = defineProps<{ project: Project; menu: ContextMenuApi }>();
const s = props.project.state;

function localeMenu(event: MouseEvent, locale: string) {
  props.menu.openMenu(
    event,
    [
      { id: "target", label: "把目标语言设为 " + locale },
      { id: "fill", label: "补全 " + locale + "（用原文，不覆盖已有）" },
      { id: "fill-over", label: "用原文覆盖 " + locale + "（含已有译文）", danger: true, separator: true },
    ],
    (id) => {
      if (id === "target") void props.project.changeLanguages(s.source.value, locale);
      else if (id === "fill") void props.project.fillDefaultLanguages(locale, true, "");
      else void props.project.fillDefaultLanguages(locale, false, "");
    },
  );
}

const totals = () => props.project.projectTotals.value;
</script>

<template>
  <div class="locales">
    <header class="section">
      <span class="title">语言覆盖</span>
    </header>

    <section v-if="s.project.value.files.length" class="block">
      <h3>项目合计</h3>
      <div class="kv"><span>文件</span><b>{{ s.project.value.files.length }}</b></div>
      <div class="kv"><span>键总数</span><b>{{ totals().entries }}</b></div>
      <div class="kv"><span>待翻译</span><b class="warn">{{ totals().todo }}</b></div>
      <div class="kv"><span>完成度</span><b class="ok">{{ totals().percent.toFixed(1) }}%</b></div>
      <div class="bar"><div class="bar-fill" :class="{ bad: totals().percent < 100 }" :style="{ width: totals().percent + '%' }"></div></div>
    </section>

    <section class="block">
      <h3>当前文件</h3>
      <div v-if="!s.audit.value?.locales.length" class="empty">打开一个文件后这里会列出各语言的覆盖率。</div>
      <div
        v-for="row in s.audit.value?.locales ?? []"
        :key="row.locale"
        class="locale-row"
        @contextmenu="localeMenu($event, row.locale)"
      >
        <div class="head">
          <Icon name="globe" :size="12" />
          <span class="mono code">{{ row.locale }}</span>
          <span class="display">{{ row.display }}</span>
          <span class="pct" :class="{ bad: row.coverage < 1 }">{{ formatCoverage(row.coverage) }}</span>
        </div>
        <div class="bar small"><div class="bar-fill" :class="{ bad: row.coverage < 1 }" :style="{ width: row.coverage * 100 + '%' }"></div></div>
        <div class="sub">{{ row.keys_present }} / {{ row.keys_expected }} 个键</div>
      </div>
    </section>

    <section v-if="s.audit.value?.locales_absent.length" class="block">
      <h3>归档里没有的语言</h3>
      <div class="absent mono">{{ s.audit.value?.locales_absent.join("  ") }}</div>
      <p class="hint">游戏切到这些语言时，地图里的文本会显示成原始键路径。用「补全语言」一次补上。</p>
    </section>
  </div>
</template>

<style scoped>
.locales { display: flex; flex-direction: column; height: 100%; overflow-y: auto; background: var(--bg-sidebar); }

.section { padding: var(--s3) var(--s5); position: sticky; top: 0; background: var(--bg-panel); z-index: 2; }
.title { font-size: var(--fs-xs); font-weight: 600; letter-spacing: 0.06em; color: var(--text-muted); text-transform: uppercase; }

.block { padding: var(--s2) var(--s5) var(--s5); display: flex; flex-direction: column; gap: var(--s3); }
.block h3 { font-size: var(--fs-xs); }

.kv { display: flex; justify-content: space-between; font-size: var(--fs-sm); color: var(--text-muted); }
.kv b { color: var(--text); font-variant-numeric: tabular-nums; }
.kv b.warn { color: var(--warn-text); }
.kv b.ok { color: var(--ok-text); }

.bar { height: 4px; border-radius: 999px; background: var(--slate-5); overflow: hidden; }
.bar.small { height: 3px; }
.bar-fill { height: 100%; background: var(--accent); transition: width var(--t-base); }
.bar-fill.bad { background: var(--warn); }

.locale-row { display: flex; flex-direction: column; gap: var(--s2); padding: var(--s2) var(--s3); border-radius: var(--r2); }
.locale-row:hover { background: var(--list-hover); }
.head { display: flex; align-items: center; gap: var(--s3); font-size: var(--fs-xs); color: var(--text-muted); }
.code { color: var(--text); }
.display { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.pct { font-variant-numeric: tabular-nums; }
.pct.bad { color: var(--warn-text); }
.sub { font-size: 10px; color: var(--text-faint); }

.absent { font-size: var(--fs-xs); color: var(--warn-text); word-break: break-all; line-height: 1.7; }
.hint { font-size: var(--fs-xs); color: var(--text-faint); margin: 0; line-height: 1.6; }
.empty { font-size: var(--fs-xs); color: var(--text-faint); }
</style>
