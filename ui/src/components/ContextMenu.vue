<script setup lang="ts">
// 右键菜单：直接用 VS Code 官方的 <vscode-context-menu>。
//
// 它吃一个 data 数组（label / keybinding / value / separator），选中时抛 vsc-menu-select。
// 我们这层只负责「把共享菜单状态翻译成它的 data，并把选中的 value 回传给业务」。

import { nextTick, ref, watch } from "vue";

import type { ContextMenuApi } from "../composables/useContextMenu";

const props = defineProps<{ menu: ContextMenuApi }>();

interface Node {
  label?: string;
  keybinding?: string;
  value?: string;
  separator?: boolean;
}

/** 自定义元素在 Vue 的类型系统里没有定义，这里用一个最小接口。 */
interface MenuElement extends HTMLElement {
  data: Node[];
  show: boolean;
}

const el = ref<MenuElement | null>(null);

watch(
  () => props.menu.menu.value,
  async (state) => {
    const node = el.value;
    if (!node) return;
    if (!state) {
      node.show = false;
      return;
    }
    node.data = state.items.map((item, index) =>
      item.separator && index > 0
        ? { separator: true }
        : { label: item.label, keybinding: item.hint, value: item.id },
    );
    node.style.left = state.x + "px";
    node.style.top = state.y + "px";
    await nextTick();
    node.show = true;
  },
);

function onSelect(event: Event) {
  const detail = (event as CustomEvent).detail as { value?: string } | string | undefined;
  const value = typeof detail === "string" ? detail : detail?.value;
  if (value) props.menu.pick(String(value));
  else props.menu.close();
}
</script>

<template>
  <!-- @ts-expect-error vscode-context-menu 是自定义元素 -->
  <vscode-context-menu ref="el" class="ctx" @vsc-context-menu-select="onSelect" />
</template>

<style scoped>
/* 组件自身是 position:absolute，这里改成 fixed，好让坐标直接用视口坐标。 */
.ctx {
  position: fixed;
  z-index: 90;
}
</style>
