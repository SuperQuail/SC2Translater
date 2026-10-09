<script setup lang="ts">
// 顶栏：归档 + 语言对 + 主操作。主操作按「先用得上的排前面」排列。

import { computed, ref, watch } from "vue";

import { baseName, formatBytes, isTauri } from "../api";
import type { AppInfo } from "../api/types";
import { COMMON_LOCALES } from "../composables/useProject";
import type { Project } from "../composables/useProject";

const props = defineProps<{ project: Project; info: AppInfo | null }>();
const emit = defineEmits<{ (e: "fill"): void }>();

const s = props.project.state;

/** 语言代码不止官方那 12 个（实测有 zhCH / enGB / jaJP），所以给一个「自定义」出口。 */
const CUSTOM = "__custom__";
const sourceChoice = ref(s.source.value);
const targetChoice = ref(s.target.value);
const customSourceOpen = ref(false);
const customTargetOpen = ref(false);
const customSource = ref("");
const customTarget = ref("");

watch(s.source, (v) => { sourceChoice.value = COMMON_LOCALES.some((l) => l.code === v) ? v : CUSTOM; });
watch(s.target, (v) => { targetChoice.value = COMMON_LOCALES.some((l) => l.code === v) ? v : CUSTOM; });

function onSourceChoice() {
  if (sourceChoice.value === CUSTOM) {
    customSourceOpen.value = true;
    return;
  }
  void props.project.changeLanguages(sourceChoice.value, s.target.value);
}

function onTargetChoice() {
  if (targetChoice.value === CUSTOM) {
    customTargetOpen.value = true;
    return;
  }
  void props.project.changeLanguages(s.source.value, targetChoice.value);
}

function applyCustom(which: "source" | "target") {
  const raw = (which === "source" ? customSource.value : customTarget.value).trim();
  if (!/^[A-Za-z]{2,4}$/.test(raw)) return;
  if (which === "source") {
    customSourceOpen.value = false;
    customSource.value = "";
    void props.project.changeLanguages(raw, s.target.value);
  } else {
    customTargetOpen.value = false;
    customTarget.value = "";
    void props.project.changeLanguages(s.source.value, raw);
  }
}

const archiveLabel = computed(() => (s.archive.value ? baseName(s.archive.value) : "未打开归档"));
const archiveMeta = computed(() =>
  props.project.state.info.value ? formatBytes(props.project.state.info.value.size) : "",
);
</script>

<template>
  <header class="titlebar">
    <div class="brand">
      <span class="mark">SC2</span>
      <div class="brand-text">
        <div class="name">SC2Translater</div>
        <div class="tag">地图汉化 · 翻译键提取与回填</div>
      </div>
      <span v-if="info" class="version">v{{ info.version }}</span>
      <span v-if="!isTauri" class="badge-demo">演示数据</span>
    </div>

    <div class="center">
      <button class="archive-chip" :disabled="s.busy.value" @click="props.project.openArchive">
        <span class="dot" :class="{ ready: Boolean(s.archive.value) }"></span>
        <span class="archive-name">{{ archiveLabel }}</span>
        <span v-if="archiveMeta" class="archive-meta">{{ archiveMeta }}</span>
        <span class="archive-action">{{ s.archive.value ? "更换" : "打开" }}</span>
      </button>

      <div class="langs">
        <label class="lang">
          <span class="lang-label">原文</span>
          <select v-model="sourceChoice" :disabled="s.busy.value" @change="onSourceChoice">
            <option v-for="l in COMMON_LOCALES" :key="l.code" :value="l.code">{{ l.label }} · {{ l.code }}</option>
            <option :value="CUSTOM">自定义…</option>
          </select>
        </label>
        <span class="arrow">→</span>
        <label class="lang">
          <span class="lang-label">目标</span>
          <select v-model="targetChoice" :disabled="s.busy.value" @change="onTargetChoice">
            <option v-for="l in COMMON_LOCALES" :key="l.code" :value="l.code">{{ l.label }} · {{ l.code }}</option>
            <option :value="CUSTOM">自定义…</option>
          </select>
        </label>
      </div>
    </div>

    <div class="actions">
      <button :disabled="!s.doc.value || s.busy.value" title="导入外部翻译 JSON，按 (字符串表, 键) 合并译文" @click="props.project.importJson">
        导入译文
      </button>
      <button :disabled="!s.doc.value || s.busy.value" title="导出翻译键为 JSON（Ctrl+S）" @click="props.project.exportJson">
        导出 JSON
      </button>
      <button
        class="primary"
        :disabled="!s.doc.value || s.busy.value"
        title="把译文写回地图（Ctrl+Enter）"
        @click="props.project.applyToArchive"
      >
        写回地图
      </button>
      <button
        :disabled="!s.doc.value || s.busy.value"
        title="没有语言文件的语言，一次全部补上"
        @click="emit('fill')"
      >
        补全语言
      </button>
      <button :disabled="!s.doc.value || s.busy.value" title="只用原文补当前目标语言" @click="props.project.seedMissing">
        一键兜底
      </button>
      <button class="ghost" :disabled="s.busy.value" title="容器自检" @click="props.project.runVerify">自检</button>
    </div>

    <div v-if="customSourceOpen || customTargetOpen" class="custom-pop">
      <h3>{{ customSourceOpen ? "自定义原文语言代码" : "自定义目标语言代码" }}</h3>
      <input
        v-if="customSourceOpen"
        v-model="customSource"
        type="text"
        placeholder="如 zhCH"
        autofocus
        @keyup.enter="applyCustom('source')"
        @keyup.esc="customSourceOpen = false"
      />
      <input
        v-else
        v-model="customTarget"
        type="text"
        placeholder="如 jaJP"
        autofocus
        @keyup.enter="applyCustom('target')"
        @keyup.esc="customTargetOpen = false"
      />
      <p class="hint">2~4 个字母。归档里实测出现过 enCN / enGB / enSG / enTW / jaJP / zhCH。</p>
      <div class="pop-actions">
        <button @click="customSourceOpen = false; customTargetOpen = false">取消</button>
        <button class="primary" @click="applyCustom(customSourceOpen ? 'source' : 'target')">确定</button>
      </div>
    </div>
  </header>
</template>

<style scoped>
.titlebar {
  position: relative;
  height: var(--h-titlebar);
  flex: 0 0 auto;
  display: flex;
  align-items: center;
  gap: var(--s6);
  padding: 0 var(--s5);
  background: var(--bg-panel);
  border-bottom: 1px solid var(--border-subtle);
}

.brand { display: flex; align-items: center; gap: var(--s4); min-width: 0; }

.mark {
  display: grid;
  place-items: center;
  width: 28px;
  height: 28px;
  border-radius: var(--r2);
  background: linear-gradient(160deg, var(--blue-9), var(--blue-7));
  color: #fff;
  font-size: var(--fs-xs);
  font-weight: 700;
  letter-spacing: 0.02em;
}

.brand-text { min-width: 0; }
.name { font-size: var(--fs-md); font-weight: 600; letter-spacing: -0.01em; }
.tag { font-size: var(--fs-xs); color: var(--text-faint); line-height: 1.2; }

.version { font-size: var(--fs-xs); color: var(--text-faint); }

.badge-demo {
  font-size: var(--fs-xs);
  padding: 1px var(--s3);
  border-radius: 999px;
  border: 1px solid var(--amber-9);
  color: var(--amber-11);
}

.center { display: flex; align-items: center; gap: var(--s5); flex: 1; min-width: 0; }

.archive-chip {
  max-width: 340px;
  height: 30px;
  gap: var(--s3);
  background: var(--bg-raised);
  border-color: var(--border-subtle);
}

.dot { width: 7px; height: 7px; border-radius: 50%; background: var(--slate-7); flex: 0 0 auto; }
.dot.ready { background: var(--ok); }

.archive-name { font-size: var(--fs-sm); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.archive-meta { font-size: var(--fs-xs); color: var(--text-faint); }
.archive-action { font-size: var(--fs-xs); color: var(--accent-text); }

.langs { display: flex; align-items: center; gap: var(--s3); }
.lang { display: flex; align-items: center; gap: var(--s3); }
.lang-label { font-size: var(--fs-xs); color: var(--text-faint); white-space: nowrap; }
.lang select { width: auto; height: 28px; font-size: var(--fs-sm); padding: 0 var(--s4); }
.arrow { color: var(--text-faint); font-size: var(--fs-sm); }

.actions { display: flex; align-items: center; gap: var(--s3); }

.custom-pop {
  position: absolute;
  top: calc(var(--h-titlebar) + var(--s2));
  left: 50%;
  transform: translateX(-50%);
  width: 320px;
  padding: var(--s5);
  display: flex;
  flex-direction: column;
  gap: var(--s4);
  background: var(--slate-3);
  border: 1px solid var(--border);
  border-radius: var(--r3);
  box-shadow: var(--shadow-pop);
  z-index: 40;
}

.custom-pop h3 { text-transform: none; letter-spacing: 0; font-size: var(--fs-sm); color: var(--text); }
.hint { font-size: var(--fs-xs); color: var(--text-faint); margin: 0; }
.pop-actions { display: flex; justify-content: flex-end; gap: var(--s3); }
</style>
