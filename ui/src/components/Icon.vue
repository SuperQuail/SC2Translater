<script setup lang="ts">
// 图标：直接用 VS Code 官方的 <vscode-icon>（Codicons 图标字体）。
// 这里只做一层名字映射 —— 业务里叫「map / save / refresh」，组件里不必知道 codicon 的具体 id。

const props = withDefaults(defineProps<{ name: string; size?: number; spin?: boolean }>(), {
  size: 16,
  spin: false,
});

/** 业务名 -> Codicon id。没登记的按同名直传。 */
const CODIcon: Record<string, string> = {
  files: "files",
  map: "map",
  file: "file",
  folder: "folder",
  "folder-open": "folder-opened",
  chevron: "chevron-right",
  "chevron-down": "chevron-down",
  close: "close",
  plus: "add",
  save: "save",
  open: "folder-opened",
  new: "new-file",
  refresh: "refresh",
  trash: "trash",
  more: "ellipsis",
  search: "search",
  filter: "filter",
  list: "list-flat",
  text: "symbol-string",
  code: "code",
  kv: "symbol-string",
  json: "json",
  globe: "globe",
  layers: "layers",
  check: "check",
  warn: "warning",
  info: "info",
  help: "question",
  translate: "globe",
  wand: "wand",
  shield: "shield",
  export: "export",
  import: "arrow-down",
  "panel-left": "layout-sidebar-left",
  "panel-right": "layout-sidebar-right",
  play: "play",
  settings: "settings-gear",
  error: "error",
  lightbulb: "lightbulb",
};

const id = () => CODIcon[props.name] ?? props.name;
</script>

<template>
  <!-- @ts-expect-error vscode-icon 是自定义元素，Vue 类型系统里没有 -->
  <vscode-icon :name="id()" :size="props.size" :spin="props.spin" />
</template>
