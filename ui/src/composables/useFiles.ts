// 打开任意文件：判类型、给对应的查看器用。
//
// 这一层只管「这个文件是什么、能不能打开」，渲染交给各自的查看器组件。

import { computed, ref } from "vue";

import { api } from "../api";
import type { FileEntry, FileKind, OpenFile } from "../api/types";

export function useFiles() {
  const root = ref("");
  const tree = ref<FileEntry[]>([]);
  const active = ref<OpenFile | null>(null);
  const busy = ref(false);
  const error = ref("");
  /** 未保存的改动（配合撤销栈，见开发文档第 5 节）。 */
  const dirty = ref(false);
  const undoDepth = ref(0);

  /** 当前打开的是不是 SC2 文档（走翻译键视图而不是文件查看器）。 */
  const isSc2 = computed(() => active.value?.kind === "sc2");

  async function scan(path: string) {
    root.value = path;
    try {
      tree.value = await api.scanFolder(path);
      error.value = "";
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e);
    }
  }

  async function open(path: string) {
    busy.value = true;
    error.value = "";
    try {
      active.value = await api.openFile(root.value, path);
      dirty.value = false;
      undoDepth.value = 0;
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e);
      active.value = { path, kind: "binary", size: 0, reason: error.value };
    } finally {
      busy.value = false;
    }
  }

  function close() {
    active.value = null;
  }

  /** 记一次可撤销的改动（真正的栈在 core，这里只做界面提示）。 */
  function markEdited() {
    dirty.value = true;
    undoDepth.value += 1;
  }

  function markSaved() {
    dirty.value = false;
  }

  const KIND_LABEL: Record<FileKind, string> = {
    sc2: "星际争霸 II 文档",
    text: "文本",
    image: "图片",
    binary: "不支持的格式",
  };

  function label(kind: FileKind): string {
    return KIND_LABEL[kind];
  }

  return { root, tree, active, busy, error, dirty, undoDepth, isSc2, scan, open, close, markEdited, markSaved, label };
}

export type FilesApi = ReturnType<typeof useFiles>;
