<script setup lang="ts">
// 最左侧的竖条（VS Code 的活动栏）：切换左栏显示什么。
// 图标顺序 = 使用频率：资源管理器 -> 过滤 -> 语言。

import Icon from "./Icon.vue";

export type Activity = "explorer" | "filters" | "locales" | "glossary" | "scm";
const props = defineProps<{
  modelValue: Activity;
  leftOpen: boolean;
  /** 侧栏图标上的小徽标（版本管理的未提交数量）。 */
  badges?: Partial<Record<Activity, number>>;
}>();
const emit = defineEmits<{ (e: "update:modelValue", v: Activity): void }>();

const ITEMS: Array<{ id: Activity; icon: string; title: string }> = [
  { id: "explorer", icon: "files", title: "资源管理器（项目与文件）" },
  { id: "filters", icon: "filter", title: "过滤器（状态 / 字符串表）" },
  { id: "locales", icon: "globe", title: "语言覆盖" },
  { id: "glossary", icon: "book", title: "专有名词库（Ctrl+F 搜索）" },
  { id: "scm", icon: "git-branch", title: "版本管理" },
];

function badgeOf(id: Activity): number {
  const n = props.badges?.[id] ?? 0;
  return n > 0 ? n : 0;
}

function pick(id: Activity) {
  emit("update:modelValue", id);
}
</script>

<template>
  <nav class="activity" aria-label="侧栏切换">
    <button
      v-for="item in ITEMS"
      :key="item.id"
      class="item"
      :class="{ active: props.modelValue === item.id, dimmed: !props.leftOpen }"
      :title="item.title"
      @click="pick(item.id)"
    >
      <Icon :name="item.icon" :size="18" />
      <span v-if="badgeOf(item.id)" class="badge">{{ badgeOf(item.id) }}</span>
    </button>
  </nav>
</template>

<style scoped>
/* VS Code 的活动栏：整条 #181818，选中项靠左侧 2px 的 activeBorder 指示，
 * 与侧栏之间**没有**分隔线（两侧同色），靠区域本身区分。 */
.activity {
  flex: 0 0 var(--w-activity);
  width: var(--w-activity);
  min-width: var(--w-activity);
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0;
  background: var(--bg-activitybar);
}

.item {
  width: 100%;
  height: 48px;
  padding: 0;
  justify-content: center;
  background: transparent;
  border-color: transparent;
  color: var(--fg-activitybar-inactive);
  border-left: 2px solid transparent;
  border-radius: 0;
}
.item :deep(svg), .item :deep(.codicon) { font-size: 20px; }

.item:hover { color: var(--fg-activitybar); background: transparent; border-color: transparent; }
.item.active {
  color: var(--fg-activitybar);
  border-left-color: var(--activitybar-active-border);
  background: transparent;
}
.item.dimmed { opacity: 1; color: var(--fg-activitybar-inactive); }

.item { position: relative; }

.badge {
  position: absolute;
  right: 5px;
  bottom: 6px;
  min-width: 15px;
  padding: 0 4px;
  border-radius: 999px;
  background: var(--vscode-activityBarBadge-background, #0078d4);
  color: var(--vscode-activityBarBadge-foreground, #fff);
  font-size: 9px;
  line-height: 15px;
  text-align: center;
  font-variant-numeric: tabular-nums;
}
</style>
