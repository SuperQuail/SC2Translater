<script setup lang="ts">
// 设置：照 IntelliJ 的设置对话框 —— 左上搜索、左边分组树、右边面包屑 + 表单、底部 确定/取消/应用。
//
// 只有真的存在的设置才放进来（皮肤、栏宽、术语库、机翻、内置工具、快捷键、关于），
// 不为了凑格子造开关。

import { computed, ref } from "vue";

import type { AppInfo, MtProvider } from "../api/types";
import type { GlossaryApi } from "../composables/useGlossary";
import type { MtApi } from "../composables/useMt";
import type { SettingsDraft } from "../composables/settings";
import type { VcsApi } from "../composables/useVcs";
import BackgroundDialog from "./BackgroundDialog.vue";
import Icon from "./Icon.vue";

const props = defineProps<{
  draft: SettingsDraft;
  info: AppInfo | null;
  mt: MtApi;
  vcs: VcsApi;
  glossary: GlossaryApi;
  dataDir?: string;
  /** 从哪一页打开（默认外观）。 */
  initialPage?: string;
  /** 打开时是否直接把背景图像子对话框弹出来（截图 / 快捷键入口用）。 */
  openBackground?: boolean;
}>();
const emit = defineEmits<{
  (e: "close"): void;
  (e: "apply", draft: SettingsDraft): void;
  (e: "help"): void;
}>();

interface Node {
  id: string;
  label: string;
  group?: boolean;
}

const TREE: Node[] = [
  { id: "appearance", label: "外观与行为", group: true },
  { id: "appearance", label: "外观" },
  { id: "background", label: "背景图像" },
  { id: "view", label: "视图", group: true },
  { id: "layout", label: "面板与栏宽" },
  { id: "editor", label: "编辑器" },
  { id: "translate", label: "翻译", group: true },
  { id: "glossary", label: "术语库" },
  { id: "mt", label: "机翻" },
  { id: "tools", label: "工具", group: true },
  { id: "sc2diff", label: "内置 SC2Diff" },
  { id: "help", label: "帮助", group: true },
  { id: "keys", label: "快捷键" },
  { id: "about", label: "关于" },
];

const CRUMBS: Record<string, string[]> = {
  appearance: ["外观与行为", "外观"],
  background: ["外观与行为", "背景图像"],
  layout: ["视图", "面板与栏宽"],
  editor: ["视图", "编辑器"],
  glossary: ["翻译", "术语库"],
  mt: ["翻译", "机翻"],
  sc2diff: ["工具", "内置 SC2Diff"],
  keys: ["帮助", "快捷键"],
  about: ["帮助", "关于"],
};

const query = ref("");
const page = ref(props.initialPage ?? "appearance");
const backgroundOpen = ref(Boolean(props.openBackground));
const dirty = ref(false);

/** 本地副本，点「应用 / 确定」才交出去。 */
const local = ref<SettingsDraft>(JSON.parse(JSON.stringify(props.draft)) as SettingsDraft);
const mtDraft = ref({ ...props.mt.config.value });

const visible = computed(() => {
  const q = query.value.trim().toLowerCase();
  if (!q) return TREE.filter((n) => !n.group);
  return TREE.filter((n) => !n.group && n.label.toLowerCase().includes(q));
});

function touch() {
  dirty.value = true;
}

function readValue(event: Event): string {
  return String((event.target as unknown as { value: string }).value ?? "");
}

function readChecked(event: Event): boolean {
  return Boolean((event.target as unknown as { checked: boolean }).checked);
}

function set<K extends keyof SettingsDraft>(key: K, value: SettingsDraft[K]) {
  local.value = { ...local.value, [key]: value };
  touch();
}

function setMt(key: keyof typeof mtDraft.value, value: string) {
  mtDraft.value = { ...mtDraft.value, [key]: value };
  touch();
}

async function apply() {
  await props.mt.saveConfig({ ...mtDraft.value, timeout_secs: Number(mtDraft.value.timeout_secs) || 30 });
  emit("apply", JSON.parse(JSON.stringify(local.value)) as SettingsDraft);
  dirty.value = false;
}

async function ok() {
  await apply();
  emit("close");
}

const DENSITIES = [
  { value: "compact", label: "紧凑" },
  { value: "standard", label: "标准" },
  { value: "comfortable", label: "舒适" },
];

const TABS = [
  { value: "table", label: "键表" },
  { value: "kv", label: "键值文本" },
  { value: "json", label: "JSON" },
];

const PROVIDERS: Array<{ id: MtProvider; label: string; hint: string }> = [
  { id: "offline", label: "离线：术语库拼装", hint: "不联网。命中术语换成译名，其余保持原文。" },
  { id: "openai", label: "在线：OpenAI 兼容接口", hint: "官方 / DeepSeek / Ollama / one-api 网关都走这套。" },
];

const KEYS: Array<[string, string]> = [
  ["Ctrl+,", "打开设置"],
  ["Ctrl+K Ctrl+O", "打开文件夹"],
  ["Ctrl+F", "搜索（术语库面板打开时搜术语）"],
  ["Ctrl+E / Ctrl+I", "导出 / 导入译文 JSON"],
  ["Ctrl+Enter", "写回地图"],
  ["F5", "容器自检"],
  ["F1", "快捷键面板"],
];
</script>

<template>
  <div class="overlay" @click.self="emit('close')">
    <div class="dialog">
      <header class="title">
        <span class="app-icon">SC2</span>
        <span class="title-text">设置</span>
        <span class="grow"></span>
        <button class="close" title="关闭" @click="emit('close')"><Icon name="close" :size="14" /></button>
      </header>

      <div class="main">
        <aside class="tree">
          <!-- @ts-expect-error vscode-textfield 是自定义元素 -->
          <vscode-textfield
            class="search"
            :value="query"
            placeholder="搜索设置"
            @input="query = readValue($event)"
          >
            <!-- @ts-expect-error 自定义元素 -->
            <vscode-icon slot="content-before" name="search" :size="14" />
          </vscode-textfield>

          <nav class="nodes">
            <template v-for="node in TREE" :key="node.id + node.label">
              <div v-if="node.group" class="group" :class="{ dim: query && !visible.some((v) => v.id.length) }">
                {{ node.label }}
              </div>
              <button
                v-else-if="!query || visible.includes(node)"
                class="node"
                :class="{ active: page === node.id && !query }"
                @click="page = node.id; query = ''"
              >
                {{ node.label }}
              </button>
            </template>
            <div v-if="query && !visible.length" class="empty">没有匹配的设置。</div>
          </nav>
        </aside>

        <section class="content">
          <div class="crumb">{{ (CRUMBS[page] ?? []).join(" › ") }}</div>

          <div class="page">
            <!-- 外观 -->
            <template v-if="page === 'appearance'">
              <h3>皮肤</h3>
              <!-- @ts-expect-error vscode-radio-group 是自定义元素 -->
              <vscode-radio-group :value="local.skin" @change="set('skin', readValue($event) as 'vscode' | 'jb')">
                <!-- @ts-expect-error 自定义元素 -->
                <vscode-radio value="vscode">VS Code（默认）</vscode-radio>
                <p class="hint">控件与配色都来自 VS Code 官方组件库与 Dark Modern 主题。</p>
                <!-- @ts-expect-error 自定义元素 -->
                <vscode-radio value="jb">JetBrains</vscode-radio>
                <p class="hint">同一批控件，配色换成 JetBrains 官方 Ring UI 的暗色令牌。</p>
              </vscode-radio-group>

              <h3>行高密度</h3>
              <!-- @ts-expect-error 自定义元素 -->
              <vscode-single-select :value="local.density" @change="set('density', readValue($event))">
                <vscode-option v-for="d in DENSITIES" :key="d.value" :value="d.value">{{ d.label }}</vscode-option>
              </vscode-single-select>
              <p class="hint">键表的行高。条目多的时候用「紧凑」，一屏能多看几行。</p>

              <h3>背景图像</h3>
              <div class="rowLine">
                <!-- @ts-expect-error 自定义元素 -->
                <vscode-button secondary icon="image" @click="backgroundOpen = true">背景图像…</vscode-button>
                <span class="hint inline">
                  {{ local.background.path ? local.background.path : "未设置" }}
                </span>
              </div>
            </template>

            <!-- 背景图像（入口页） -->
            <template v-else-if="page === 'background'">
              <h3>背景图像</h3>
              <p class="hint">
                给编辑器垫一张底图。图片只记在本机，不会写进地图，也不会上传。
              </p>
              <div class="rowLine">
                <!-- @ts-expect-error 自定义元素 -->
                <vscode-button icon="image" @click="backgroundOpen = true">选择背景图像…</vscode-button>
                <!-- @ts-expect-error 自定义元素 -->
                <vscode-button
                  secondary
                  :disabled="!local.background.path"
                  @click="set('background', { ...local.background, path: '' })"
                >清除</vscode-button>
              </div>
              <table class="kv">
                <tbody>
                  <tr><td>当前图像</td><td>{{ local.background.path || "（未设置）" }}</td></tr>
                  <tr><td>不透明度</td><td>{{ local.background.opacity }}%</td></tr>
                  <tr><td>铺放</td><td>{{ local.background.placement }}</td></tr>
                  <tr><td>作用范围</td><td>{{ local.background.projectOnly ? "仅此项目" : "全局" }}</td></tr>
                </tbody>
              </table>
            </template>

            <!-- 面板与栏宽 -->
            <template v-else-if="page === 'layout'">
              <h3>面板</h3>
              <!-- @ts-expect-error 自定义元素 -->
              <vscode-checkbox :checked="local.left" label="打开时显示左栏" @change="set('left', readChecked($event))"></vscode-checkbox>
              <!-- @ts-expect-error 自定义元素 -->
              <vscode-checkbox :checked="local.right" label="打开时显示右栏（详情）" @change="set('right', readChecked($event))"></vscode-checkbox>

              <h3>栏宽</h3>
              <div class="rowLine">
                <label class="field">
                  <span>左栏</span>
                  <!-- @ts-expect-error 自定义元素 -->
                  <vscode-textfield type="number" :value="String(local.leftWidth)" @change="set('leftWidth', Number(readValue($event)) || 252)"></vscode-textfield>
                </label>
                <label class="field">
                  <span>右栏</span>
                  <!-- @ts-expect-error 自定义元素 -->
                  <vscode-textfield type="number" :value="String(local.rightWidth)" @change="set('rightWidth', Number(readValue($event)) || 336)"></vscode-textfield>
                </label>
              </div>
              <p class="hint">拖动分隔条也会改这两个值，并立刻记下来。</p>
            </template>

            <!-- 编辑器 -->
            <template v-else-if="page === 'editor'">
              <h3>默认打开的视图</h3>
              <!-- @ts-expect-error 自定义元素 -->
              <vscode-single-select :value="local.tabView" @change="set('tabView', readValue($event) as 'table' | 'kv' | 'json')">
                <vscode-option v-for="t in TABS" :key="t.value" :value="t.value">{{ t.label }}</vscode-option>
              </vscode-single-select>
              <p class="hint">三个视图操作同一份数据，随时可以切。</p>
            </template>

            <!-- 术语库 -->
            <template v-else-if="page === 'glossary'">
              <h3>全局术语库</h3>
              <p class="hint">所有项目共用。项目术语库跟着项目文件走，在项目里改。</p>
              <table class="kv">
                <tbody>
                  <tr><td>位置</td><td>{{ glossary.globalPath.value || "（首次保存时创建）" }}</td></tr>
                  <tr><td>词条数</td><td>{{ glossary.counts.value.global }}</td></tr>
                </tbody>
              </table>
              <div class="rowLine">
                <!-- @ts-expect-error 自定义元素 -->
                <vscode-button secondary icon="export" @click="glossary.exportFile('global', true)">导出 CSV</vscode-button>
                <!-- @ts-expect-error 自定义元素 -->
                <vscode-button secondary icon="import" @click="glossary.importFile('global')">导入</vscode-button>
              </div>
              <p class="hint">改动会自动保存，不需要手动存。</p>
            </template>

            <!-- 机翻 -->
            <template v-else-if="page === 'mt'">
              <h3>方式</h3>
              <!-- @ts-expect-error 自定义元素 -->
              <vscode-radio-group :value="mtDraft.provider" @change="setMt('provider', readValue($event))">
                <template v-for="p in PROVIDERS" :key="p.id">
                  <!-- @ts-expect-error 自定义元素 -->
                  <vscode-radio :value="p.id">{{ p.label }}</vscode-radio>
                  <p class="hint">{{ p.hint }}</p>
                </template>
              </vscode-radio-group>

              <template v-if="mtDraft.provider === 'openai'">
                <h3>接口</h3>
                <label class="field wide">
                  <span>接口地址</span>
                  <!-- @ts-expect-error 自定义元素 -->
                  <vscode-textfield :value="mtDraft.base_url" placeholder="https://api.openai.com/v1" @input="setMt('base_url', readValue($event))"></vscode-textfield>
                </label>
                <label class="field wide">
                  <span>API Key</span>
                  <!-- @ts-expect-error 自定义元素 -->
                  <vscode-textfield type="password" :value="mtDraft.api_key" @input="setMt('api_key', readValue($event))"></vscode-textfield>
                </label>
                <label class="field wide">
                  <span>模型</span>
                  <!-- @ts-expect-error 自定义元素 -->
                  <vscode-textfield :value="mtDraft.model" @input="setMt('model', readValue($event))"></vscode-textfield>
                </label>
                <p class="hint">Key 明文存在应用数据目录的 mt.json 里，不会上传到别处。</p>
              </template>

              <h3>目标语言</h3>
              <!-- @ts-expect-error 自定义元素 -->
              <vscode-textfield class="short" :value="mtDraft.target_language" @input="setMt('target_language', readValue($event))"></vscode-textfield>

              <h3>缓存</h3>
              <div class="rowLine">
                <!-- @ts-expect-error 自定义元素 -->
                <vscode-button secondary @click="mt.clearCache()">清空翻译缓存</vscode-button>
                <span class="hint inline">{{ mt.status.value }}</span>
              </div>
            </template>

            <!-- 内置 SC2Diff -->
            <template v-else-if="page === 'sc2diff'">
              <h3>内置 SC2Diff</h3>
              <p class="hint">随程序一起发，首次用到版本管理时静默装好，不需要你装工具链。</p>
              <table class="kv">
                <tbody>
                  <tr><td>状态</td><td>{{ vcs.tool.value?.installed ? "已就位" : "不可用" }}</td></tr>
                  <tr><td>版本</td><td>{{ vcs.tool.value?.version ?? "—" }}</td></tr>
                  <tr><td>位置</td><td>{{ vcs.tool.value?.path ?? "—" }}</td></tr>
                </tbody>
              </table>
              <div class="rowLine">
                <!-- @ts-expect-error 自定义元素 -->
                <vscode-button secondary icon="refresh" @click="vcs.ensureTool(true)">重新安装</vscode-button>
              </div>
            </template>

            <!-- 快捷键 -->
            <template v-else-if="page === 'keys'">
              <h3>常用快捷键</h3>
              <table class="kv">
                <tbody>
                  <tr v-for="[k, what] in KEYS" :key="k"><td class="mono">{{ k }}</td><td>{{ what }}</td></tr>
                </tbody>
              </table>
              <div class="rowLine">
                <!-- @ts-expect-error 自定义元素 -->
                <vscode-button secondary icon="question" @click="emit('help')">打开完整快捷键面板</vscode-button>
              </div>
            </template>

            <!-- 关于 -->
            <template v-else>
              <h3>SC2Translater</h3>
              <p class="hint">把《星际争霸 II》地图 / 模组里的翻译键提取出来、填回去。</p>
              <table class="kv">
                <tbody>
                  <tr><td>版本</td><td>{{ info?.version ?? "—" }}</td></tr>
                  <tr><td>许可证</td><td>MIT</td></tr>
                  <tr><td>数据目录</td><td>{{ dataDir ?? "（由桌面壳提供）" }}</td></tr>
                </tbody>
              </table>
              <p class="hint">不提供、不分发任何暴雪版权资源；不做破解，也不改游戏授权校验。</p>
            </template>
          </div>

          <footer class="bar">
            <button class="help" title="快捷键" @click="emit('help')">?</button>
            <span class="grow"></span>
            <!-- @ts-expect-error 自定义元素 -->
            <vscode-button @click="ok">确定</vscode-button>
            <!-- @ts-expect-error 自定义元素 -->
            <vscode-button secondary @click="emit('close')">取消</vscode-button>
            <!-- @ts-expect-error 自定义元素 -->
            <vscode-button secondary :disabled="!dirty" @click="apply">应用(A)</vscode-button>
          </footer>
        </section>
      </div>
    </div>

    <BackgroundDialog
      v-if="backgroundOpen"
      :value="local.background"
      @close="backgroundOpen = false"
      @save="set('background', $event); backgroundOpen = false"
    />
  </div>
</template>

<style scoped>
.overlay { position: fixed; inset: 0; background: rgba(0, 0, 0, 0.45); display: grid; place-items: center; z-index: 80; }
.dialog {
  width: min(1180px, 94vw);
  height: min(860px, 92vh);
  display: flex;
  flex-direction: column;
  background: var(--bg-app);
  border: 1px solid var(--border);
  border-radius: var(--r3);
  box-shadow: var(--shadow-pop);
  overflow: hidden;
}

.title { flex: 0 0 auto; display: flex; align-items: center; gap: var(--s4); height: 44px; padding: 0 var(--s5); }
.app-icon {
  width: 22px; height: 22px; border-radius: 3px;
  background: var(--accent); color: #fff;
  display: grid; place-items: center; font-size: 9px; font-weight: 700;
}
.title-text { font-size: var(--fs-lg); }
.grow { flex: 1; }
.close { width: 26px; height: 26px; padding: 0; justify-content: center; background: transparent; color: var(--text-muted); }
.close:hover { background: var(--list-hover); color: var(--text); }

.main { flex: 1; min-height: 0; display: flex; }

.tree { flex: 0 0 300px; display: flex; flex-direction: column; padding: var(--s4) var(--s4) var(--s4); gap: var(--s4); border-right: 1px solid var(--border-subtle); }
.search { width: 100%; }
.nodes { flex: 1; min-height: 0; overflow-y: auto; display: flex; flex-direction: column; gap: 1px; }
.group { padding: var(--s4) var(--s5) var(--s2); font-size: var(--fs-sm); font-weight: 600; color: var(--text); }
.group.dim { opacity: 0.5; }
.node {
  display: flex; align-items: center; height: 28px; padding: 0 var(--s5) 0 var(--s7);
  border: none; border-radius: var(--r1); background: transparent;
  color: var(--text-muted); font-size: var(--fs-md); justify-content: flex-start; width: 100%;
}
.node:hover { background: var(--list-hover); color: var(--text); }
.node.active { background: var(--list-active-bg); color: var(--list-active-fg); }
.empty { padding: var(--s5); font-size: var(--fs-sm); color: var(--text-faint); }

.content { flex: 1; min-width: 0; display: flex; flex-direction: column; }
.crumb { flex: 0 0 auto; padding: var(--s4) var(--s7) var(--s3); font-size: var(--fs-md); color: var(--text); border-bottom: 1px solid var(--border-subtle); }
.page { flex: 1; min-height: 0; overflow-y: auto; padding: var(--s5) var(--s7) var(--s7); }

/* 单选组默认是横向 flex，这里要的是「一项一行 + 每项下面一条说明」 */
vscode-radio-group {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 2px;
}
vscode-radio-group vscode-radio { margin-right: 0; }

h3 { font-size: var(--fs-md); font-weight: 600; color: var(--text); margin: var(--s6) 0 var(--s3); text-transform: none; letter-spacing: 0; }
h3:first-child { margin-top: 0; }

.hint { font-size: var(--fs-sm); color: var(--text-faint); line-height: 1.7; margin: 2px 0 var(--s3); }
.hint.inline { margin: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

.rowLine { display: flex; align-items: center; gap: var(--s4); flex-wrap: wrap; }
.field { display: flex; flex-direction: column; gap: var(--s2); font-size: var(--fs-sm); color: var(--text-muted); }
.field.wide { display: block; margin-bottom: var(--s4); max-width: 520px; }
.field.wide span { display: block; margin-bottom: var(--s2); font-size: var(--fs-sm); color: var(--text-muted); }
.short { max-width: 200px; }

table.kv { border-collapse: collapse; font-size: var(--fs-sm); margin: var(--s3) 0 var(--s5); }
table.kv td { padding: 4px var(--s5) 4px 0; color: var(--text-muted); vertical-align: top; }
table.kv td:first-child { width: 120px; color: var(--text-faint); }
.mono { font-family: var(--font-mono); }

.bar { flex: 0 0 auto; display: flex; align-items: center; gap: var(--s3); padding: var(--s4) var(--s5); border-top: 1px solid var(--border-subtle); }
.help {
  width: 28px; height: 28px; padding: 0; justify-content: center;
  border-radius: 50%; border: 1px solid var(--border-subtle);
  background: transparent; color: var(--text-muted);
}
.help:hover { background: var(--list-hover); color: var(--text); }
</style>
