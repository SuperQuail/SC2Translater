// 类型化命令封装 + 展示用格式化助手。组件只调这里，不直接碰 invoke。

import { call, isTauri } from "./bridge";
import type {
  ApplyResult,
  AppInfo,
  AuditReport,
  InspectReport,
  TranslationDocument,
  VerifyReport,
} from "./types";

export { isTauri };
export type * from "./types";

export const api = {
  appInfo: () => call<AppInfo>("app_info"),

  pickArchive: () => call<string | null>("pick_archive"),
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

  verify: (archive: string, deep: boolean) => call<VerifyReport>("verify", { archive, deep }),
  reveal: (path: string) => call<void>("reveal", { path }),
};

// --------------------------------------------------------------------------- //
// 展示助手
// --------------------------------------------------------------------------- //

export function formatCoverage(coverage: number): string {
  return (coverage * 100).toFixed(1) + "%";
}

export function severityLabel(severity: string): string {
  if (severity === "ok") return "正常";
  if (severity === "warning") return "需要处理";
  return "提示";
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
