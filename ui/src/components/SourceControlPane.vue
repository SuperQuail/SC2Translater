<script setup lang="ts">
// 版本管理面板（左栏之一）。
//
// 交互按 JetBrains 那套来：勾选式、对话框式，**不需要敲命令**；
// 但每个动作下面都给出「等价命令行」，习惯 CLI 的人可以复制走 —— 两条路都留着。

import { computed } from "vue";

import type { ContextMenuApi } from "../composables/useContextMenu";
import type { VcsApi } from "../composables/useVcs";
import Icon from "./Icon.vue";

const props = defineProps<{ vcs: VcsApi; menu: ContextMenuApi; document: string | null }>();
const v = props.vcs;

const toolReady = computed(() => Boolean(v.tool.value?.installed));
const branch = computed(() => v.status.value?.branch || "main");

function copyCommand() {
  if (v.lastCommand.value) void navigator.clipboard.writeText(v.lastCommand.value);
}

function branchMenu(event: MouseEvent) {
  const items = [
    ...v.refs.value.branches.map((b) => ({
      id: "checkout:" + b,
      label: "切换到 " + b,
      disabled: b === branch.value,
    })),
    { id: "new-branch", label: "新建分支…", separator: true },
    { id: "new-tag", label: "在当前提交打标签…" },
  ];
  props.menu.openMenu(event, items, (id) => {
    if (id.startsWith("checkout:")) void v.refAction("checkout", id.slice("checkout:".length));
    else if (id === "new-branch") {
      const name = window.prompt("新分支名", "zhcn-wip");
      if (name) void v.refAction("branch", name);
    } else if (id === "new-tag") {
      const name = window.prompt("标签名", "v0.1");
      if (name) void v.refAction("tag", name);
    }
  });
}

function changeMenu(event: MouseEvent, name: string) {
  props.menu.openMenu(
    event,
    [
      { id: "copy", label: "复制组件名" },
      { id: "reveal", label: "打开仓库目录", separator: true },
    ],
    (id) => {
      if (id === "copy") void navigator.clipboard.writeText(name);
      else void v.reveal();
    },
  );
}
</script>

<template>
  <div class="scm">
    <header class="section">
      <span class="title">版本管理</span>
      <!-- @ts-expect-error vscode-toolbar-container 是自定义元素 -->
      <vscode-toolbar-container class="actions">
        <!-- @ts-expect-error 自定义元素 -->
        <vscode-toolbar-button icon="refresh" label="刷新" @click="v.refresh"></vscode-toolbar-button>
        <!-- @ts-expect-error 自定义元素 -->
        <vscode-toolbar-button icon="folder-opened" label="打开仓库目录" @click="v.reveal"></vscode-toolbar-button>
        <!-- @ts-expect-error 自定义元素 -->
        <vscode-toolbar-button icon="cloud-download" label="重新安装内置 SC2Diff" @click="v.ensureTool(true)"></vscode-toolbar-button>
      </vscode-toolbar-container>
    </header>

    <div v-if="!v.ready.value" class="notice">
      先打开一个文件（并在项目里保存一次），版本管理才知道要跟踪谁。
    </div>

    <template v-else>
      <div v-if="!toolReady" class="notice err">
        {{ v.tool.value?.message ?? "内置的 SC2Diff 还没就位" }}
        <!-- @ts-expect-error 自定义元素 -->
        <vscode-button secondary @click="v.ensureTool(true)">重新安装</vscode-button>
      </div>

      <div v-else-if="!v.isRepo.value" class="notice">
        <div>这个文件还没有版本仓库。</div>
        <div class="dim">建仓会把当前文档导入为基线提交，之后每次改动都能回退。</div>
        <!-- @ts-expect-error 自定义元素 -->
        <vscode-button @click="v.initRepo">初始化仓库</vscode-button>
      </div>

      <template v-else>
        <div class="branchRow" @contextmenu="branchMenu">
          <Icon name="git-branch" :size="13" />
          <span class="branch">{{ branch }}</span>
          <span class="count" :class="{ zero: !v.changeCount.value }">{{ v.changeCount.value }}</span>
          <button class="pick" title="切换 / 新建分支" @click="branchMenu">▾</button>
        </div>

        <div class="refs" v-if="v.refs.value.tags.length">
          <Icon name="tag" :size="12" />
          <span class="mono">{{ v.refs.value.tags.join("  ") }}</span>
        </div>

        <div class="commitBox">
          <!-- @ts-expect-error vscode-textarea 是自定义元素 -->
          <vscode-textarea
            class="msg"
            :value="v.message.value"
            rows="3"
            resize="vertical"
            placeholder="提交信息（例如：汉化 zhCN，补全 677 条）"
            @input="v.message.value = String(($event.target as unknown as { value: string }).value)"
          ></vscode-textarea>
          <div class="commitRow">
            <!-- @ts-expect-error 自定义元素 -->
            <vscode-button :disabled="v.busy.value" @click="v.commit">
              <Icon name="check" :size="13" />提交 {{ v.changeCount.value }} 项改动
            </vscode-button>
            <!-- @ts-expect-error 自定义元素 -->
            <vscode-button secondary :disabled="v.busy.value" @click="v.syncWorkingTree">同步工作树</vscode-button>
            <!-- @ts-expect-error 自定义元素 -->
            <vscode-button secondary :disabled="v.busy.value" @click="v.showWorkingDiff">工作树差异</vscode-button>
          </div>
        </div>

        <div class="listHead">
          <span>改动</span>
          <span class="dim">{{ v.status.value?.staged ?? 0 }} 暂存 / {{ v.status.value?.dirty ?? 0 }} 未暂存 / {{ v.status.value?.untracked ?? 0 }} 未跟踪</span>
        </div>
        <div class="list changes">
          <div
            v-for="c in v.status.value?.changes ?? []"
            :key="c.name"
            class="change"
            @contextmenu="changeMenu($event, c.name)"
          >
            <span class="kind" :class="c.index === '?' ? 'new' : c.index !== ' ' ? 'staged' : 'dirty'">
              {{ c.index === "?" ? "未跟踪" : c.index !== " " ? "已暂存" : "已修改" }}
            </span>
            <span class="name mono" :title="c.name">{{ c.name }}</span>
          </div>
          <div v-if="!(v.status.value?.changes ?? []).length" class="empty">工作树干净。</div>
        </div>

        <div class="listHead">
          <span>历史</span>
          <!-- @ts-expect-error 自定义元素 -->
          <vscode-button secondary icon="export" @click="v.pack">导出文档</vscode-button>
        </div>
        <div class="list history">
          <div
            v-for="c in v.commits.value"
            :key="c.sha"
            class="commit"
            :class="{ active: v.selectedCommit.value === c.sha }"
            :title="c.sha"
            @click="v.showDiff(c.sha)"
          >
            <span class="sha mono">{{ c.short }}</span>
            <span class="msg" :title="c.message">{{ c.message }}</span>
          </div>
          <div v-if="!v.commits.value.length" class="empty">还没有提交。</div>
        </div>

        <div v-if="v.diffText.value" class="diffBox">
          <div class="diffHead">
            <span>{{ v.selectedCommit.value ? "提交差异" : "工作树差异" }}</span>
            <button class="mini" @click="v.diffText.value = ''">收起</button>
          </div>
          <pre class="diff">{{ v.diffText.value }}</pre>
        </div>

        <div v-if="v.lastCommand.value" class="cmdBox">
          <div class="cmdHead">
            <span>等价命令行</span>
            <button class="mini" @click="copyCommand">复制</button>
          </div>
          <code class="cmd">{{ v.lastCommand.value }}</code>
        </div>

        <div v-if="v.log.value.length" class="log">
          <div v-for="(line, i) in v.log.value.slice(0, 8)" :key="i">{{ line }}</div>
        </div>
      </template>
    </template>
  </div>
</template>

<style scoped>
.scm { display: flex; flex-direction: column; height: 100%; overflow-y: auto; background: var(--bg-sidebar); }

.section {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 24px;
  padding: 0 var(--s2) 0 var(--s5);
  flex: 0 0 auto;
  position: sticky;
  top: 0;
  background: var(--bg-sidebar);
  z-index: 2;
}
.title { font-size: var(--fs-xs); font-weight: 700; letter-spacing: 0.04em; color: var(--text-muted); text-transform: uppercase; }
.actions { display: flex; }

.notice {
  margin: var(--s3) var(--s4);
  padding: var(--s4);
  border: 1px solid var(--border-subtle);
  border-radius: var(--r2);
  background: var(--bg-editor);
  font-size: var(--fs-sm);
  color: var(--text-muted);
  display: flex;
  flex-direction: column;
  gap: var(--s3);
  align-items: flex-start;
}
.notice.err { border-color: var(--err); color: var(--err-text); }
.dim { color: var(--text-faint); font-size: var(--fs-xs); line-height: 1.6; }

.branchRow {
  display: flex;
  align-items: center;
  gap: var(--s3);
  padding: var(--s3) var(--s5);
  font-size: var(--fs-sm);
  color: var(--text);
}
.branch { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.count {
  font-size: 10px; padding: 0 5px; border-radius: 999px;
  background: var(--badge-background, #616161); color: var(--badge-foreground, #fff);
}
.count.zero { background: transparent; color: var(--text-faint); }
.pick { width: 18px; height: 18px; padding: 0; justify-content: center; background: transparent; color: var(--text-faint); }
.pick:hover { background: var(--bg-hover); color: var(--text); }

.refs { display: flex; align-items: center; gap: var(--s3); padding: 0 var(--s5) var(--s3); font-size: var(--fs-xs); color: var(--text-faint); }

.commitBox { padding: var(--s3) var(--s4); display: flex; flex-direction: column; gap: var(--s3); }
.msg { width: 100%; }
.commitRow { display: flex; gap: var(--s3); flex-wrap: wrap; }

.listHead {
  display: flex; align-items: center; justify-content: space-between;
  padding: var(--s3) var(--s5) var(--s2);
  font-size: var(--fs-xs); font-weight: 600; color: var(--text-muted);
  text-transform: uppercase; letter-spacing: 0.04em;
}
.listHead .dim { text-transform: none; letter-spacing: 0; font-weight: 400; color: var(--text-faint); }

.list { display: flex; flex-direction: column; padding: 0 var(--s3); }
.changes { max-height: 200px; overflow-y: auto; }
.history { max-height: 220px; overflow-y: auto; }

.change { display: flex; align-items: center; gap: var(--s3); padding: 2px var(--s3); border-radius: var(--r1); font-size: var(--fs-xs); }
.change:hover { background: var(--list-hover); }
.kind { flex: 0 0 auto; padding: 0 4px; border-radius: var(--r1); font-size: 10px; background: var(--slate-7); color: var(--text); }
.kind.staged { background: var(--accent); color: #fff; }
.kind.dirty { background: var(--warn); color: #241a05; }
.kind.new { background: var(--ok); color: #04160d; }
.name { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

.commit { display: flex; align-items: center; gap: var(--s3); padding: 2px var(--s3); border-radius: var(--r1); font-size: var(--fs-xs); cursor: default; }
.commit:hover { background: var(--list-hover); }
.commit.active { background: var(--list-active-bg); color: var(--list-active-fg); }
.sha { color: var(--text-faint); }
.commit .msg { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

.empty { padding: var(--s4); font-size: var(--fs-xs); color: var(--text-faint); }

.diffBox { margin: var(--s3) var(--s4); border: 1px solid var(--border-subtle); border-radius: var(--r2); overflow: hidden; }
.diffHead, .cmdHead { display: flex; align-items: center; justify-content: space-between; padding: 2px var(--s3); background: var(--bg-editor); font-size: var(--fs-xs); color: var(--text-muted); }
.mini { height: 18px; padding: 0 var(--s3); font-size: var(--fs-xs); background: transparent; color: var(--accent-text); }
.diff {
  margin: 0; padding: var(--s3); max-height: 240px; overflow: auto;
  background: var(--bg-inset); font-family: var(--font-mono); font-size: var(--fs-xs);
  white-space: pre-wrap; color: var(--text-muted);
}

.cmdBox { margin: var(--s3) var(--s4); border: 1px solid var(--border-subtle); border-radius: var(--r2); overflow: hidden; }
.cmd { display: block; padding: var(--s3); background: var(--bg-inset); font-size: 11px; color: var(--text-faint); word-break: break-all; }

.log { padding: var(--s3) var(--s5) var(--s6); font-size: 10px; color: var(--text-faint); display: flex; flex-direction: column; gap: 2px; }
</style>
