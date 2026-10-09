// 右键菜单：一个共享实例，任何地方调 openMenu(event, items, onPick) 即可。
//
// 菜单项是纯数据，动作通过回调回来 —— 这样菜单组件本身不需要知道任何业务。

import { onBeforeUnmount, onMounted, ref } from "vue";

export interface MenuItem {
  id: string;
  label: string;
  /** 右侧的快捷键提示。 */
  hint?: string;
  disabled?: boolean;
  /** 危险动作（红字）。 */
  danger?: boolean;
  /** 在它之前画一条分隔线。 */
  separator?: boolean;
}

export interface MenuState {
  x: number;
  y: number;
  items: MenuItem[];
}

export function useContextMenu() {
  const menu = ref<MenuState | null>(null);
  let handler: ((id: string) => void) | null = null;

  function openMenu(event: MouseEvent, items: MenuItem[], onPick: (id: string) => void) {
    event.preventDefault();
    event.stopPropagation();
    // 贴着视口边缘时往回收
    const width = 240;
    const height = items.length * 26 + 12;
    const x = Math.min(event.clientX, window.innerWidth - width - 8);
    const y = Math.min(event.clientY, window.innerHeight - height - 8);
    handler = onPick;
    menu.value = { x: Math.max(8, x), y: Math.max(8, y), items };
  }

  function pick(id: string) {
    const fn = handler;
    const item = menu.value?.items.find((i) => i.id === id);
    close();
    // 禁用项点了不做事（VS Code 的菜单项本身没有 disabled 属性，这里兜住）
    if (item?.disabled) return;
    fn?.(id);
  }

  function close() {
    menu.value = null;
    handler = null;
  }

  function onGlobalPointer() {
    if (menu.value) close();
  }

  function onKey(event: KeyboardEvent) {
    if (event.key === "Escape" && menu.value) close();
  }

  onMounted(() => {
    window.addEventListener("pointerdown", onGlobalPointer);
    window.addEventListener("blur", onGlobalPointer);
    window.addEventListener("keydown", onKey);
  });
  onBeforeUnmount(() => {
    window.removeEventListener("pointerdown", onGlobalPointer);
    window.removeEventListener("blur", onGlobalPointer);
    window.removeEventListener("keydown", onKey);
  });

  return { menu, openMenu, pick, close };
}

export type ContextMenuApi = ReturnType<typeof useContextMenu>;
