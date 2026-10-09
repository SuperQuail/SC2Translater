<script setup lang="ts">
// SC2Translater 主界面：一条「选地图 -> 看诊断 -> 导出/回填/兜底」的直线流程。
// 业务都在 Rust 侧，这里只做展示与调用。

import { onMounted, ref } from "vue";

import { api, formatBytes, formatCoverage, isTauri, severityLabel } from "./api";
import type { AppInfo } from "./api/types";
import { COMMON_LOCALES, useTranslate } from "./composables/useTranslate";

const t = useTranslate();
const info = ref<AppInfo | null>(null);
const showFileDetail = ref(false);
/** 语言下拉框允许手填：归档里实测出现过 zhCH / enGB 等非官方 12 语言的代码。 */
const customTarget = ref("");

onMounted(async () => {
  info.value = await api.appInfo();
});

function applyCustomTarget() {
  const code = customTarget.value.trim();
  if (/^[A-Za-z]{2,4}$/.test(code)) {
    t.target.value = code;
    customTarget.value = "";
    void t.refreshAudit();
  }
}

function onTargetChange() {
  void t.refreshAudit();
}

function onSourceChange() {
  void t.refreshAudit();
}
</script>

<template>
  <div class="shell">
    <header class="top">
      <div class="brand">
        <span class="logo">SC2</span>
        <div>
          <h1>SC2Translater</h1>
          <p class="sub">星际争霸 II 地图 / 模组汉化工具 — 提取原始键、填回目标语言</p>
        </div>
      </div>
      <div class="meta">
        <span v-if="info">v{{ info.version }}</span>
        <span v-if="!isTauri" class="badge demo">浏览器演示模式</span>
      </div>
    </header>

    <main>
      <section class="card">
        <h2><span class="step">1</span>选择地图</h2>
        <div class="row">
          <button class="primary" :disabled="t.busy.value" @click="t.pickArchive">选择 .SC2Map / .SC2Mod</button>
          <span class="path mono">{{ t.archive.value ?? "尚未选择文件" }}</span>
        </div>
        <div class="row langs">
          <label>
            原文语言
            <select :value="t.source.value" :disabled="t.busy.value" @change="onSourceChange">
              <option v-for="l in COMMON_LOCALES" :key="l.code" :value="l.code">{{ l.label }}（{{ l.code }}）</option>
            </select>
          </label>
          <label>
            目标语言
            <select v-model="t.target.value" :disabled="t.busy.value" @change="onTargetChange">
              <option v-for="l in COMMON_LOCALES" :key="l.code" :value="l.code">{{ l.label }}（{{ l.code }}）</option>
            </select>
          </label>
          <label class="custom">
            其它代码
            <input v-model="customTarget" type="text" placeholder="如 jaJP" @keyup.enter="applyCustomTarget" />
          </label>
        </div>
      </section>

      <section v-if="t.audit.value" class="card">
        <h2><span class="step">2</span>诊断</h2>
        <div v-if="t.diagnosis.value" class="banner" :class="t.diagnosis.value.severity">
          <strong>{{ severityLabel(t.diagnosis.value.severity) }}</strong>
          <span>{{ t.diagnosis.value.message }}</span>
        </div>
        <div v-if="t.diagnosis.value?.actionable" class="hint">{{ t.diagnosis.value.actionable }}</div>

        <table>
          <thead>
            <tr>
              <th>语言</th>
              <th>期望键数</th>
              <th>已有</th>
              <th>缺失</th>
              <th>覆盖率</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="row in t.audit.value.locales" :key="row.locale">
              <td class="mono">{{ row.locale }}</td>
              <td>{{ row.keys_expected }}</td>
              <td>{{ row.keys_present }}</td>
              <td :class="{ bad: row.keys_missing > 0 }">{{ row.keys_missing }}</td>
              <td :class="{ bad: row.coverage < 1 }">{{ formatCoverage(row.coverage) }}</td>
            </tr>
          </tbody>
        </table>

        <button class="link" @click="showFileDetail = !showFileDetail">
          {{ showFileDetail ? "收起" : "展开" }}按文件明细（哪些表面向玩家）
        </button>
        <table v-if="showFileDetail" class="sub-table">
          <thead>
            <tr><th>字符串表</th><th>面向玩家</th><th>源键数</th><th>各语言覆盖率</th></tr>
          </thead>
          <tbody>
            <tr v-for="(meta, name) in t.audit.value.by_file" :key="name">
              <td class="mono">{{ name }}</td>
              <td>{{ meta.player_facing ? "是" : "编辑器专用" }}</td>
              <td>{{ meta.source_keys }}</td>
              <td class="mono small">
                {{ meta.locales.map((l) => l.locale + " " + formatCoverage(l.coverage)).join("  ") }}
              </td>
            </tr>
          </tbody>
        </table>

        <p v-if="t.audit.value.locales_absent.length" class="hint">
          归档里没有这些语言目录：{{ t.audit.value.locales_absent.join("、") }}
        </p>
      </section>

      <section v-if="t.archive.value" class="card">
        <h2><span class="step">3</span>处理</h2>
        <div class="row">
          <button :disabled="t.busy.value" @click="t.doExtract">① 导出翻译键（JSON）</button>
          <button :disabled="t.busy.value" @click="t.doApply">② 导入译文并写回</button>
          <button class="primary" :disabled="t.busy.value" @click="t.doSeed">③ 一键兜底（先用原文补满）</button>
        </div>
        <p class="hint">
          「一键兜底」把原文整份补进目标语言：游戏里不会再出现 <code>Param/Value/xxxxxxxx</code> 这类键路径，
          显示的是原文，等你把译文填进 JSON 再走 ②。
        </p>

        <div v-if="t.result.value" class="result">
          <div class="row">
            <span>产物</span>
            <span class="path mono">{{ t.result.value.output }}</span>
            <button @click="t.revealOutput">打开所在目录</button>
            <button @click="t.doVerify">容器自检</button>
          </div>
          <div class="row stats">
            <span>体积 {{ formatBytes(t.result.value.bytes_in) }} → {{ formatBytes(t.result.value.bytes_out) }}</span>
            <span v-if="t.result.value.seeded !== undefined">补齐 {{ t.result.value.seeded }} 个键</span>
            <span :class="{ bad: !t.result.value.ok }">回读校验 {{ t.result.value.ok ? "通过" : "失败" }}</span>
          </div>
          <table>
            <thead><tr><th>字符串表</th><th>新增</th><th>更新</th><th>合计</th></tr></thead>
            <tbody>
              <tr
                v-for="(item, name) in t.result.value.components_written[t.result.value.target_locale]"
                :key="name"
              >
                <td class="mono">{{ name }}</td>
                <td>{{ item.added }}</td>
                <td>{{ item.updated }}</td>
                <td>{{ item.total }}</td>
              </tr>
            </tbody>
          </table>
        </div>

        <div v-if="t.verifyReport.value" class="result">
          <div class="row stats">
            <span>dwRawChunkSize = {{ t.verifyReport.value.raw_chunk_size }}（必须为 0）</span>
            <span>HET 逐字节重建 {{ t.verifyReport.value.het_roundtrip_exact }}</span>
            <span>BET 逐字节重建 {{ t.verifyReport.value.bet_roundtrip_exact }}</span>
            <span :class="{ bad: !t.verifyReport.value.healthy }">整体 {{ t.verifyReport.value.healthy ? "健康" : "异常" }}</span>
          </div>
        </div>
      </section>

      <section v-if="t.logs.value.length || t.error.value" class="card">
        <h2>日志</h2>
        <div v-if="t.error.value" class="banner warning">{{ t.error.value }}</div>
        <pre class="log">{{ t.logs.value.join("\n") }}</pre>
      </section>
    </main>

    <footer>
      <p>
        为什么会出现「错误的路径」：星际争霸 II <strong>不会</strong>在语言缺失时回退到英文，
        它只加载与当前游戏语言同名的那一份语言目录。地图只带 <code>enUS.SC2Data</code> 时，
        中文环境下所有自定义键都查不到，游戏就把键本身当文本显示出来。
      </p>
      <p class="dim">
        CLI（面向 agent）：<code>sc2trans audit / extract / apply / seed / verify</code>，任何子命令加
        <code>--json</code> 输出机器可读结果。
      </p>
      <p v-if="t.busy.value" class="dim">进行中：{{ t.status.value }}</p>
    </footer>
  </div>
</template>

<style scoped>
.shell {
  max-width: 1060px;
  margin: 0 auto;
  padding: 22px 20px 40px;
  display: flex;
  flex-direction: column;
  gap: var(--gap-lg);
}

.top {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--gap);
}

.brand { display: flex; align-items: center; gap: 12px; }

.logo {
  display: grid;
  place-items: center;
  width: 42px;
  height: 42px;
  border-radius: var(--radius);
  background: var(--brand);
  color: #fff;
  font-weight: 700;
  letter-spacing: 0.5px;
}

h1 { font-size: 19px; }

.sub { margin: 2px 0 0; color: var(--text-dim); font-size: 12.5px; }

.meta { display: flex; align-items: center; gap: 10px; color: var(--text-faint); font-size: 12.5px; }

.badge {
  padding: 2px 8px;
  border-radius: 999px;
  border: 1px solid var(--border-strong);
}

.badge.demo { border-color: var(--warn); color: var(--warn); }

.card {
  background: var(--bg-raised);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: var(--pad);
  display: flex;
  flex-direction: column;
  gap: var(--gap);
}

h2 { font-size: 15px; display: flex; align-items: center; gap: 8px; }

.step {
  display: grid;
  place-items: center;
  width: 20px;
  height: 20px;
  border-radius: 50%;
  background: var(--brand);
  color: #fff;
  font-size: 12px;
}

.row { display: flex; align-items: center; gap: var(--gap); flex-wrap: wrap; }

.langs label { display: flex; align-items: center; gap: 8px; color: var(--text-dim); font-size: 13px; }

.path { color: var(--text-dim); word-break: break-all; }

.banner {
  padding: 10px 12px;
  border-radius: var(--radius-sm);
  border: 1px solid var(--border-strong);
  display: flex;
  gap: 10px;
  align-items: baseline;
}

.banner.warning { border-color: var(--warn); background: rgba(216, 161, 58, 0.08); }
.banner.ok { border-color: var(--ok); background: rgba(58, 166, 117, 0.08); }
.banner.info { border-color: var(--info); background: rgba(77, 140, 224, 0.08); }

.hint { color: var(--text-faint); font-size: 12.5px; margin: 0; }

.hint code, footer code { background: var(--bg-inset); padding: 1px 5px; border-radius: 4px; }

table { margin-top: 2px; }

.bad { color: var(--warn); }

.sub-table { font-size: 13px; }

.small { font-size: 12px; }

.link {
  align-self: flex-start;
  background: none;
  border: none;
  color: var(--brand);
  padding: 0;
  font-size: 13px;
}

.result { display: flex; flex-direction: column; gap: 8px; border-top: 1px solid var(--border); padding-top: var(--gap); }

.stats { color: var(--text-dim); font-size: 13px; gap: 18px; }

.log {
  margin: 0;
  max-height: 200px;
  overflow: auto;
  background: var(--bg-inset);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  padding: 10px;
  font-family: var(--mono);
  font-size: 12px;
  white-space: pre-wrap;
  color: var(--text-dim);
}

footer { color: var(--text-dim); font-size: 12.5px; border-top: 1px solid var(--border); padding-top: var(--gap); }
footer p { margin: 0 0 6px; }
.dim { color: var(--text-faint); }
</style>
