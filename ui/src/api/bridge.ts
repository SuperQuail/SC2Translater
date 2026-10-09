// 前后端桥接。
//
// 在 Tauri 里走 invoke；在浏览器里（pnpm dev，没有 Rust 后端）自动降级到**演示数据**，
// 这样调 UI 不需要每次都起桌面壳 —— 与参考项目 SC2Miyin Launcher 的做法一致。

import { invoke } from "@tauri-apps/api/core";

import type {
  ApplyResult,
  AppInfo,
  AuditReport,
  InspectReport,
  TranslationDocument,
  VerifyReport,
} from "./types";

/** 当前是不是跑在 Tauri 桌面壳里。 */
export const isTauri: boolean =
  typeof window !== "undefined" &&
  "__TAURI_INTERNALS__" in (window as unknown as Record<string, unknown>);

/** 统一入口；浏览器下走演示数据。 */
export async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauri) {
    return demoInvoke(cmd, args) as T;
  }
  return invoke<T>(cmd, args);
}

// --------------------------------------------------------------------------- //
// 演示模式
// --------------------------------------------------------------------------- //

const DEMO_ARCHIVE = "D:/示例/Stukov2.SC2Map";

function demoAudit(): AuditReport {
  return {
    path: DEMO_ARCHIVE,
    source_locale: "enUS",
    source_keys: 677,
    locales: [
      { locale: "enUS", display: "英语（美国）", keys_expected: 677, keys_present: 677, keys_missing: 0, keys_extra: 0, coverage: 1, files: ["GameStrings.txt", "ObjectStrings.txt", "TriggerStrings.txt"] },
    ],
    by_file: {
      "GameStrings.txt": { player_facing: true, source_keys: 404, locales: [{ locale: "enUS", present: 404, expected: 404, coverage: 1 }] },
      "ObjectStrings.txt": { player_facing: true, source_keys: 29, locales: [{ locale: "enUS", present: 29, expected: 29, coverage: 1 }] },
      "TriggerStrings.txt": { player_facing: false, source_keys: 244, locales: [{ locale: "enUS", present: 244, expected: 244, coverage: 1 }] },
    },
    locales_absent: ["deDE", "zhCN", "zhTW"],
    diagnosis: {
      severity: "warning",
      message: "数据集里只有 enUS：目标语言 zhCN 一个键都没有，游戏语言设为中文时全部文本都会显示成原始键路径。",
      actionable: "先点「一键兜底」让游戏里至少有英文，再导出翻译键交给翻译者。",
    },
  };
}

function demoExtract(target: string): TranslationDocument {
  return {
    schema: "sc2trans.keys.v1",
    tool: "sc2trans (demo)",
    generated_at: "2026-10-09T00:00:00Z",
    archive: { path: DEMO_ARCHIVE, size: 902586, sha256: "demo" },
    source_locale: "enUS",
    target_locale: target,
    files: ["GameStrings.txt", "ObjectStrings.txt", "TriggerStrings.txt"],
    entries: [
      { key: "DocInfo/Name", file: "GameStrings.txt", source: "Stukov's Gambit", target: undefined, status: "missing", origin: ["enUS.SC2Data/GameStrings.txt"] },
      { key: "Param/Value/0F5D0539", file: "GameStrings.txt", source: "Hive Destroyed", target: "主巢被摧毁", status: "translated", origin: ["enUS.SC2Data/GameStrings.txt"] },
      { key: "MapInfo/Player01/Name", file: "GameStrings.txt", source: "Kerrigan's Brood", target: undefined, status: "missing", origin: ["enUS.SC2Data/GameStrings.txt"] },
    ],
    stats: { entries: 677, by_status: { missing: 677 }, by_file: { "GameStrings.txt": 404, "ObjectStrings.txt": 29, "TriggerStrings.txt": 244 }, translatable: 677, locales_present: ["enUS"] },
    diagnosis: { severity: "warning", message: "简体中文（zhCN）缺少 677 / 677 个键。" },
  };
}

function demoApplyResult(mode: string): ApplyResult {
  return {
    ok: true,
    input: DEMO_ARCHIVE,
    output: DEMO_ARCHIVE.replace(".SC2Map", ".zhCN.SC2Map"),
    target_locale: "zhCN",
    source_locale: "enUS",
    components_written: {
      zhCN: {
        "GameStrings.txt": { component: "zhCN.SC2Data\\LocalizedData\\GameStrings.txt", added: 404, updated: 0, total: 404 },
        "ObjectStrings.txt": { component: "zhCN.SC2Data\\LocalizedData\\ObjectStrings.txt", added: 29, updated: 0, total: 29 },
        "TriggerStrings.txt": { component: "zhCN.SC2Data\\LocalizedData\\TriggerStrings.txt", added: 244, updated: 0, total: 244 },
      },
    },
    readback: { "GameStrings.txt": { ok: true, keys: 404, expected: 404 } },
    bytes_in: 902586,
    bytes_out: 878025,
    seeded: mode === "seed" ? 677 : undefined,
  };
}

function demoInvoke(cmd: string, args?: Record<string, unknown>): unknown {
  const target = (args?.target as string) ?? "zhCN";
  switch (cmd) {
    case "app_info":
      return { name: "SC2Translater", version: "0.1.0", tool: "sc2trans 0.1.0 (demo)", schema: "sc2trans.keys.v1" } satisfies AppInfo;
    case "pick_archive":
      return DEMO_ARCHIVE;
    case "pick_json_save":
      return "D:/示例/Stukov2." + target + ".keys.json";
    case "pick_json_open":
      return "D:/示例/Stukov2." + target + ".keys.json";
    case "pick_archive_save":
      return DEMO_ARCHIVE.replace(".SC2Map", "." + target + ".SC2Map");
    case "default_output_name":
      return "Stukov2." + target + ".SC2Map";
    case "default_json_name":
      return "Stukov2." + target + ".keys.json";
    case "audit":
      return demoAudit();
    case "extract":
      return demoExtract(target);
    case "apply":
    case "seed":
      return demoApplyResult(cmd);
    case "inspect":
      return {
        path: DEMO_ARCHIVE,
        size: 902586,
        sha256: "demo",
        container: { format_version: 3, header_size: 208, block_size: 16384, components: 46, hash_entries: 64, blocks: 48, has_het: true, has_bet: true, has_attributes: true, header_md5_valid: true, raw_chunk_size: 0 },
        locales: [{ locale: "enUS", display: "英语（美国）", files: { "GameStrings.txt": { component: "enUS.SC2Data\\LocalizedData\\GameStrings.txt", bytes: 20480 } } }],
      } satisfies InspectReport;
    case "verify":
      return { path: DEMO_ARCHIVE, raw_chunk_size: 0, header_md5_valid: true, het_roundtrip_exact: true, bet_roundtrip_exact: true, het_vs_classic_mismatch: [], md5_ok: 49, md5_bad: 2, crc_ok: 50, crc_bad: 1, healthy: true, string_tables_roundtrip_exact: true } satisfies VerifyReport;
    case "reveal":
      return null;
    default:
      throw new Error("演示模式下没有实现这条命令: " + cmd);
  }
}
