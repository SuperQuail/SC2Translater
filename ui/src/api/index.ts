// 类型化的命令封装 —— 组件只调这些函数，不直接碰 invoke。

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
  extract: (archive: string, source: string, target: string, out: string, allLocales: boolean) =>
    call<TranslationDocument>("extract", { archive, source, target, out, allLocales }),
  apply: (
    archive: string,
    jsonPath: string,
    out: string,
    target: string | null,
    fillMissing: boolean,
    withOriginal: boolean,
    sortKeys: boolean,
  ) =>
    call<ApplyResult>("apply", {
      archive,
      jsonPath,
      out,
      target,
      fillMissing,
      withOriginal,
      sortKeys,
    }),
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

/** 覆盖率 -> 百分比文本。 */
export function formatCoverage(coverage: number): string {
  return (coverage * 100).toFixed(1) + "%";
}

/** 严重程度 -> 中文标签。 */
export function severityLabel(severity: string): string {
  if (severity === "ok") return "正常";
  if (severity === "warning") return "需要处理";
  return "提示";
}

/** 人类可读的文件大小。 */
export function formatBytes(bytes: number): string {
  if (bytes < 1024) return bytes + " B";
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + " KB";
  return (bytes / 1024 / 1024).toFixed(2) + " MB";
}
