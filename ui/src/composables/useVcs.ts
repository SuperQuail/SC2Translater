// 版本管理的状态与动作。
//
// 交互按 JetBrains 那套来：**非命令式** —— 勾选变更、填提交信息、点按钮；
// 分支/标签用对话框，而不是让人敲命令。但每个动作都同时给出**等价命令行**，
// 习惯 CLI 的人可以原样复制（命令式操作也照样兼容）。

import { computed, ref } from "vue";

import { api } from "../api";
import type { ToolStatus, VcsCommit, VcsRefs, VcsStatus } from "../api/types";

export function useVcs(getProjectFile: () => string | null, getDocument: () => string | null) {
  const tool = ref<ToolStatus | null>(null);
  const repo = ref("");
  const status = ref<VcsStatus | null>(null);
  const commits = ref<VcsCommit[]>([]);
  const refs = ref<VcsRefs>({ branches: [], tags: [] });
  const diffText = ref("");
  const selectedCommit = ref<string | null>(null);
  const message = ref("");
  const busy = ref(false);
  const log = ref<string[]>([]);
  const error = ref("");
  /** 最近一次动作的等价命令行 —— 给习惯 CLI 的人复制用。 */
  const lastCommand = ref("");

  const ready = computed(() => Boolean(getProjectFile() && getDocument()));
  const isRepo = computed(() => Boolean(status.value?.is_repo));
  const changeCount = computed(() => status.value?.changes.length ?? 0);

  function push(text: string) {
    const stamp = new Date().toLocaleTimeString("zh-CN", { hour12: false });
    log.value = ["[" + stamp + "] " + text].concat(log.value).slice(0, 100);
  }

  function fail(err: unknown) {
    error.value = err instanceof Error ? err.message : String(err);
    push("失败：" + error.value);
  }

  async function guard<T>(label: string, fn: () => Promise<T>): Promise<T | null> {
    busy.value = true;
    error.value = "";
    try {
      return await fn();
    } catch (e) {
      fail(e);
      return null;
    } finally {
      busy.value = false;
      void label;
    }
  }

  /** 保证内置 SC2Diff 就位（首次调用即静默安装）。 */
  async function ensureTool(force = false) {
    const res = await guard("准备 SC2Diff…", () => (force ? api.vcsReinstall() : api.vcsTool()));
    if (res) {
      tool.value = res;
      if (res.installed) {
        push("SC2Diff " + (res.version ?? "?") + " 就位（" + res.source + "）");
      } else {
        push(res.message ?? "SC2Diff 不可用");
      }
    }
    return tool.value;
  }

  async function refresh() {
    const pf = getProjectFile();
    const doc = getDocument();
    if (!pf || !doc) {
      status.value = null;
      return;
    }
    const bundle = await guard("读取版本状态…", () => api.vcsStatus(pf, doc));
    if (!bundle) return;
    tool.value = bundle.tool;
    repo.value = bundle.repo;
    status.value = bundle.status;
    if (!bundle.status.is_repo) {
      commits.value = [];
      refs.value = { branches: [], tags: [] };
      return;
    }
    const [logRes, refsRes] = await Promise.all([api.vcsLog(pf, doc, 50), api.vcsRefs(pf, doc)]);
    commits.value = logRes;
    refs.value = refsRes;
  }

  /** 为当前文档建仓（导入当前文档作为基线）。 */
  async function initRepo() {
    const pf = getProjectFile();
    const doc = getDocument();
    if (!pf || !doc) return;
    const res = await guard("初始化仓库…", () => api.vcsInit(pf, doc));
    if (res) {
      lastCommand.value = 'sc2diff -C "<repo>" init "' + doc + '"';
      push("已建仓并导入 " + doc);
      await refresh();
    }
  }

  /** 提交全部改动。 */
  async function commit() {
    const pf = getProjectFile();
    const doc = getDocument();
    if (!pf || !doc) return;
    const text = message.value.trim() || "更新 " + new Date().toLocaleString("zh-CN");
    const res = await guard("提交…", () => api.vcsCommit(pf, doc, text));
    if (res) {
      lastCommand.value = 'sc2diff -C "<repo>" add -A && sc2diff -C "<repo>" commit -m "' + text + '"';
      message.value = "";
      push("已提交：" + text);
      await refresh();
    }
  }

  /** 看某个提交相对上一个提交的语义 diff。 */
  async function showDiff(sha: string) {
    const pf = getProjectFile();
    const doc = getDocument();
    if (!pf || !doc) return;
    selectedCommit.value = sha;
    const before = commits.value[commits.value.findIndex((c) => c.sha === sha) + 1];
    const from = before?.sha ?? null;
    const res = await guard("取差异…", () => api.vcsDiff(pf, doc, from, sha));
    if (res) {
      diffText.value = res.text;
      lastCommand.value = "sc2diff -C \"<repo>\" diff " + (from ? from.slice(0, 8) + " " : "") + sha.slice(0, 8);
    }
  }

  /** 看工作树相对索引的差异。 */
  async function showWorkingDiff() {
    const pf = getProjectFile();
    const doc = getDocument();
    if (!pf || !doc) return;
    selectedCommit.value = null;
    const res = await guard("取工作树差异…", () => api.vcsDiff(pf, doc, null, null));
    if (res) {
      diffText.value = res.text;
      lastCommand.value = 'sc2diff -C "<repo>" diff';
    }
  }

  async function refAction(action: "branch" | "tag" | "checkout" | "switch", value: string) {
    const pf = getProjectFile();
    const doc = getDocument();
    if (!pf || !doc || !value.trim()) return;
    const res = await guard("执行 " + action + "…", () => api.vcsRefAction(pf, doc, action, value.trim()));
    if (res) {
      lastCommand.value = res.command;
      push(res.output.split("\n")[0] || action + " " + value);
      await refresh();
    }
  }

  /** 从仓库导出文档。 */
  async function pack() {
    const pf = getProjectFile();
    const doc = getDocument();
    if (!pf || !doc) return;
    const stem = (doc.split(/[\\/]/).pop() ?? "out").replace(/\.[^.]+$/, "");
    const out = await api.vcsPickPack(stem + ".from-repo.SC2Map");
    if (!out) return;
    const res = await guard("导出文档…", () => api.vcsPack(pf, doc, out));
    if (res) {
      lastPath.value = res.path;
      lastCommand.value = 'sc2diff -C "<repo>" pack "' + out + '"';
      push("已导出 " + res.path);
    }
  }

  /** 把当前文档重新解包进仓库工作树（外部改过地图后用）。 */
  async function syncWorkingTree() {
    const pf = getProjectFile();
    const doc = getDocument();
    if (!pf || !doc) return;
    const res = await guard("同步工作树…", () => api.vcsUnpack(pf, doc));
    if (res) {
      lastCommand.value = 'sc2diff -C "<repo>" unpack "' + doc + '"';
      push("已把当前文档解包进仓库工作树");
      await refresh();
    }
  }

  const lastPath = ref("");

  async function reveal() {
    const pf = getProjectFile();
    const doc = getDocument();
    if (pf && doc) await api.vcsReveal(pf, doc);
  }

  return {
    tool,
    repo,
    status,
    commits,
    refs,
    diffText,
    selectedCommit,
    message,
    busy,
    log,
    error,
    lastCommand,
    lastPath,
    ready,
    isRepo,
    changeCount,
    ensureTool,
    refresh,
    initRepo,
    commit,
    showDiff,
    showWorkingDiff,
    refAction,
    pack,
    syncWorkingTree,
    reveal,
    push,
  };
}

export type VcsApi = ReturnType<typeof useVcs>;
