<script setup lang="ts">
// 开始页：照 JetBrains 的 Welcome（新建 / 打开 + 最近的项目），三版排布可选。
//
// 颜色全部走 --jb-* 变量：VS Code 皮肤下指向本应用的令牌，JB 皮肤下换成 Ring UI 的真实色值。
// 所以同一个组件在两种皮肤下都会「像是长在里面的」。

import { computed } from "vue";

import { avatarColor, projectInitials } from "../api";
import type { RecentEntry } from "../api/types";
import Icon from "./Icon.vue";

const props = withDefaults(
  defineProps<{
    variant?: 1 | 2 | 3;
    recents: RecentEntry[];
    current?: { name: string; path: string } | null;
    dropping?: boolean;
  }>(),
  { variant: 1, current: null, dropping: false },
);

const emit = defineEmits<{
  (e: "open"): void;
  (e: "create"): void;
  (e: "pick", path: string): void;
  (e: "forget", path: string): void;
}>();

const variant = computed(() => props.variant);

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
  <div class="jb-welcome" :class="['v' + variant, { dropping: props.dropping }]">
    <div class="wrap">
      <!-- 左栏：动作 + 当前项目（V3 时是独立的边栏） -->
      <div class="rail">
        <div class="actions">
          <button class="action primary" @click="emit('create')">
            <Icon name="plus" :size="16" />新建项目…
          </button>
          <button class="action" @click="emit('open')">
            <Icon name="folder-open" :size="16" />打开…
          </button>
        </div>

        <template v-if="variant === 3 && props.current">
          <div class="group">打开项目</div>
          <button class="row" @click="emit('pick', props.current.path)">
            <span class="avatar sm" :style="{ background: avatarColor(props.current.path) }">
              {{ projectInitials(props.current.name) }}
            </span>
            <span class="col">
              <span class="name sm">{{ props.current.name }}</span>
              <span class="path">{{ props.current.path }}</span>
            </span>
          </button>
        </template>
      </div>

      <!-- 右栏：最近的项目 -->
      <div class="list">
        <template v-if="variant !== 3">
          <template v-if="props.current">
            <div class="group">打开项目</div>
            <button class="row" @click="emit('pick', props.current.path)">
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
        </template>

        <template v-else>
          <div class="group">最近的项目</div>
          <div class="grid">
            <button
              v-for="entry in props.recents"
              :key="entry.path"
              class="cell"
              :class="{ dead: missing(entry) }"
              :title="entry.path"
              @click="emit('pick', entry.path)"
            >
              <span class="avatar sm" :style="{ background: avatarColor(entry.path) }">
                {{ projectInitials(entry.name) }}
              </span>
              <span class="col">
                <span class="name sm">{{ entry.name }}</span>
                <span class="path">{{ entry.path }}</span>
              </span>
            </button>
          </div>
          <div v-if="!props.recents.length" class="empty">还没有打开过任何文件夹。</div>
        </template>
      </div>
    </div>
  </div>
</template>

<style scoped>
.jb-welcome {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  background: var(--jb-bg);
  transition: outline-color var(--t-base);
  outline: 2px dashed transparent;
  outline-offset: -12px;
}
.jb-welcome.dropping { outline-color: var(--jb-main); }

/* 全局 button 规则（base.css）给了固定 26px 高，会把这里的行压扁，先重置掉。 */
.action,
.row,
.cell {
  display: flex;
  align-items: center;
  justify-content: flex-start;
  height: auto;
  min-height: 0;
  border: none;
  background: transparent;
  color: inherit;
  font: inherit;
  white-space: normal;
  cursor: default;
}

.actions { display: flex; flex-direction: column; gap: 2px; }

.action {
  gap: 10px;
  height: 36px;
  padding: 0 10px;
  border-radius: var(--jb-radius);
  color: var(--jb-text);
  font-size: 14px;
}
.action:hover { background: var(--jb-hover); }
.action.primary { background: var(--jb-main); color: #fff; font-weight: 500; }
.action.primary:hover { background: var(--jb-main-hover); }

.group { font-size: 12px; font-weight: 600; color: var(--jb-secondary); margin: 18px 0 6px 10px; }

.row {
  gap: 12px;
  width: 100%;
  padding: 10px;
  border-radius: 6px;
  text-align: left;
}
.row:hover { background: var(--jb-hover); }
.row.dead { opacity: 0.55; }

.avatar {
  flex: 0 0 auto;
  width: 36px;
  height: 36px;
  border-radius: 6px;
  display: grid;
  place-items: center;
  color: #fff;
  font-size: 13px;
  font-weight: 600;
}
.avatar.sm { width: 32px; height: 32px; font-size: 12px; border-radius: 5px; }

.col { min-width: 0; flex: 1; display: block; }
.name { display: block; font-size: 15px; font-weight: 500; color: var(--jb-text); }
.name.sm { font-size: 14px; }
.path {
  display: block;
  font-size: 12px;
  color: var(--jb-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.meta { display: flex; align-items: center; gap: 6px; margin-top: 2px; font-size: 12px; color: var(--jb-hint); }

.empty { padding: 16px 10px; font-size: 13px; color: var(--jb-hint); }

/* ---- V1：照原图，左对齐单栏，内容列 1040 封顶 ---- */
.v1 .wrap { max-width: 1040px; padding: 20px 24px 40px; }
.v1 .rail { margin-bottom: 2px; }

/* ---- V2：居中卡片，动作并排在顶部 ---- */
.v2 { display: flex; justify-content: center; }
.v2 .wrap { width: 760px; max-width: 100%; padding: 40px 24px; }
.v2 .actions { flex-direction: row; gap: 10px; justify-content: center; margin-bottom: 8px; }
.v2 .action { border: 1px solid var(--jb-line); padding: 0 18px; height: 38px; justify-content: center; }
.v2 .action.primary { border-color: var(--jb-main); }
.v2 .row { padding: 8px 12px; }
.v2 .row:hover { background: var(--jb-selected); }
.v2 .group:first-of-type { margin-top: 20px; }

/* ---- V3：左右两栏 ---- */
.v3 .wrap { display: flex; height: 100%; }
.v3 .rail {
  flex: 0 0 340px;
  padding: 20px 16px;
  border-right: 1px solid var(--jb-line);
}
.v3 .list { flex: 1; min-width: 0; padding: 20px 20px 40px; overflow-y: auto; }
.v3 .group:first-child { margin-top: 0; }
.v3 .grid { display: grid; grid-template-columns: 1fr 1fr; gap: 2px 8px; }
.v3 .cell {
  gap: 10px;
  padding: 8px 10px;
  border-radius: 6px;
  text-align: left;
  min-width: 0;
}
.v3 .cell:hover { background: var(--jb-hover); }
.v3 .cell.dead { opacity: 0.55; }
</style>
