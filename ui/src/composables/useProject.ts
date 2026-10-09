// 项目状态与动作。
//
// 组件只负责渲染；所有业务动作、过滤、统计、脏标记都收在这里，便于测试与替换。
// 设计上按「专业工具」的路子来：三栏 + 表格 + 键盘优先，而不是一堆大按钮的命令单。

import { computed, ref, shallowRef } from "vue";

import { api, formatCoverage, markupTokens } from "../api";
import type {
  ApplyResult,
  AuditReport,
  Entry,
  InspectReport,
  Project as ProjectModel,
  ProjectFile,
  RecentEntry,
  TranslationDocument,
  VerifyReport,
} from "../api/types";

/** 新建一个空项目。 */
function emptyProject(name = "未命名项目"): ProjectModel {
  const now = new Date().toISOString();
  return { format: 1, name, files: [], active: 0, created_at: now, updated_at: now, path: null };
}

/** 从路径取一个好看的标签。 */
function labelFromPath(path: string): string {
  const base = path.split(/[\\/]/).pop() ?? path;
  return base.replace(/\.[^.]+$/, "");
}

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

  // ---- 项目 ----
  const project = ref<ProjectModel>(emptyProject());
  const projectPath = ref<string | null>(null);
  const projectDirty = ref(false);
  const recents = ref<RecentEntry[]>([]);

  const hasArchive = computed(() => Boolean(doc.value));
  const hasProject = computed(() => project.value.files.length > 0);

  /** 项目整体的待翻译量与完成度（把每个文件的缓存统计加起来）。 */
  const projectTotals = computed(() => {
    let entries = 0;
    let todo = 0;
    let translated = 0;
    for (const f of project.value.files) {
      if (!f.stats) continue;
      entries += f.stats.entries;
      todo += f.stats.missing + f.stats.identical;
      translated += f.stats.translated;
    }
    const percent = entries === 0 ? 0 : Math.round(((entries - todo) / entries) * 1000) / 10;
    return { entries, todo, translated, percent };
  });

  function markProjectDirty() {
    projectDirty.value = true;
    project.value.updated_at = new Date().toISOString();
  }

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
      // 把统计写回项目文件条目（文件列表与项目总进度都读它）
      syncActiveStats(loaded, path);
      log(
        "载入 " + loaded.stats.entries + " 个键；" + (loaded.diagnosis?.message ?? ""),
      );
    });
  }

  /** 把一次扫描的结果写回当前项目条目。 */
  function syncActiveStats(loaded: TranslationDocument, path: string) {
    const idx = project.value.files.findIndex((f) => f.path === path);
    if (idx < 0) return;
    const byStatus = loaded.stats.by_status ?? {};
    const want = (k: string) => byStatus[k] ?? 0;
    const file: ProjectFile = {
      ...project.value.files[idx],
      source: source.value,
      target: target.value,
      stats: {
        entries: loaded.stats.entries,
        missing: want("missing"),
        identical: want("identical"),
        translated: want("translated"),
        locales_present: loaded.stats.locales_present?.length ?? 0,
        size: loaded.archive?.size ?? 0,
        scanned_at: new Date().toISOString(),
      },
    };
    project.value.files.splice(idx, 1, file);
    markProjectDirty();
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

  // ------------------------------------------------------------------ 项目动作
  async function refreshRecents() {
    const recent = await guard("读取最近项目…", () => api.recentProjects());
    if (recent) recents.value = recent.items;
  }

  function newProject(name = "未命名项目") {
    project.value = emptyProject(name);
    projectPath.value = null;
    projectDirty.value = false;
    archive.value = null;
    doc.value = null;
    entries.value = [];
    audit.value = null;
    info.value = null;
    result.value = null;
    log("新建项目：" + name);
  }

  async function openProjectDialog() {
    const path = await api.pickProjectOpen();
    if (!path) return;
    await openProject(path);
  }

  async function openProject(path: string) {
    const loaded = await guard("打开项目…", () => api.loadProject(path));
    if (!loaded) return;
    project.value = loaded;
    projectPath.value = loaded.path ?? path;
    projectDirty.value = false;
    await refreshRecents();
    log("打开项目：" + loaded.name + "（" + loaded.files.length + " 个文件）");
    if (loaded.files.length > 0) {
      await selectFile(Math.min(loaded.active, loaded.files.length - 1));
    } else {
      archive.value = null;
      doc.value = null;
      entries.value = [];
    }
  }

  async function saveProject(saveAs = false) {
    let path = projectPath.value;
    if (!path || saveAs) {
      const suggested = (project.value.name || "未命名项目") + ".sc2transproj";
      const picked = await api.pickProjectSave(suggested);
      if (!picked) return;
      path = picked;
    }
    // 去掉 Vue 的响应式包装，避免把 Proxy 塞进 IPC
    const plain: ProjectModel = JSON.parse(JSON.stringify(project.value));
    const saved = await guard("保存项目…", () => api.saveProject(path as string, plain));
    if (saved) {
      project.value = saved;
      projectPath.value = saved.path ?? path;
      projectDirty.value = false;
      await refreshRecents();
      log("已保存项目 " + projectPath.value);
    }
  }

  /** 添加文件（可多选）。 */
  async function addFiles() {
    const picked = await api.pickArchives();
    if (!picked.length) return;
    let firstNew = -1;
    for (const path of picked) {
      const existing = project.value.files.findIndex((f) => f.path === path);
      if (existing >= 0) {
        if (firstNew < 0) firstNew = existing;
        continue;
      }
      project.value.files.push({
        path,
        label: labelFromPath(path),
        source: source.value,
        target: target.value,
        output: null,
        stats: null,
      });
      if (firstNew < 0) firstNew = project.value.files.length - 1;
    }
    markProjectDirty();
    log("已加入 " + picked.length + " 个文件");
    if (firstNew >= 0) await selectFile(firstNew);
  }

  /** 直接按路径加文件（拖放用）。 */
  async function addFilePath(path: string) {
    const existing = project.value.files.findIndex((f) => f.path === path);
    if (existing >= 0) {
      await selectFile(existing);
      return;
    }
    project.value.files.push({
      path,
      label: labelFromPath(path),
      source: source.value,
      target: target.value,
      output: null,
      stats: null,
    });
    markProjectDirty();
    await selectFile(project.value.files.length - 1);
  }

  /** 切换当前文件。 */
  async function selectFile(index: number) {
    if (index < 0 || index >= project.value.files.length) return;
    project.value.active = index;
    const file = project.value.files[index];
    source.value = file.source || "enUS";
    target.value = file.target || "zhCN";
    archive.value = file.path;
    await reload();
  }

  function removeFile(index: number) {
    if (index < 0 || index >= project.value.files.length) return;
    const [removed] = project.value.files.splice(index, 1);
    if (project.value.active > index) project.value.active -= 1;
    if (project.value.active >= project.value.files.length) {
      project.value.active = Math.max(0, project.value.files.length - 1);
    }
    markProjectDirty();
    log("已移出项目：" + removed.label);
    if (project.value.files.length === 0) {
      archive.value = null;
      doc.value = null;
      entries.value = [];
      audit.value = null;
      info.value = null;
    } else if (project.value.active !== index || index === 0) {
      void selectFile(project.value.active);
    }
  }

  function renameFileLabel(index: number, label: string) {
    const file = project.value.files[index];
    if (!file) return;
    file.label = label.trim() || labelFromPath(file.path);
    markProjectDirty();
  }

  function renameProject(name: string) {
    project.value.name = name.trim() || "未命名项目";
    markProjectDirty();
  }

  async function forgetRecent(path: string) {
    await api.forgetProject(path);
    await refreshRecents();
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
    project,
    projectPath,
    projectDirty,
    recents,
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
    hasProject,
    projectTotals,
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
    refreshRecents,
    newProject,
    openProjectDialog,
    openProject,
    saveProject,
    addFiles,
    addFilePath,
    selectFile,
    removeFile,
    renameFileLabel,
    renameProject,
    forgetRecent,
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

/** 组合式函数的返回类型（组件里把它当 "项目 API" 用）。 */
export type Project = ReturnType<typeof useProject>;
