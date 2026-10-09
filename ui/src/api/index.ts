// 类型化命令封装 + 展示用格式化助手。组件只调这里，不直接碰 invoke。

import { call, isTauri } from "./bridge";
import type {
  ApplyResult,
  AppInfo,
  AuditReport,
  Glossary,
  GlossaryBundle,
  InspectReport,
  MtConfig,
  MtResult,
  MtSuggestion,
  ToolStatus,
  Project,
  RecentEntry,
  Term,
  TranslationDocument,
  VcsCommit,
  VcsRefs,
  VcsStatusBundle,
  VerifyReport,
} from "./types";

export { isTauri };
export type * from "./types";

export const api = {
  appInfo: () => call<AppInfo>("app_info"),

  pickArchive: () => call<string | null>("pick_archive"),
  /** 一次选多个地图 / 模组。 */
  pickArchives: () => call<string[]>("pick_archives"),

  // --- 项目 ---
  recentProjects: () => call<{ items: RecentEntry[] }>("recent_projects"),
  forgetProject: (path: string) => call<void>("forget_project", { path }),
  pickProjectSave: (defaultName: string) => call<string | null>("pick_project_save", { defaultName }),
  pickProjectOpen: () => call<string | null>("pick_project_open"),
  saveProject: (path: string, project: Project) =>
    call<Project>("save_project", { path, project }),
  loadProject: (path: string) => call<Project>("load_project", { path }),
  pickJsonSave: (defaultName: string) => call<string | null>("pick_json_save", { defaultName }),
  pickJsonOpen: () => call<string | null>("pick_json_open"),
  pickArchiveSave: (defaultName: string) => call<string | null>("pick_archive_save", { defaultName }),
  defaultOutputName: (archive: string, target: string) =>
    call<string>("default_output_name", { archive, target }),
  defaultJsonName: (archive: string, target: string) =>
    call<string>("default_json_name", { archive, target }),

  inspect: (archive: string) => call<InspectReport>("inspect", { archive }),
  audit: (archive: string, source: string) => call<AuditReport>("audit", { archive, source }),

  /** 载入全部翻译键（表格数据源）。 */
  loadKeys: (archive: string, source: string, target: string, allLocales: boolean) =>
    call<TranslationDocument>("load_keys", { archive, source, target, allLocales }),

  /** 把内存里的文档导出成 JSON 文件。 */
  exportDocument: (doc: TranslationDocument, out: string) =>
    call<{ path: string; bytes: number }>("export_document", { doc, out }),

  /** 读入一份翻译 JSON（导入外部译文）。 */
  importDocument: (path: string) => call<unknown>("import_document", { path }),

  /** 把内存里的文档写回归档。 */
  applyDocument: (
    archive: string,
    doc: TranslationDocument,
    out: string,
    withOriginal: boolean,
    sortKeys: boolean,
  ) => call<ApplyResult>("apply_document", { archive, doc, out, withOriginal, sortKeys }),

  seed: (
    archive: string,
    source: string,
    target: string,
    out: string,
    withOriginal: boolean,
    sortKeys: boolean,
  ) => call<ApplyResult>("seed", { archive, source, target, out, withOriginal, sortKeys }),

  /** 补全默认语言：用 source 的文本覆盖「没有语言文件」的语言。 */
  fillLocales: (
    archive: string,
    source: string,
    locales: string,
    onlyMissing: boolean,
    withOriginal: boolean,
    out: string,
  ) => call<ApplyResult>("fill_locales", { archive, source, locales, onlyMissing, withOriginal, out }),

  verify: (archive: string, deep: boolean) => call<VerifyReport>("verify", { archive, deep }),

  // --- 专有名词库 ---
  glossaryLoad: (projectFile: string | null) =>
    call<GlossaryBundle>("glossary_load", { projectFile: projectFile ?? null }),
  glossarySave: (scope: string, glossary: Glossary, projectFile: string | null) =>
    call<{ path: string; terms: number }>("glossary_save", { scope, glossary, projectFile }),
  glossaryPick: (save: boolean, defaultName: string) =>
    call<string | null>("glossary_pick", { save, defaultName }),
  glossaryRead: (path: string, scope: string) =>
    call<{ terms: Term[]; count: number; format: string }>("glossary_read", { path, scope }),
  glossaryWrite: (path: string, glossary: Glossary) =>
    call<{ path: string; terms: number }>("glossary_write", { path, glossary }),

  // --- 机翻参考 ---
  mtConfigGet: () => call<MtConfig>("mt_config_get"),
  mtConfigSet: (config: MtConfig) => call<MtConfig>("mt_config_set", { config }),
  mtTranslate: (text: string, projectFile: string | null) =>
    call<MtSuggestion>("mt_translate", { text, projectFile: projectFile ?? null }),
  mtTranslateMany: (texts: string[], projectFile: string | null) =>
    call<{ results: MtResult[] }>("mt_translate_many", { texts, projectFile: projectFile ?? null }),
  mtClearCache: () => call<{ cleared: number }>("mt_clear_cache"),

  // --- 版本管理 ---
  vcsTool: () => call<ToolStatus>("vcs_tool"),
  vcsReinstall: () => call<ToolStatus>("vcs_reinstall"),
  vcsRepoPath: (projectFile: string, document: string) =>
    call<string>("vcs_repo_path", { projectFile, document }),
  vcsStatus: (projectFile: string, document: string) =>
    call<VcsStatusBundle>("vcs_status", { projectFile, document }),
  vcsInit: (projectFile: string, document: string) =>
    call<{ output: string }>("vcs_init", { projectFile, document }),
  vcsCommit: (projectFile: string, document: string, message: string) =>
    call<{ staged: string; commit: string }>("vcs_commit", { projectFile, document, message }),
  vcsLog: (projectFile: string, document: string, limit: number) =>
    call<VcsCommit[]>("vcs_log", { projectFile, document, limit }),
  vcsDiff: (projectFile: string, document: string, from: string | null, to: string | null) =>
    call<{ text: string }>("vcs_diff", { projectFile, document, from, to }),
  vcsRefs: (projectFile: string, document: string) =>
    call<VcsRefs>("vcs_refs", { projectFile, document }),
  vcsRefAction: (projectFile: string, document: string, action: string, value: string) =>
    call<{ output: string; command: string }>("vcs_ref_action", { projectFile, document, action, value }),
  vcsPack: (projectFile: string, document: string, out: string) =>
    call<{ output: string; path: string }>("vcs_pack", { projectFile, document, out }),
  vcsPickPack: (defaultName: string) => call<string | null>("vcs_pick_pack", { defaultName }),
  vcsUnpack: (projectFile: string, document: string) =>
    call<{ output: string }>("vcs_unpack", { projectFile, document }),
  /** 选一张背景图片（原生对话框）。 */
  pickImage: () => call<string | null>("pick_image"),

  vcsReveal: (projectFile: string, document: string) =>
    call<void>("vcs_reveal", { projectFile, document }),
  reveal: (path: string) => call<void>("reveal", { path }),
};

// --------------------------------------------------------------------------- //
// 展示助手
// --------------------------------------------------------------------------- //

/**
 * 工程方块的文字：照 JetBrains 的规则取前两个「词」的首字母。
 *
 * 规则（对着参考图里 10 个真实项目逐个对出来的）：
 * - 按分隔符与驼峰切词，丢掉纯数字的词；
 * - 第一个词全是字母大写（MDK / SLS）就取它前两个字符；
 * - 否则取前两个词的首字母；
 * - 只有一个词（中文名、SCMR 这种）就取前两个字符。
 */
export function projectInitials(name: string): string {
  const cleaned = name.replace(/\.(sc2transproj|SC2Map|SC2Mod)$/i, "");
  const parts = cleaned
    .replace(/([a-z0-9])([A-Z])/g, "$1 $2")
    .split(/[\s_\-./\\()[\]]+/)
    .filter((w) => w.length > 0 && !/^[0-9]+$/.test(w));
  if (!parts.length) return cleaned.slice(0, 2).toUpperCase();

  const first = parts[0];
  if (parts.length === 1) return first.slice(0, 2).toUpperCase();
  if (first.length >= 2 && first === first.toUpperCase() && /[A-Z]/.test(first)) {
    return first.slice(0, 2).toUpperCase();
  }
  return (first[0] + parts[1][0]).toUpperCase();
}

/** 方块底色：照 JetBrains 的做法由路径决定，同一个工程永远同色。 */
const AVATAR_COLORS = [
  "#3574F0",
  "#55A76A",
  "#E08840",
  "#4FA3A3",
  "#7A6FF0",
  "#B45BC4",
  "#C4508F",
  "#8A63D2",
  "#C9A227",
  "#3D9AA8",
];

export function avatarColor(seed: string): string {
  let h = 0x811c9dc5;
  for (let i = 0; i < seed.length; i += 1) {
    h = Math.imul(h ^ seed.charCodeAt(i), 0x01000193) >>> 0;
  }
  return AVATAR_COLORS[h % AVATAR_COLORS.length];
}

export function formatCoverage(coverage: number): string {
  return (coverage * 100).toFixed(1) + "%";
}

export function formatBytes(bytes: number): string {
  if (bytes < 1024) return bytes + " B";
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + " KB";
  return (bytes / 1024 / 1024).toFixed(2) + " MB";
}

/** 状态 -> 中文标签与色板键。 */
export const STATUS_META: Record<string, { label: string; tone: "err" | "warn" | "ok" | "info" | "muted" }> = {
  missing: { label: "缺失", tone: "err" },
  identical: { label: "未翻译", tone: "warn" },
  translated: { label: "已翻译", tone: "ok" },
  extra: { label: "多余", tone: "info" },
  "no-source": { label: "无原文", tone: "muted" },
};

export function statusLabel(status: string): string {
  return STATUS_META[status]?.label ?? status;
}

/** 从归档路径里取文件名（跨平台）。 */
export function baseName(path: string): string {
  const parts = path.split(/[\\/]/);
  return parts[parts.length - 1] || path;
}

/** 内联标记统计：回填时必须整体保留，这里给用户一个提示。 */
export function markupTokens(text: string | null | undefined): string[] {
  if (!text) return [];
  const out: string[] = [];
  const patterns = [/<n\/>/g, /<c val="[^"]*">/g, /<s val="[^"]*">/g, /<IMG [^>]*>/g, /%[^%]+%/g, /~[A-Za-z]~/g];
  for (const p of patterns) {
    for (const m of text.match(p) ?? []) out.push(m);
  }
  return out;
}
