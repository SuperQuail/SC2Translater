<script setup lang="ts">
// 项目工具窗：新建 / 打开 + 打开项目 + 最近的项目。
//
// 排布照 JetBrains：停靠在左栏的一个工具窗里（未打开任何东西时它就是整个左栏，
// 打开了工作区之后它作为「项目」这一项继续在）。
// 颜色全部走 --jb-* 变量，两种皮肤下都像长在里面的。

import { avatarColor, projectInitials } from "../api";
import type { RecentEntry } from "../api/types";
import Icon from "./Icon.vue";

const props = defineProps<{
  recents: RecentEntry[];
  current?: { name: string; path: string } | null;
}>();
const emit = defineEmits<{
  (e: "open"): void;
  (e: "create"): void;
  (e: "pick", path: string): void;
  (e: "forget", path: string): void;
  (e: "menu", event: MouseEvent): void;
  (e: "collapse"): void;
}>();

/** 路径失效的（盘没挂、文件夹删了）标灰但留着 —— 外接盘拔了不等于用户不要了。 */
function missing(entry: RecentEntry): boolean {
  return entry.files < 0;
}

function metaOf(entry: RecentEntry): string {
  if (missing(entry)) return "路径已失效";
  return entry.files > 0 ? entry.files + " 个文件" : "";
}
</script>

<template>
  <div class="project-pane">
    <header class="head">
      <span class="title">项目</span>
      <span class="head-actions">
        <button class="icon" title="更多" @click="emit('menu', $event)"><Icon name="more" :size="14" /></button>
        <button class="icon" title="收起" @click="emit('collapse')"><Icon name="close" :size="14" /></button>
      </span>
    </header>

    <div class="scroll">
      <div class="actions">
        <button class="action primary" @click="emit('create')">
          <Icon name="plus" :size="16" />新建项目…
        </button>
        <button class="action" @click="emit('open')">
          <Icon name="folder-open" :size="16" />打开…
        </button>
      </div>

      <template v-if="props.current">
        <div class="group">打开项目</div>
        <button class="row" :title="props.current.path" @click="emit('pick', props.current.path)">
          <span class="avatar" :style="{ background: avatarColor(props.current.path) }">
            {{ projectInitials(props.current.name) }}
          </span>
          <span class="col">
            <span class="name">{{ props.current.name }}</span>
            <span class="path">{{ props.current.path }}</span>
          </span>
        </button>
      </template>

      <div class="group">最近的项目</div>
      <button
        v-for="entry in props.recents"
        :key="entry.path"
        class="row"
        :class="{ dead: missing(entry) }"
        :title="entry.path"
        @click="emit('pick', entry.path)"
        @contextmenu.prevent="emit('forget', entry.path)"
      >
        <span class="avatar" :style="{ background: avatarColor(entry.path) }">
          {{ projectInitials(entry.name) }}
        </span>
        <span class="col">
          <span class="name">{{ entry.name }}</span>
          <span class="path">{{ entry.path }}</span>
          <span v-if="metaOf(entry)" class="meta">
            <Icon v-if="missing(entry)" name="warn" :size="12" />{{ metaOf(entry) }}
          </span>
        </span>
      </button>

      <div v-if="!props.recents.length" class="empty">
        还没有打开过任何文件夹。点上面的「打开…」选一个放着 .SC2Map / .SC2Mod 的目录。
      </div>
    </div>
  </div>
</template>

<style scoped>
.project-pane {
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
  background: var(--bg-sidebar);
}

/* 工具窗头：标题 + 两个图标按钮，照 JB 的工具窗 */
.head {
  flex: 0 0 auto;
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 32px;
  padding: 0 var(--s2) 0 var(--s5);
}
.title { font-size: var(--fs-xs); font-weight: 700; letter-spacing: 0.04em; color: var(--text-muted); text-transform: uppercase; }
.head-actions { display: flex; }
.icon {
  width: 22px;
  height: 22px;
  padding: 0;
  justify-content: center;
  background: transparent;
  color: var(--text-muted);
}
.icon:hover { background: var(--list-hover); color: var(--text); }

.scroll { flex: 1; min-height: 0; overflow-y: auto; padding: 4px 8px 16px; }

/* 全局 button 规则给了固定 26px 高，会把这里的行压扁，先重置掉 */
.action,
.row {
  display: flex;
  align-items: center;
  justify-content: flex-start;
  height: auto;
  border: none;
  background: transparent;
  color: inherit;
  font: inherit;
  white-space: normal;
  cursor: default;
}

.actions { display: flex; flex-direction: column; gap: 2px; padding: 0 2px; }

.action {
  gap: 10px;
  height: 34px;
  padding: 0 8px;
  border-radius: var(--jb-radius);
  color: var(--jb-text);
  font-size: 14px;
}
.action:hover { background: var(--jb-hover); }
.action.primary { background: var(--jb-main); color: #fff; font-weight: 500; }
.action.primary:hover { background: var(--jb-main-hover); }

.group { font-size: 12px; font-weight: 600; color: var(--jb-secondary); margin: 16px 0 4px 8px; }

.row {
  gap: 10px;
  width: 100%;
  padding: 8px;
  border-radius: 6px;
  text-align: left;
}
.row:hover { background: var(--jb-hover); }
.row.dead { opacity: 0.55; }

.avatar {
  flex: 0 0 auto;
  width: 34px;
  height: 34px;
  border-radius: 6px;
  display: grid;
  place-items: center;
  color: #fff;
  font-size: 12px;
  font-weight: 600;
}

.col { min-width: 0; flex: 1; display: block; }
.name { display: block; font-size: 14px; font-weight: 500; color: var(--jb-text); }
.path {
  display: block;
  font-size: 12px;
  color: var(--jb-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.meta { display: flex; align-items: center; gap: 6px; margin-top: 2px; font-size: 12px; color: var(--jb-hint); }

.empty { padding: 14px 8px; font-size: 13px; line-height: 1.6; color: var(--jb-hint); }
</style>
