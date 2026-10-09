// 工作区：**一个文件夹就是一个工程**。
//
// 分工说明（重要）：
//   * 这个 composable 管**持久化**：工程根、.sc2translater/workspace.json、扫描出来的文件树、
//     最近打开的文件夹、老项目文件迁移；
//   * useProject 管**编辑会话**：当前打开的归档、翻译键、撤销栈。
//   两者通过 setRoot + 相对路径对接：清单里存相对，调 api 前解析成绝对。

import { computed, ref, watch } from "vue";

import { api, isTauri } from "../api";
import type { FileEntry, OpenFile } from "../api/types";

/** core 里的 Workspace 结构（只用得到这些字段）。 */
export interface WorkspaceEntry {
  path: string;
  label: string;
  source: string;
  target: string;
  output?: string | null;
  keys_json?: string | null;
}

export interface WorkspaceModel {
  format: number;
  name: string;
  defaults: { source: string; target: string };
  files: WorkspaceEntry[];
  open: string[];
  active: number;
  view: Record<string, unknown>;
  created_at: string;
  updated_at: string;
}

/** 演示工程目录（浏览器调试用）。 */
export const DEMO_FOLDER = "D:/示例/斯托科夫战役";

export function useWorkspace() {
  const root = ref("");
  const model = ref<WorkspaceModel | null>(null);
  const tree = ref<FileEntry[]>([]);
  const active = ref<OpenFile | null>(null);
  const legacy = ref<string[]>([]);
  const busy = ref(false);
  const error = ref("");
  const status = ref("");

  const ready = computed(() => Boolean(root.value));
  const name = computed(() => model.value?.name ?? "");

  function fail(e: unknown) {
    error.value = e instanceof Error ? e.message : String(e);
  }

  /** 打开一个文件夹**并读它的工作区**（没有清单就是新建）。 */
  async function load(path: string) {
    root.value = path;
    busy.value = true;
    error.value = "";
    try {
      const res = await api.workspaceLoad(path);
      model.value = res.workspace as WorkspaceModel;
      legacy.value = res.legacy_projects ?? [];
      tree.value = await api.workspaceScan(path);
      await api.recentTouch(path);
      status.value = legacy.value.length
        ? "这个文件夹里有 " + legacy.value.length + " 个老项目文件，可以升级成工作区"
        : "";
    } catch (e) {
      fail(e);
    } finally {
      busy.value = false;
    }
    return root.value;
  }

  /** 弹文件夹选择框（桌面壳）；浏览器调试时用演示目录。 */
  async function openFolder(): Promise<string> {
    const path = isTauri ? await api.pickFolder() : DEMO_FOLDER;
    if (!path) return "";
    return load(path);
  }

  /** 把老项目文件迁移成工作区（不删原件，另存 .bak）。 */
  async function migrate(projectFile: string) {
    busy.value = true;
    try {
      const res = await api.workspaceMigrate(root.value, projectFile);
      model.value = res.workspace as WorkspaceModel;
      legacy.value = [];
      status.value = res.outside.length
        ? "迁移完成；有 " + res.outside.length + " 个文件在工程目录之外，路径保持绝对"
        : "迁移完成";
      return res;
    } catch (e) {
      fail(e);
      return null;
    } finally {
      busy.value = false;
    }
  }

  /** 重新扫一遍目录。 */
  async function rescan() {
    if (!root.value) return;
    try {
      tree.value = await api.workspaceScan(root.value);
    } catch (e) {
      fail(e);
    }
  }

  /** 写清单（改一个文件条目就调一次，去抖）。 */
  let timer: number | undefined;
  async function save() {
    if (!root.value || !model.value) return;
    try {
      await api.workspaceSave(root.value, model.value);
    } catch (e) {
      fail(e);
    }
  }

  function saveSoon() {
    if (timer) window.clearTimeout(timer);
    timer = window.setTimeout(() => void save(), 600);
  }

  watch(model, () => saveSoon(), { deep: true });

  /** 打开一个文件（路径相对工程根）。 */
  async function open(rel: string) {
    if (!root.value) return;
    busy.value = true;
    error.value = "";
    try {
      active.value = await api.openFile(root.value, rel);
      if (model.value && !model.value.open.includes(rel)) {
        model.value.open.push(rel);
      }
    } catch (e) {
      fail(e);
      active.value = { path: rel, kind: "binary", size: 0, reason: error.value };
    } finally {
      busy.value = false;
    }
  }

  function close() {
    active.value = null;
  }

  /** 关掉工作区，回到小窗。 */
  function closeWorkspace() {
    root.value = "";
    model.value = null;
    tree.value = [];
    active.value = null;
    legacy.value = [];
    status.value = "";
  }

  /** 把 useProject 的编辑会话接上工程根。 */
  function bind(project: { setRoot: (r: string) => void }) {
    project.setRoot(root.value);
  }

  return {
    root,
    model,
    tree,
    active,
    legacy,
    busy,
    error,
    status,
    ready,
    name,
    load,
    openFolder,
    migrate,
    rescan,
    save,
    open,
    close,
    closeWorkspace,
    bind,
  };
}

export type WorkspaceApi = ReturnType<typeof useWorkspace>;
