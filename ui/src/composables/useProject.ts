// 项目状态与动作。
//
// 组件只负责渲染；所有业务动作、过滤、统计、脏标记都收在这里，便于测试与替换。
// 设计上按「专业工具」的路子来：三栏 + 表格 + 键盘优先，而不是一堆大按钮的命令单。

import { computed, ref, shallowRef } from "vue";

import { api, formatCoverage, markupTokens } from "../api";
import type { ApplyResult, AuditReport, Entry, InspectReport, TranslationDocument, VerifyReport } from "../api/types";

/** 常见语言，供下拉框使用；也允许手填（归档里实测出现过 zhCH / enGB 等）。 */
export const COMMON_LOCALES = [
  { code: "enUS", label: "英语（美国）" },
  { code: "zhCN", label: "简体中文" },
  { code: "zhTW", label: "繁体中文" },
  { code: "koKR", label: "韩语" },
  { code: "jaJP", label: "日语" },
  { code: "ruRU", label: "俄语" },
  { code: "deDE", label: "德语" },
  { code: "frFR", label: "法语" },
  { code: "esES", label: "西班牙语" },
  { code: "ptBR", label: "葡萄牙语（巴西）" },
];

export type StatusFilter = "all" | "todo" | "missing" | "identical" | "translated" | "extra";
export type Density = "compact" | "standard" | "comfortable";
export type CompareMode = "by-file" | "attention-first";

/** 表格行的唯一 id：同一个 key 可能出现在不同的字符串表里。 */
export function entryId(entry: Entry): string {
  return entry.file + "::" + entry.key;
}

export function useProject() {
  const archive = ref<string | null>(null);
  const source = ref("enUS");
  const target = ref("zhCN");

  const info = shallowRef<InspectReport | null>(null);
  const audit = shallowRef<AuditReport | null>(null);
  const doc = shallowRef<TranslationDocument | null>(null);
  /** 工作副本：表格直接改这里，导出/写回都用它。 */
  const entries = ref<Entry[]>([]);
  const originals = new Map<string, string>();

  const selected = ref<string | null>(null);
  const checked = ref<Set<string>>(new Set());
  const dirty = ref<Set<string>>(new Set());

  const query = ref("");
  const statusFilter = ref<StatusFilter>("todo");
  const fileFilter = ref<string>("all");
  const density = ref<Density>("standard");

  const busy = ref(false);
  const busyText = ref("");
  const error = ref("");
  const logs = ref<string[]>([]);
  const result = shallowRef<ApplyResult | null>(null);
  const verifyReport = shallowRef<VerifyReport | null>(null);
  const lastExport = ref<string>("");

  const hasArchive = computed(() => Boolean(doc.value));

  // ------------------------------------------------------------------ 日志
  function log(message: string) {
    const stamp = new Date().toLocaleTimeString("zh-CN", { hour12: false });
    logs.value = ["[" + stamp + "] " + message].concat(logs.value).slice(0, 120);
  }

  function fail(err: unknown) {
    const text = err instanceof Error ? err.message : String(err);
    error.value = text;
    log("失败：" + text);
  }

  async function guard<T>(label: string, fn: () => Promise<T>): Promise<T | null> {
    busy.value = true;
    busyText.value = label;
    error.value = "";
    try {
      return await fn();
    } catch (err) {
      fail(err);
      return null;
    } finally {
      busy.value = false;
      busyText.value = "";
    }
  }

  // ------------------------------------------------------------------ 过滤
  const files = computed<string[]>(() => {
    const set = new Set(entries.value.map((e) => e.file));
    return Array.from(set).sort();
  });

  const filtered = computed<Entry[]>(() => {
    const q = query.value.trim().toLowerCase();
    return entries.value.filter((e) => {
      if (fileFilter.value !== "all" && e.file !== fileFilter.value) return false;
      switch (statusFilter.value) {
        case "todo":
          if (e.status !== "missing" && e.status !== "identical") return false;
          break;
        case "missing":
          if (e.status !== "missing") return false;
          break;
        case "identical":
          if (e.status !== "identical") return false;
          break;
        case "translated":
          if (e.status !== "translated") return false;
          break;
        case "extra":
          if (e.status !== "extra" && e.status !== "no-source") return false;
          break;
        default:
          break;
      }
      if (!q) return true;
      return (
        e.key.toLowerCase().includes(q) ||
        (e.source ?? "").toLowerCase().includes(q) ||
        (e.target ?? "").toLowerCase().includes(q)
      );
    });
  });

  const selectedEntry = computed<Entry | null>(() => {
    const id = selected.value;
    if (!id) return null;
    return entries.value.find((e) => entryId(e) === id) ?? null;
  });

  /** 逐状态计数（基于全部条目，不受过滤影响）。 */
  const counts = computed(() => {
    const out: Record<string, number> = { all: entries.value.length, missing: 0, identical: 0, translated: 0, extra: 0, "no-source": 0 };
    for (const e of entries.value) {
      out[e.status] = (out[e.status] ?? 0) + 1;
    }
    out.todo = out.missing + out.identical;
    return out;
  });

  /** 玩家可见文本的完成度（translated + identical 视为已有文本）。 */
  const progress = computed(() => {
    const total = entries.value.length;
    if (!total) return { done: 0, total: 0, percent: 0 };
    const done = entries.value.filter((e) => (e.target ?? "").trim().length > 0).length;
    return { done, total, percent: Math.round((done / total) * 1000) / 10 };
  });

  const dirtyCount = computed(() => dirty.value.size);

  // ------------------------------------------------------------------ 动作
  async function openArchive() {
    const path = await guard("选择地图…", () => api.pickArchive());
    if (!path) return;
    archive.value = path;
    await reload();
  }

  /** 直接指定归档路径（拖放文件用）。 */
  async function openPath(path: string) {
    archive.value = path;
    await reload();
  }

  async function reload() {
    if (!archive.value) return;
    const path = archive.value;
    await guard("读取归档…", async () => {
      info.value = await api.inspect(path);
      audit.value = await api.audit(path, source.value);
      const loaded = await api.loadKeys(path, source.value, target.value, false);
      doc.value = loaded;
      entries.value = loaded.entries.map((e) => ({ ...e }));
      originals.clear();
      for (const e of entries.value) originals.set(entryId(e), e.target ?? "");
      dirty.value = new Set();
      checked.value = new Set();
      selected.value = filtered.value[0] ? entryId(filtered.value[0]) : null;
      result.value = null;
      verifyReport.value = null;
      log(
        "载入 " + loaded.stats.entries + " 个键；" + (loaded.diagnosis?.message ?? ""),
      );
    });
  }

  async function changeLanguages(nextSource: string, nextTarget: string) {
    source.value = nextSource;
    target.value = nextTarget;
    if (dirtyCount.value > 0) {
      log("注意：切换语言会丢弃当前 " + dirtyCount.value + " 处未保存的编辑。");
    }
    await reload();
  }

  function editTarget(entry: Entry, value: string) {
    entry.target = value;
    const id = entryId(entry);
    const base = originals.get(id) ?? "";
    const next = new Set(dirty.value);
    if (value === base) next.delete(id);
    else next.add(id);
    dirty.value = next;
    // 状态跟着改：有值且与原值不同就是「已翻译」，清空就回到「缺失」
    if (!value.trim()) {
      entry.status = "missing";
    } else if (entry.source && value === entry.source) {
      entry.status = "identical";
    } else {
      entry.status = "translated";
    }
  }

  /** 把原文填进译文（只填空的）。scope 决定作用范围。 */
  function fillFromSource(scope: "selected" | "filtered" | "all") {
    const targets =
      scope === "all"
        ? entries.value
        : scope === "filtered"
          ? filtered.value
          : selectedEntry.value
            ? [selectedEntry.value]
            : [];
    let n = 0;
    for (const e of targets) {
      if (!(e.target ?? "").trim() && e.source) {
        editTarget(e, e.source);
        n += 1;
      }
    }
    log("把原文填入译文：" + n + " 条（范围：" + scope + "）");
  }

  /** 清空译文（把已填的删掉）。 */
  function clearTargets(scope: "selected" | "filtered" | "all") {
    const targets =
      scope === "all" ? entries.value : scope === "filtered" ? filtered.value : selectedEntry.value ? [selectedEntry.value] : [];
    let n = 0;
    for (const e of targets) {
      if ((e.target ?? "").trim()) {
        editTarget(e, "");
        n += 1;
      }
    }
    log("清空译文：" + n + " 条");
  }

  function revertEdits() {
    for (const e of entries.value) {
      const base = originals.get(entryId(e)) ?? "";
      if ((e.target ?? "") !== base) {
        e.target = base || null;
        e.status = base ? (base === e.source ? "identical" : "translated") : "missing";
      }
    }
    dirty.value = new Set();
    log("已放弃全部未保存编辑");
  }

  function toggleCheck(id: string) {
    const next = new Set(checked.value);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    checked.value = next;
  }

  function checkAllFiltered() {
    checked.value = new Set(filtered.value.map(entryId));
  }

  function clearChecked() {
    checked.value = new Set();
  }

  /** 把一条翻译条目还原成「当前编辑结果」的文档，供导出/写回使用。 */
  function currentDocument(): TranslationDocument | null {
    if (!doc.value) return null;
    const byStatus: Record<string, number> = {};
    for (const e of entries.value) byStatus[e.status] = (byStatus[e.status] ?? 0) + 1;
    return {
      ...doc.value,
      target_locale: target.value,
      source_locale: source.value,
      entries: entries.value.map((e) => ({ ...e })),
      stats: { ...doc.value.stats, entries: entries.value.length, by_status: byStatus },
    };
  }

  async function exportJson() {
    const current = currentDocument();
    if (!current || !archive.value) return;
    const name = await api.defaultJsonName(archive.value, target.value);
    const out = await api.pickJsonSave(name);
    if (!out) return;
    const res = await guard("导出 JSON…", () => api.exportDocument(current, out));
    if (res) {
      lastExport.value = res.path;
      log("已导出 " + entries.value.length + " 条到 " + res.path);
    }
  }

  /** 导入一份外部翻译 JSON，按 (file, key) 合并进当前工作表（不改原文，只填译文）。 */
  async function importJson() {
    const path = await api.pickJsonOpen();
    if (!path) return;
    const raw = await guard("导入译文…", () => api.importDocument(path));
    if (!raw) return;

    type Item = { file?: string; key: string; target: string };
    const items: Item[] = [];
    const obj = raw as { entries?: unknown } & Record<string, unknown>;
    if (Array.isArray(obj.entries)) {
      for (const e of obj.entries as Array<Record<string, unknown>>) {
        const key = e?.key;
        const target = e?.target;
        if (typeof key === "string" && typeof target === "string" && target.length > 0) {
          items.push({ file: typeof e.file === "string" ? e.file : undefined, key, target });
        }
      }
    } else {
      for (const [key, value] of Object.entries(obj)) {
        if (typeof value === "string" && value.length > 0) items.push({ key, target: value });
      }
    }

    let applied = 0;
    let skipped = 0;
    for (const item of items) {
      const hit = item.file
        ? entries.value.find((e) => e.file === item.file && e.key === item.key)
        : entries.value.find((e) => e.key === item.key);
      if (!hit) {
        skipped += 1;
        continue;
      }
      if ((hit.target ?? "") !== item.target) {
        editTarget(hit, item.target);
        applied += 1;
      }
    }
    log("导入 " + path + "：应用 " + applied + " 条" + (skipped ? "，未匹配 " + skipped + " 条" : ""));
  }

  async function applyToArchive() {
    const current = currentDocument();
    if (!current || !archive.value) return;
    const name = await api.defaultOutputName(archive.value, target.value);
    const out = await api.pickArchiveSave(name);
    if (!out) return;
    const res = await guard("写回归档…", () => api.applyDocument(archive.value as string, current, out, false, false));
    if (res) {
      result.value = res;
      const files = Object.values(res.components_written[res.target_locale] ?? {});
      const total = files.reduce((sum, f) => sum + f.total, 0);
      log("已写出 " + res.output + "；" + res.target_locale + " 共 " + total + " 条，回读校验" + (res.ok ? "通过" : "失败"));
      dirty.value = new Set();
    }
  }

  async function seedMissing() {
    if (!archive.value) return;
    const name = await api.defaultOutputName(archive.value, target.value);
    const out = await api.pickArchiveSave(name);
    if (!out) return;
    const res = await guard("一键兜底…", () =>
      api.seed(archive.value as string, source.value, target.value, out, false, false),
    );
    if (res) {
      result.value = res;
      log("已写出 " + res.output + "；补了 " + (res.seeded ?? 0) + " 个键");
      await reload();
    }
  }

  /**
   * 补全默认语言：用当前源语言的文本，去覆盖那些「没有语言文件」的语言。
   *
   * locales 为 "all" 或逗号分隔的语言列表；onlyMissing=true 时不覆盖已有译文。
   */
  async function fillDefaultLanguages(locales: string, onlyMissing: boolean, out: string) {
    if (!archive.value) return;
    const res = await guard("补全默认语言…", () =>
      api.fillLocales(archive.value as string, source.value, locales, onlyMissing, false, out),
    );
    if (res) {
      result.value = res;
      const count = Object.keys(res.components_written).length;
      log(
        "已写出 " +
          res.output +
          "；补全 " +
          count +
          " 种语言，共新增 " +
          (res.seeded ?? 0) +
          " 条，回读校验" +
          (res.ok ? "通过" : "失败"),
      );
    }
  }

  async function runVerify() {
    const path = result.value?.output ?? archive.value;
    if (!path) return;
    const res = await guard("容器自检…", () => api.verify(path, true));
    if (res) {
      verifyReport.value = res;
      log("自检 " + path + "：整体" + (res.healthy ? "健康" : "有问题"));
    }
  }

  async function revealOutput() {
    const path = result.value?.output ?? lastExport.value;
    if (!path) return;
    await guard("打开目录…", () => api.reveal(path));
  }

  // ------------------------------------------------------------------ 键盘导航
  function moveSelection(delta: number) {
    const list = filtered.value;
    if (!list.length) return;
    const currentId = selected.value;
    let index = list.findIndex((e) => entryId(e) === currentId);
    if (index < 0) index = 0;
    index = Math.min(list.length - 1, Math.max(0, index + delta));
    selected.value = entryId(list[index]);
  }

  /** 覆盖率文本，给侧栏用。 */
  function coverageText(percent: number): string {
    return formatCoverage(percent / 100);
  }

  /** 当前条目里的内联标记（提醒用户别弄丢）。 */
  const selectedMarkup = computed(() => markupTokens(selectedEntry.value?.source ?? ""));

  // 注意：**不要**把 ref 塞进 reactive() —— reactive 会深层解包 ref，
  // 于是 s.archive 直接变成值、s.archive.value 变成非法访问。
  // 这里用普通对象，模板与脚本里统一写 .value，类型也清楚。
  const state = {
    archive,
    source,
    target,
    info,
    audit,
    doc,
    entries,
    selected,
    checked,
    dirty,
    query,
    statusFilter,
    fileFilter,
    density,
    busy,
    busyText,
    error,
    logs,
    result,
    verifyReport,
    lastExport,
  };

  return {
    state,
    hasArchive,
    files,
    filtered,
    selectedEntry,
    counts,
    progress,
    dirtyCount,
    selectedMarkup,
    openArchive,
    openPath,
    reload,
    changeLanguages,
    editTarget,
    fillFromSource,
    clearTargets,
    revertEdits,
    toggleCheck,
    checkAllFiltered,
    clearChecked,
    exportJson,
    importJson,
    applyToArchive,
    seedMissing,
    fillDefaultLanguages,
    runVerify,
    revealOutput,
    moveSelection,
    coverageText,
    log,
  };
}

export type Project = ReturnType<typeof useProject>;
