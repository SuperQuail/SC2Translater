<script setup lang="ts">
// 快捷键与右键菜单速查（F1）。

defineEmits<{ (e: "close"): void }>();

const GROUPS: Array<{ title: string; rows: Array<[string, string]> }> = [
  {
    title: "项目",
    rows: [
      ["Ctrl+N", "新建项目"],
      ["Ctrl+O", "打开项目"],
      ["Ctrl+S", "保存项目"],
      ["Ctrl+Shift+S", "项目另存为"],
      ["Ctrl+Shift+O", "添加文件到项目"],
    ],
  },
  {
    title: "文件切换",
    rows: [
      ["Ctrl+Tab", "下一个文件"],
      ["Ctrl+Shift+Tab", "上一个文件"],
      ["Ctrl+1 … Ctrl+9", "直接跳到第 N 个文件"],
      ["Ctrl+W", "从项目中移出当前文件"],
    ],
  },
  {
    title: "翻译",
    rows: [
      ["Ctrl+F", "搜索键 / 原文 / 译文"],
      ["↑ / ↓", "上一条 / 下一条"],
      ["Enter", "把焦点移到译文输入框"],
      ["Ctrl+D", "把原文填入当前条译文"],
      ["Ctrl+Shift+D", "把原文填入所有空译文"],
    ],
  },
  {
    title: "输出",
    rows: [
      ["Ctrl+E", "导出翻译键 JSON"],
      ["Ctrl+Enter", "写回地图"],
      ["Ctrl+Shift+F", "补全默认语言"],
      ["F5", "容器自检"],
    ],
  },
  {
    title: "视图",
    rows: [
      ["Ctrl+B", "收起 / 展开左栏"],
      ["Ctrl+J", "收起 / 展开右栏"],
      ["Ctrl+1…9 / Ctrl+Shift+1…9", "（已被文件切换占用）"],
      ["F1", "本帮助"],
      ["Esc", "关闭弹层 / 退出输入框"],
    ],
  },
];

const MOUSE: Array<[string, string]> = [
  ["右键文件标签", "重命名 / 重新扫描 / 定位 / 移出项目"],
  ["右键键表行", "用原文填入 / 清空 / 复制键 / 复制原文 / 只显示该字符串表 / 只显示该键前缀"],
  ["右键左栏状态或字符串表", "只显示这一类"],
  ["双击文件标签", "重命名"],
  ["拖动分隔条", "调整左右栏宽度"],
];
</script>

<template>
  <div class="overlay" @click.self="$emit('close')">
    <div class="dialog">
      <header>
        <h2>快捷键与右键菜单</h2>
        <!-- @ts-expect-error vscode-button 是自定义元素 -->
        <vscode-button secondary icon-only icon="close" title="关闭" @click="$emit('close')"></vscode-button>
      </header>

      <div class="cols">
        <section v-for="g in GROUPS" :key="g.title">
          <h3>{{ g.title }}</h3>
          <div v-for="([key, desc], i) in g.rows" :key="i" class="row">
            <kbd>{{ key }}</kbd>
            <span>{{ desc }}</span>
          </div>
        </section>
      </div>

      <section class="mouse">
        <h3>鼠标</h3>
        <div v-for="([what, desc], i) in MOUSE" :key="i" class="row">
          <b>{{ what }}</b>
          <span>{{ desc }}</span>
        </div>
      </section>
    </div>
  </div>
</template>

<style scoped>
.overlay {
  position: fixed;
  inset: 0;
  background: rgba(6, 8, 12, 0.62);
  display: grid;
  place-items: center;
  z-index: 70;
}

.dialog {
  width: 720px;
  max-height: 84vh;
  overflow-y: auto;
  background: var(--bg-panel);
  border: 1px solid var(--border);
  border-radius: var(--r4);
  box-shadow: var(--shadow-pop);
  padding: var(--s6);
  display: flex;
  flex-direction: column;
  gap: var(--s5);
}

header { display: flex; align-items: center; justify-content: space-between; }
header h2 { font-size: var(--fs-lg); }
.close { height: 26px; width: 26px; justify-content: center; padding: 0; font-size: 16px; }

.cols { display: grid; grid-template-columns: 1fr 1fr; gap: var(--s5) var(--s7); }

section { display: flex; flex-direction: column; gap: var(--s2); }
section h3 { font-size: var(--fs-xs); margin-bottom: var(--s1); }

.row { display: flex; align-items: baseline; gap: var(--s4); font-size: var(--fs-sm); }
.row span { color: var(--text-muted); }

kbd {
  flex: 0 0 auto;
  min-width: 132px;
  padding: 1px var(--s3);
  border-radius: var(--r1);
  border: 1px solid var(--border);
  background: var(--bg-inset);
  font-family: var(--font-mono);
  font-size: var(--fs-xs);
  color: var(--text);
}

.mouse { border-top: 1px solid var(--border-subtle); padding-top: var(--s5); }
.mouse b { flex: 0 0 auto; min-width: 132px; font-weight: 500; color: var(--text); font-size: var(--fs-sm); }
</style>
