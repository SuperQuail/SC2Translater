<script setup lang="ts">
// 机翻设置：选 provider、填接口。默认是离线（不需要任何配置）。

import { ref } from "vue";

import type { MtApi } from "../composables/useMt";
import type { MtConfig, MtProvider } from "../api/types";

const props = defineProps<{ mt: MtApi }>();
const emit = defineEmits<{ (e: "close"): void }>();

const draft = ref<MtConfig>({ ...props.mt.config.value });

const PROVIDERS: Array<{ id: MtProvider; label: string; hint: string }> = [
  { id: "offline", label: "离线：术语库拼装", hint: "不联网。把命中的术语换成译名，其余保持原文 —— 术语是对的，语序要自己来。" },
  { id: "openai", label: "在线：OpenAI 兼容接口", hint: "填 base_url 与 api_key。官方、DeepSeek、本地 Ollama、one-api 网关都走这套。" },
];

function readValue(event: Event): string {
  return String((event.target as unknown as { value: string }).value ?? "");
}

function setField<K extends keyof MtConfig>(key: K, value: MtConfig[K]) {
  draft.value = { ...draft.value, [key]: value };
}

async function save() {
  await props.mt.saveConfig({ ...draft.value, timeout_secs: Number(draft.value.timeout_secs) || 30 });
  emit("close");
}
</script>

<template>
  <div class="overlay" @click.self="emit('close')">
    <div class="dialog">
      <header>
        <h2>机翻设置</h2>
        <!-- @ts-expect-error vscode-button 是自定义元素 -->
        <vscode-button secondary icon-only icon="close" title="关闭" @click="emit('close')"></vscode-button>
      </header>

      <!-- @ts-expect-error vscode-radio-group 是自定义元素 -->
      <vscode-radio-group :value="draft.provider" @change="setField('provider', readValue($event) as MtProvider)">
        <template v-for="p in PROVIDERS" :key="p.id">
          <!-- @ts-expect-error 自定义元素 -->
          <vscode-radio :value="p.id">{{ p.label }}</vscode-radio>
          <p class="hint">{{ p.hint }}</p>
        </template>
      </vscode-radio-group>

      <div v-if="draft.provider === 'openai'" class="fields">
        <label>
          <span>接口地址（base_url）</span>
          <!-- @ts-expect-error 自定义元素 -->
          <vscode-textfield :value="draft.base_url" placeholder="https://api.openai.com/v1" @input="setField('base_url', readValue($event))"></vscode-textfield>
        </label>
        <label>
          <span>API Key</span>
          <!-- @ts-expect-error 自定义元素 -->
          <vscode-textfield type="password" :value="draft.api_key" placeholder="sk-..." @input="setField('api_key', readValue($event))"></vscode-textfield>
        </label>
        <label>
          <span>模型</span>
          <!-- @ts-expect-error 自定义元素 -->
          <vscode-textfield :value="draft.model" placeholder="gpt-4o-mini / deepseek-chat" @input="setField('model', readValue($event))"></vscode-textfield>
        </label>
        <p class="hint">Key 明文存在应用数据目录的 mt.json 里，不会上传到别处。</p>
      </div>

      <label class="target">
        <span>目标语言</span>
        <!-- @ts-expect-error 自定义元素 -->
        <vscode-textfield :value="draft.target_language" placeholder="简体中文" @input="setField('target_language', readValue($event))"></vscode-textfield>
      </label>

      <div class="cacheRow">
        <!-- @ts-expect-error 自定义元素 -->
        <vscode-button secondary @click="props.mt.clearCache()">清空翻译缓存</vscode-button>
        <span class="hint">{{ props.mt.status.value }}</span>
      </div>

      <footer>
        <!-- @ts-expect-error 自定义元素 -->
        <vscode-button secondary @click="emit('close')">取消</vscode-button>
        <!-- @ts-expect-error 自定义元素 -->
        <vscode-button @click="save">保存</vscode-button>
      </footer>
    </div>
  </div>
</template>

<style scoped>
.overlay { position: fixed; inset: 0; background: rgba(6, 8, 12, 0.62); display: grid; place-items: center; z-index: 78; }
.dialog {
  width: 560px; max-height: 86vh; overflow-y: auto;
  background: var(--bg-panel); border: 1px solid var(--border); border-radius: var(--r3);
  box-shadow: var(--shadow-pop); padding: var(--s6);
  display: flex; flex-direction: column; gap: var(--s5);
}
header { display: flex; align-items: center; justify-content: space-between; }
h2 { font-size: var(--fs-lg); }
.hint { margin: 0; font-size: var(--fs-xs); color: var(--text-faint); line-height: 1.6; }
.fields { display: flex; flex-direction: column; gap: var(--s4); }
label { display: flex; flex-direction: column; gap: var(--s2); font-size: var(--fs-sm); color: var(--text-muted); }
.cacheRow { display: flex; align-items: center; gap: var(--s4); }
footer { display: flex; justify-content: flex-end; gap: var(--s3); }
</style>
