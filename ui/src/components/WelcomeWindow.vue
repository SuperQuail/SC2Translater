<script setup lang="ts">
// 没打开项目时的**小窗**（照 JetBrains 的欢迎窗）。
//
// JetBrains 系就是这样：没有项目时开一个小窗，标题栏、左侧导航、右边搜索 + 动作 + 最近项目；
// 打开项目之后才换成主窗口。我们照做 —— 所以这一屏不是「主界面的一块」，而是独立的窗口形态。
//
// 标题栏是我们自己画的（IntelliJ 在 Windows 上也自己画），Tauri 那边配 decorations=false。

import { computed, ref } from "vue";

import { avatarColor, isTauri, projectInitials } from "../api";
import type { RecentEntry } from "../api/types";
import Icon from "./Icon.vue";

const props = defineProps<{ recents: RecentEntry[]; dropping?: boolean; version?: string }>();
const emit = defineEmits<{
  (e: "open"): void;
  (e: "create"): void;
  (e: "pick", path: string): void;
  (e: "forget", path: string): void;
  (e: "settings"): void;
  (e: "help"): void;
  (e: "minimize"): void;
  (e: "maximize"): void;
  (e: "close"): void;
}>();

const query = ref("");
const hovered = ref<string | null>(null);

const filtered = computed(() => {
  const q = query.value.trim().toLowerCase();
  if (!q) return props.recents;
  return props.recents.filter(
    (r) => r.name.toLowerCase().includes(q) || r.path.toLowerCase().includes(q),
  );
});

function missing(entry: RecentEntry): boolean {
  return entry.files < 0;
}

function metaOf(entry: RecentEntry): string {
  if (missing(entry)) return "路径已失效";
  return entry.files > 0 ? entry.files + " 个文件" : "";
}

function readValue(event: Event): string {
  return String((event.target as unknown as { value: string }).value ?? "");
}

function rowMenu(entry: RecentEntry) {
  // 简单起见：右键即从列表移除（和主界面里的一致），确认由用户自己负责
  if (window.confirm("把「" + entry.name + "」从最近列表里移除？")) emit("forget", entry.path);
}
</script>

<template>
  <div class="welcome-window" :class="{ dropping: props.dropping }">
    <!-- 自绘标题栏（IntelliJ 在 Windows 上也是自绘） -->
    <header class="chrome" data-tauri-drag-region>
      <span class="chrome-icon">SC2</span>
      <span class="chrome-title">欢迎访问 SC2Translater</span>
      <span class="chrome-spacer" data-tauri-drag-region></span>
      <template v-if="isTauri">
        <button class="win-btn" title="最小化" @click="emit('minimize')"><span class="glyph">—</span></button>
        <button class="win-btn" title="最大化" @click="emit('maximize')"><span class="glyph">▢</span></button>
        <button class="win-btn danger" title="关闭" @click="emit('close')"><span class="glyph">✕</span></button>
      </template>
    </header>

    <div class="body">
      <!-- 左侧导航 -->
      <nav class="nav">
        <div class="brand">
          <span class="logo">SC2</span>
          <span class="brand-text">
            <span class="brand-name">SC2Translater</span>
            <span class="brand-ver">{{ props.version ?? "0.1.0" }}</span>
          </span>
        </div>

        <button class="nav-item active">项目</button>
        <button class="nav-item" @click="emit('settings')">设置</button>
        <button class="nav-item" @click="emit('help')">快捷键</button>

        <span class="grow"></span>

        <button class="nav-item gear" title="设置" @click="emit('settings')">
          <Icon name="settings" :size="16" />
        </button>
      </nav>

      <!-- 右侧：搜索 + 动作 + 最近项目 -->
      <main class="content">
        <div class="toolbar">
          <!-- @ts-expect-error vscode-textfield 是自定义元素 -->
          <vscode-textfield
            class="search"
            :value="query"
            placeholder="搜索项目"
            @input="query = readValue($event)"
          >
            <!-- @ts-expect-error 自定义元素 -->
            <vscode-icon slot="content-before" name="search" :size="14" />
          </vscode-textfield>

          <!-- @ts-expect-error vscode-button 是自定义元素 -->
          <vscode-button icon="plus" @click="emit('create')">新建项目</vscode-button>
          <!-- @ts-expect-error 自定义元素 -->
          <vscode-button secondary icon="folder-opened" @click="emit('open')">打开</vscode-button>
        </div>

        <div class="list">
          <div
            v-for="entry in filtered"
            :key="entry.path"
            class="row"
            :class="{ dead: missing(entry), hovered: hovered === entry.path }"
            :title="entry.path"
            @click="emit('pick', entry.path)"
            @mouseenter="hovered = entry.path"
            @mouseleave="hovered = null"
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
            <button class="more" title="更多" @click.stop="rowMenu(entry)"><Icon name="more" :size="16" /></button>
          </div>

          <div v-if="!filtered.length" class="empty">
            <template v-if="query">没有匹配「{{ query }}」的项目。</template>
            <template v-else>
              还没有打开过任何项目。<br />
              点右上角的「打开」选一个放着 .SC2Map / .SC2Mod 的文件夹，或者把文件直接拖进来。
            </template>
          </div>
        </div>
      </main>
    </div>
  </div>
</template>

<style scoped>
.welcome-window {
  height: 100%;
  display: flex;
  flex-direction: column;
  background: var(--jb-bg);
  overflow: hidden;
}
.welcome-window.dropping { outline: 2px dashed var(--jb-main); outline-offset: -8px; }

/* ---- 自绘标题栏 ---- */
.chrome {
  flex: 0 0 auto;
  height: 34px;
  display: flex;
  align-items: center;
  gap: 8px;
  padding-left: 10px;
  background: var(--jb-bg);
}
.chrome-icon {
  width: 18px;
  height: 18px;
  border-radius: 3px;
  background: var(--jb-main);
  color: #fff;
  font-size: 8px;
  font-weight: 700;
  display: grid;
  place-items: center;
}
.chrome-title { font-size: 13px; color: var(--jb-text); }
.chrome-spacer { flex: 1; height: 100%; }
.win-btn {
  width: 44px;
  height: 34px;
  padding: 0;
  justify-content: center;
  border: none;
  border-radius: 0;
  background: transparent;
  color: var(--jb-secondary);
}
.win-btn:hover { background: var(--jb-hover); color: var(--jb-text); }
.win-btn.danger:hover { background: #c42b1c; color: #fff; }
.glyph { font-size: 12px; line-height: 1; }

/* ---- 主体：左导航 + 右内容 ---- */
.body { flex: 1; min-height: 0; display: flex; }

.nav {
  flex: 0 0 250px;
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 8px 12px 12px;
  border-right: 1px solid var(--jb-line);
}

.brand { display: flex; align-items: center; gap: 12px; padding: 8px 8px 20px; }
.logo {
  width: 36px; height: 36px; border-radius: 6px;
  background: var(--jb-main); color: #fff;
  display: grid; place-items: center;
  font-size: 12px; font-weight: 700;
}
.brand-text { display: flex; flex-direction: column; }
.brand-name { font-size: 14px; font-weight: 600; color: var(--jb-text); }
.brand-ver { font-size: 12px; color: var(--jb-secondary); }

.nav-item {
  display: flex;
  align-items: center;
  gap: 10px;
  height: 34px;
  padding: 0 12px;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: var(--jb-text);
  font-size: 14px;
  text-align: left;
  justify-content: flex-start;
  cursor: default;
}
.nav-item:hover { background: var(--jb-hover); }
.nav-item.active { background: var(--jb-selected); font-weight: 500; }
.grow { flex: 1; }
.gear { width: 34px; padding: 0; justify-content: center; color: var(--jb-secondary); }

.content { flex: 1; min-width: 0; display: flex; flex-direction: column; }

.toolbar {
  flex: 0 0 auto;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 16px 20px 8px;
}
.search { flex: 1; max-width: 320px; }

.list { flex: 1; min-height: 0; overflow-y: auto; padding: 4px 20px 20px; }

.row {
  display: flex;
  align-items: center;
  gap: 12px;
  width: 100%;
  padding: 10px 12px;
  border: none;
  border-radius: 6px;
  background: transparent;
  text-align: left;
  cursor: default;
}
.row.hovered { background: var(--jb-selected); }
.row.dead { opacity: 0.55; }

.avatar {
  flex: 0 0 auto;
  width: 36px; height: 36px;
  border-radius: 6px;
  display: grid; place-items: center;
  color: #fff; font-size: 13px; font-weight: 600;
}
.col { min-width: 0; flex: 1; display: block; }
.name { display: block; font-size: 15px; font-weight: 500; color: var(--jb-text); }
.path {
  display: block; font-size: 12px; color: var(--jb-secondary);
  overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
}
.meta { display: flex; align-items: center; gap: 6px; margin-top: 2px; font-size: 12px; color: var(--jb-hint); }

.more {
  flex: 0 0 auto;
  width: 26px; height: 26px; padding: 0;
  justify-content: center;
  background: transparent;
  color: var(--jb-secondary);
  opacity: 0;
}
.row.hovered .more { opacity: 1; }
.more:hover { background: var(--jb-hover); color: var(--jb-text); }

.empty { padding: 24px 12px; font-size: 13px; line-height: 1.8; color: var(--jb-hint); }
</style>
