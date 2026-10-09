<script setup lang="ts">
// 背景图像子对话框（照 IntelliJ 的那个）。
//
// 图像路径 + 不透明度 + 铺放方式（四格预览）+ 作用范围 + 目标，底部三个按钮。

import { ref } from "vue";

import { api } from "../api";
import type { BackgroundPrefs } from "../composables/settings";
import Icon from "./Icon.vue";

const props = defineProps<{ value: BackgroundPrefs }>();
const emit = defineEmits<{
  (e: "close"): void;
  (e: "save", value: BackgroundPrefs): void;
}>();

const draft = ref<BackgroundPrefs>({ ...props.value });

const PLACEMENTS = [
  { id: "tile", label: "平铺" },
  { id: "center", label: "居中" },
  { id: "fit", label: "适应" },
  { id: "stretch", label: "拉伸" },
];

function readValue(event: Event): string {
  return String((event.target as unknown as { value: string }).value ?? "");
}

function set<K extends keyof BackgroundPrefs>(key: K, value: BackgroundPrefs[K]) {
  draft.value = { ...draft.value, [key]: value };
}

async function pick() {
  try {
    const path = await api.pickImage();
    if (path) set("path", path);
  } catch {
    // 浏览器调试时没有原生对话框，静默忽略
  }
}

function clampOpacity(raw: number): number {
  const n = Number.isFinite(raw) ? Math.round(raw) : 15;
  return Math.min(100, Math.max(0, n));
}
</script>

<template>
  <div class="overlay" @click.self="emit('close')">
    <div class="dialog">
      <header class="title">
        <span class="app-icon">SC2</span>
        <span class="title-text">背景图像</span>
        <span class="grow"></span>
        <button class="close" title="关闭" @click="emit('close')"><Icon name="close" :size="14" /></button>
      </header>

      <div class="body">
        <label class="line">
          <span class="label">图像:</span>
          <!-- @ts-expect-error vscode-textfield 是自定义元素 -->
          <vscode-textfield
            class="path"
            :value="draft.path"
            placeholder="还没选图片"
            @input="set('path', readValue($event))"
          ></vscode-textfield>
          <!-- @ts-expect-error 自定义元素 -->
          <vscode-button secondary icon-only icon="folder-opened" title="选择图片" @click="pick"></vscode-button>
        </label>

        <label class="line">
          <span class="label">不透明度:</span>
          <input
            class="slider"
            type="range"
            min="0"
            max="100"
            :value="draft.opacity"
            @input="set('opacity', clampOpacity(Number(readValue($event))))"
          />
          <!-- @ts-expect-error 自定义元素 -->
          <vscode-textfield
            class="num"
            type="number"
            :value="String(draft.opacity)"
            @change="set('opacity', clampOpacity(Number(readValue($event))))"
          ></vscode-textfield>
        </label>

        <div class="previews">
          <button
            v-for="p in PLACEMENTS"
            :key="p.id"
            class="cell"
            :class="[p.id, { active: draft.placement === p.id }]"
            :title="p.label"
            @click="set('placement', p.id)"
          >
            <span class="thumb" :class="p.id"></span>
            <span class="cellLabel">{{ p.label }}</span>
          </button>
        </div>

        <div class="line">
          <span class="label"></span>
          <!-- @ts-expect-error vscode-checkbox 是自定义元素 -->
          <vscode-checkbox
            :checked="draft.projectOnly"
            label="仅此项目"
            @change="set('projectOnly', Boolean(($event.target as unknown as { checked: boolean }).checked))"
          ></vscode-checkbox>
        </div>

        <p class="hint">
          图片路径记在设置里，不会写进地图。桌面版选完文件会直接填路径，浏览器调试时手动填。
        </p>
      </div>

      <footer class="bar">
        <!-- @ts-expect-error 自定义元素 -->
        <vscode-button :disabled="!draft.path" @click="emit('save', draft)">应用</vscode-button>
        <!-- @ts-expect-error 自定义元素 -->
        <vscode-button secondary @click="emit('close')">取消</vscode-button>
        <!-- @ts-expect-error 自定义元素 -->
        <vscode-button secondary @click="emit('save', { ...draft, path: '' })">清除并关闭</vscode-button>
      </footer>
    </div>
  </div>
</template>

<style scoped>
.overlay { position: fixed; inset: 0; background: rgba(0, 0, 0, 0.5); display: grid; place-items: center; z-index: 90; }
.dialog {
  width: 660px;
  max-width: 94vw;
  background: var(--bg-app);
  border: 1px solid var(--border);
  border-radius: var(--r3);
  box-shadow: var(--shadow-pop);
  overflow: hidden;
}

.title { display: flex; align-items: center; gap: var(--s4); height: 42px; padding: 0 var(--s5); }
.app-icon {
  width: 20px; height: 20px; border-radius: 3px;
  background: var(--accent); color: #fff;
  display: grid; place-items: center; font-size: 9px; font-weight: 700;
}
.title-text { font-size: var(--fs-md); }
.grow { flex: 1; }
.close { width: 26px; height: 26px; padding: 0; justify-content: center; background: transparent; color: var(--text-muted); }
.close:hover { background: var(--list-hover); color: var(--text); }

.body { padding: var(--s5) var(--s6) var(--s6); display: flex; flex-direction: column; gap: var(--s5); }

.line { display: flex; align-items: center; gap: var(--s4); }
.label { flex: 0 0 84px; font-size: var(--fs-md); color: var(--text-muted); text-align: right; }
.path { flex: 1; min-width: 0; }
.num { width: 78px; flex: 0 0 auto; }

.slider { flex: 1; accent-color: var(--accent); height: 4px; }

.previews { display: flex; gap: var(--s3); padding-left: 100px; }
/* 全局 button 规则给了固定 26px 高，会把缩略图压扁，先重置 */
.cell {
  display: flex; flex-direction: column; align-items: center; justify-content: center;
  gap: 4px;
  height: auto;
  padding: 4px; border: 1px solid transparent; border-radius: var(--r2);
  background: transparent;
}
.cell.active { border-color: var(--accent); background: var(--list-hover); }

/* 缩略图：棋盘底 + 一块「图片」，按铺放方式画不同样子 */
.thumb {
  display: block; width: 64px; height: 44px;
  border: 1px solid var(--border-subtle);
  background-color: var(--bg-inset);
  background-image:
    linear-gradient(45deg, rgba(255, 255, 255, 0.06) 25%, transparent 25%, transparent 75%, rgba(255, 255, 255, 0.06) 75%),
    linear-gradient(45deg, rgba(255, 255, 255, 0.06) 25%, transparent 25%, transparent 75%, rgba(255, 255, 255, 0.06) 75%);
  background-size: 10px 10px;
  background-position: 0 0, 5px 5px;
  position: relative;
  overflow: hidden;
}
.thumb::after { content: ""; position: absolute; background: var(--accent); opacity: 0.55; }
.thumb.tile::after {
  inset: 0;
  background:
    repeating-linear-gradient(90deg, var(--accent) 0 12px, transparent 12px 24px),
    repeating-linear-gradient(0deg, var(--accent) 0 12px, transparent 12px 24px);
  opacity: 0.18;
}
.thumb.center::after { left: 22px; top: 14px; width: 20px; height: 16px; }
.thumb.fit::after { inset: 0 8px; }
.thumb.stretch::after { inset: 0; }

.cellLabel { font-size: 11px; color: var(--text-faint); }
.cell.active .cellLabel { color: var(--text); }

.hint { font-size: var(--fs-sm); color: var(--text-faint); line-height: 1.7; margin: 0; }

.bar { display: flex; justify-content: flex-end; gap: var(--s3); padding: var(--s4) var(--s5); border-top: 1px solid var(--border-subtle); }
</style>
