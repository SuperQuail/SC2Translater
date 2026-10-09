// 撤销 / 重做（每个文件一条历史）。
//
// 为什么在前端而不是 core：编辑发生在界面里，放 core 就要每次按键走一趟 IPC。
// （开发文档里原本写的是 core undo.rs，这里改主意了，理由如上，文档也同步改了。）
//
// 存**差分**不存快照：一万个键的文档存 200 份快照是几百 MB。

import { computed, ref } from "vue";

/** 一次改动：某个键从 before 变成 after。 */
export interface Patch {
  key: string;
  before: string;
  after: string;
}

/** 内存里保留多少步。 */
export const HISTORY_LIMIT = 200;

/** 同一个键在这个时间窗内的连续输入合并成一步。 */
const MERGE_MS = 500;

export function useHistory() {
  const past = ref<Patch[][]>([]);
  const future = ref<Patch[][]>([]);
  /** 正在攒的那一步（还没落进 past）。 */
  let pending: (Patch & { at: number }) | null = null;

  const canUndo = computed(() => past.value.length > 0 || pending !== null);
  const canRedo = computed(() => future.value.length > 0);
  const depth = computed(() => past.value.length);

  /** 把攒着的那一步收进历史。切文件、执行批量、点撤销前都要先调。 */
  function flush() {
    if (pending && pending.before !== pending.after) {
      push([{ key: pending.key, before: pending.before, after: pending.after }]);
    }
    pending = null;
  }

  function push(patches: Patch[]) {
    if (!patches.length) return;
    past.value.push(patches);
    if (past.value.length > HISTORY_LIMIT) past.value.shift();
    // 新改动作废「重做」—— 这是撤销栈的规矩
    future.value = [];
  }

  /** 记一次单键改动（连续输入会合并）。 */
  function record(key: string, before: string, after: string) {
    const now = Date.now();
    if (pending && pending.key === key && now - pending.at < MERGE_MS) {
      pending.after = after;
      pending.at = now;
      return;
    }
    flush();
    pending = { key, before, after, at: now };
  }

  /** 记一批（原文填空译文、清空、导入 JSON 这类）。 */
  function recordBatch(patches: Patch[]) {
    flush();
    push(patches.filter((p) => p.before !== p.after));
  }

  /** 取出要撤销的那一步；调用方负责把它写回文档。 */
  function undo(): Patch[] | null {
    flush();
    const step = past.value.pop();
    if (!step) return null;
    future.value.push(step);
    return step;
  }

  function redo(): Patch[] | null {
    flush();
    const step = future.value.pop();
    if (!step) return null;
    past.value.push(step);
    return step;
  }

  function clear() {
    past.value = [];
    future.value = [];
    pending = null;
  }

  return { canUndo, canRedo, depth, record, recordBatch, undo, redo, flush, clear };
}

export type HistoryApi = ReturnType<typeof useHistory>;

/** 把一步补丁写回「键 -> 值」的表。undo 时用 before，redo 时用 after。 */
export function applyPatches(
  patches: Patch[],
  target: Map<string, string> | Record<string, string>,
  direction: "before" | "after",
): void {
  const put = (k: string, v: string) => {
    if (target instanceof Map) target.set(k, v);
    else (target as Record<string, string>)[k] = v;
  };
  // 撤销时倒着回放，重做时正着
  const list = direction === "before" ? [...patches].reverse() : patches;
  for (const p of list) put(p.key, p[direction]);
}
