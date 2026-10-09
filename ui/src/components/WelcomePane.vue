<script setup lang="ts">
// 未打开归档时的首屏。把「为什么会出现键路径」讲清楚，再给一个明确的入口。

import { isTauri } from "../api";

const props = defineProps<{ dropping?: boolean }>();
defineEmits<{ (e: "open"): void }>();
</script>

<template>
  <div class="welcome" :class="{ dropping: props.dropping }">
    <div class="inner">
      <h1>把地图里的<em>原始键</em>换成能读的文本</h1>
      <p class="lead">
        星际争霸 II 不会在语言缺失时回退到英文 —— 它只加载与当前游戏语言同名的那一份语言目录。
        地图只带 <code>enUS.SC2Data</code> 时，中文环境下所有自定义键都查不到，
        游戏就把键本身当文本显示，于是界面上出现 <code>Param/Value/0F5D0539</code> 这种东西。
      </p>

      <div class="steps">
        <div class="step">
          <span class="no">1</span>
          <div>
            <b>打开地图</b>
            <span>选一张 .SC2Map / .SC2Mod。工具会列出所有翻译键，并算出每种语言的覆盖率。</span>
          </div>
        </div>
        <div class="step">
          <span class="no">2</span>
          <div>
            <b>补齐或翻译</b>
            <span>一键兜底先用原文把键填满（游戏里至少不再显示路径），或者在表格里逐条写译文。</span>
          </div>
        </div>
        <div class="step">
          <span class="no">3</span>
          <div>
            <b>写回地图</b>
            <span>生成新的归档，原有组件逐字节保留，只多出目标语言的那几个字符串表。</span>
          </div>
        </div>
      </div>

      <div class="cta">
        <!-- @ts-expect-error vscode-button 是自定义元素 -->
        <vscode-button icon="folder-opened" @click="$emit('open')">打开地图 / 模组</vscode-button>
        <span v-if="isTauri" class="dropHint">也可以把文件直接拖进来</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.welcome {
  flex: 1;
  display: grid;
  place-items: center;
  padding: var(--s8);
  overflow-y: auto;
  background: radial-gradient(900px 420px at 50% -10%, rgba(0, 144, 255, 0.08), transparent 70%), var(--bg-app);
  transition: background var(--t-base), outline-color var(--t-base);
  outline: 2px dashed transparent;
  outline-offset: -14px;
}

.welcome.dropping {
  background: radial-gradient(900px 420px at 50% -10%, rgba(0, 144, 255, 0.22), transparent 70%), var(--bg-app);
  outline-color: var(--accent);
}

.inner { max-width: 720px; }

h1 {
  font-size: 26px;
  font-weight: 600;
  letter-spacing: -0.02em;
  line-height: 1.35;
  margin-bottom: var(--s5);
}

h1 em { font-style: normal; color: var(--accent-text); }

.lead { color: var(--text-muted); font-size: var(--fs-md); line-height: 1.75; margin-bottom: var(--s8); }

.lead code {
  background: var(--bg-raised);
  border: 1px solid var(--border-subtle);
  border-radius: var(--r1);
  padding: 1px 5px;
  color: var(--text);
}

.steps { display: flex; flex-direction: column; gap: var(--s5); margin-bottom: var(--s8); }

.step { display: flex; gap: var(--s5); align-items: flex-start; }

.no {
  flex: 0 0 auto;
  display: grid;
  place-items: center;
  width: 24px;
  height: 24px;
  border-radius: 50%;
  background: var(--blue-4);
  color: var(--accent-text);
  font-size: var(--fs-sm);
  font-weight: 600;
}

.step b { display: block; font-weight: 600; margin-bottom: 2px; }
.step span { color: var(--text-muted); font-size: var(--fs-sm); line-height: 1.65; }

.cta { display: flex; align-items: center; gap: var(--s5); }
.dropHint { font-size: var(--fs-sm); color: var(--text-faint); }
</style>
