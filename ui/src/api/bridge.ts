// 前后端桥接。
//
// 在 Tauri 里走 invoke；在浏览器里（pnpm dev，没有 Rust 后端）自动降级到**演示数据**，
// 这样调 UI 不需要每次都起桌面壳。

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
// 演示模式：造一份够大的样本，让表格、过滤、编辑都能在浏览器里真跑一遍
// --------------------------------------------------------------------------- //

/** 演示模式下的示例归档路径；浏览器里加 ?demo=1 可以直接进工作区。 */
export const DEMO_ARCHIVE = "D:/示例/Stukov2.SC2Map";

const DEMO_ROWS: Array<[string, string, string, string]> = [
  // [file, key, source, 已有译文或空]
  ["GameStrings.txt", "DocInfo/Name", "Stukov's Gambit", ""],
  ["GameStrings.txt", "DocInfo/Author", "Blizzard Entertainment", "暴雪娱乐"],
  ["GameStrings.txt", "DocInfo/DescShort", "Stukov02", ""],
  ["GameStrings.txt", "DocInfo/DescLong", "After the fall of the UED expeditionary force, Stukov's fate remained unknown.", ""],
  ["GameStrings.txt", "MapInfo/Player01/Name", "Stukov's Forces", ""],
  ["GameStrings.txt", "MapInfo/Player02/Name", "Feral Zerg", "野生异虫"],
  ["GameStrings.txt", "MapInfo/Player03/Name", "Infested Terrans", ""],
  ["GameStrings.txt", "MapInfo/Player09/Name", "Hostile", "敌对"],
  ["GameStrings.txt", "LoadingScreen/Title", "<s val=\"StukovIntroTitle\">THE DEVOURING ONE</s>", ""],
  ["GameStrings.txt", "LoadingScreen/TextBody", "<s val=\"StukovIntroBody\">Deep within the ruins of a shattered world, something ancient stirs.</s>", ""],
  ["GameStrings.txt", "LoadingScreen/Tip", "Infested Terrans can be healed by Medics.", ""],
  ["GameStrings.txt", "LoadingScreen/TipRestart", "You can hotkey your production buildings to rebuild your army quickly.", ""],
  ["GameStrings.txt", "Param/Value/0F5D0539", "Hive Destroyed", "主巢被摧毁"],
  ["GameStrings.txt", "Param/Value/16EFCB35", "Warp Conduit", "时空航道"],
  ["GameStrings.txt", "Param/Value/24A5289E", "Attack", ""],
  ["GameStrings.txt", "Param/Value/27E13E84", "Warp Conduit Activated", ""],
  ["GameStrings.txt", "Param/Value/28AB902F", "Attack", ""],
  ["GameStrings.txt", "Param/Value/2D28C849", "Attack", ""],
  ["GameStrings.txt", "Param/Value/39BE779F", "Warp Conduit", ""],
  ["GameStrings.txt", "Param/Value/3AC333C6", "Hive Destroyed", ""],
  ["GameStrings.txt", "Param/Value/40F7F007", "Zerg Units Destroyed", ""],
  ["GameStrings.txt", "Param/Value/42177F7F", "All of your warriors have been killed.", ""],
  ["GameStrings.txt", "Param/Value/5FD05402", "Hybrid", "混合体"],
  ["GameStrings.txt", "Param/Value/62C86722", "Hive Destroyed", ""],
  ["GameStrings.txt", "Param/Value/660AAA7D", "Conduit Activated", ""],
  ["GameStrings.txt", "Param/Value/8371F17B", "Zerg Hive", ""],
  ["GameStrings.txt", "Param/Value/8C90AFB4", "Hybrid", ""],
  ["GameStrings.txt", "Param/Value/A3827A78", "Percentage of Creep Removed", ""],
  ["GameStrings.txt", "Param/Value/A943C427", "Stukov's Vanguard", ""],
  ["GameStrings.txt", "Param/Value/B197DE66", "Press <c val=\"FFFFFF\">F2</c> to Select All Units in your Army", ""],
  ["GameStrings.txt", "Param/Expression/83371866", "~A~ units", ""],
  ["GameStrings.txt", "Param/Expression/F4535E0A", "~B~%", ""],
  ["ObjectStrings.txt", "Unit/EditorSuffix/Stukov", "Infested Admiral", ""],
  ["ObjectStrings.txt", "Unit/EditorSuffix/BroodLord", "Brood Lord", "巢虫领主"],
  ["ObjectStrings.txt", "Abil/EditorSuffix/DeepTunnel", "Deep Tunnel", ""],
  ["TriggerStrings.txt", "Category/Name/0CD515FF", "Main Objective - Destroy the Hives", ""],
  ["TriggerStrings.txt", "Category/Name/4B90E61C", "Intro Cinematic", ""],
  ["TriggerStrings.txt", "Category/Name/5CC867D1", "Debug", ""],
  ["TriggerStrings.txt", "FunctionDef/Grammar/27EDB7DC", "Spawn ~qty~ ~unitType~ from ~nydus~ for ~player~ and attack the player", ""],
];

function demoKeys(target: string): TranslationDocument {
  const entries = DEMO_ROWS.map(([file, key, source, target0]) => {
    const value = target0 || null;
    const status = value ? (value === source ? "identical" : "translated") : "missing";
    return { key, file, source, target: value, status, origin: ["enUS.SC2Data/" + file] };
  });
  const byStatus: Record<string, number> = {};
  const byFile: Record<string, number> = {};
  for (const e of entries) {
    byStatus[e.status] = (byStatus[e.status] ?? 0) + 1;
    byFile[e.file] = (byFile[e.file] ?? 0) + 1;
  }
  return {
    schema: "sc2trans.keys.v1",
    tool: "sc2trans (demo)",
    generated_at: "2026-10-09T00:00:00Z",
    archive: { path: DEMO_ARCHIVE, size: 902586, sha256: "demo" },
    source_locale: "enUS",
    target_locale: target,
    files: ["GameStrings.txt", "ObjectStrings.txt", "TriggerStrings.txt"],
    entries,
    stats: {
      entries: entries.length,
      by_status: byStatus,
      by_file: byFile,
      translatable: entries.filter((e) => e.status === "missing" || e.status === "identical").length,
      locales_present: ["enUS"],
    },
    diagnosis: {
      severity: "warning",
      message: "简体中文（zhCN）缺少 33 / 39 个键：游戏语言设为 zhCN 时这些文本会显示成原始键路径，而不是 enUS 里写的文本。",
      missing_in_target: 33,
      actionable: "先点「一键兜底」让游戏里至少有英文，再导出翻译键交给翻译者。",
    },
  };
}

function demoAudit(): AuditReport {
  return {
    path: DEMO_ARCHIVE,
    source_locale: "enUS",
    source_keys: 39,
    locales: [
      { locale: "enUS", display: "英语（美国）", keys_expected: 39, keys_present: 39, keys_missing: 0, keys_extra: 0, coverage: 1, files: ["GameStrings.txt", "ObjectStrings.txt", "TriggerStrings.txt"] },
      { locale: "zhCN", display: "简体中文", keys_expected: 39, keys_present: 6, keys_missing: 33, keys_extra: 0, coverage: 0.1538, files: ["GameStrings.txt"] },
    ],
    by_file: {
      "GameStrings.txt": { player_facing: true, source_keys: 32, locales: [{ locale: "enUS", present: 32, expected: 32, coverage: 1 }, { locale: "zhCN", present: 6, expected: 32, coverage: 0.1875 }] },
      "ObjectStrings.txt": { player_facing: true, source_keys: 3, locales: [{ locale: "enUS", present: 3, expected: 3, coverage: 1 }, { locale: "zhCN", present: 0, expected: 3, coverage: 0 }] },
      "TriggerStrings.txt": { player_facing: false, source_keys: 4, locales: [{ locale: "enUS", present: 4, expected: 4, coverage: 1 }] },
    },
    locales_absent: ["deDE", "jaJP", "zhTW"],
    diagnosis: {
      severity: "warning",
      message: "以下语言的**玩家可见**文本覆盖率不足：zhCN 15.4%。用这些语言进入游戏时，未覆盖的键会显示成原始键路径，而不是文本。",
      worst: ["zhCN"],
      player_files: ["GameStrings.txt", "ObjectStrings.txt"],
    },
  };
}

function demoApply(mode: string): ApplyResult {
  return {
    ok: true,
    input: DEMO_ARCHIVE,
    output: DEMO_ARCHIVE.replace(".SC2Map", ".zhCN.SC2Map"),
    target_locale: "zhCN",
    source_locale: "enUS",
    components_written: {
      zhCN: {
        "GameStrings.txt": { component: "zhCN.SC2Data\\LocalizedData\\GameStrings.txt", added: 26, updated: 6, total: 32 },
        "ObjectStrings.txt": { component: "zhCN.SC2Data\\LocalizedData\\ObjectStrings.txt", added: 3, updated: 0, total: 3 },
      },
    },
    readback: { "GameStrings.txt": { ok: true, keys: 32, expected: 32 }, "ObjectStrings.txt": { ok: true, keys: 3, expected: 3 } },
    bytes_in: 902586,
    bytes_out: 878025,
    seeded: mode === "seed" ? 33 : undefined,
  };
}

function demoInvoke(cmd: string, args?: Record<string, unknown>): unknown {
  const target = (args?.target as string) ?? "zhCN";
  const archive = (args?.archive as string) ?? DEMO_ARCHIVE;
  switch (cmd) {
    case "app_info":
      return { name: "SC2Translater", version: "0.1.0", tool: "sc2trans 0.1.0 (demo)", schema: "sc2trans.keys.v1" } satisfies AppInfo;
    case "pick_archive":
      return DEMO_ARCHIVE;
    case "pick_archives":
      return [DEMO_ARCHIVE, DEMO_ARCHIVE.replace("Stukov2", "Stukov3"), DEMO_ARCHIVE.replace("Stukov2", "ZergX03")];
    case "recent_projects":
      return {
        items: [
          { path: "D:/示例/斯托科夫战役.sc2transproj", name: "斯托科夫战役", opened_at: "2026-10-09T00:00:00Z", files: 3 },
          { path: "D:/示例/星际枢纽汉化.sc2transproj", name: "星际枢纽汉化", opened_at: "2026-10-08T00:00:00Z", files: 12 },
        ],
      };
    case "forget_project":
      return null;
    case "pick_project_save":
      return "D:/示例/未命名项目.sc2transproj";
    case "pick_project_open":
      return "D:/示例/斯托科夫战役.sc2transproj";
    case "save_project":
      return { ...(args?.project as object), path: args?.path };
    case "load_project":
      return {
        format: 1,
        name: "斯托科夫战役",
        files: [
          { path: DEMO_ARCHIVE, label: "Stukov2 关卡", source: "enUS", target: "zhCN", stats: { entries: 677, missing: 404, identical: 0, translated: 273, locales_present: 1, size: 902586, scanned_at: "2026-10-09T00:00:00Z" } },
          { path: DEMO_ARCHIVE.replace("Stukov2", "Stukov3"), label: "Stukov3 关卡", source: "enUS", target: "zhCN", stats: { entries: 512, missing: 512, identical: 0, translated: 0, locales_present: 1, size: 810240, scanned_at: "2026-10-09T00:00:00Z" } },
          { path: DEMO_ARCHIVE.replace("Stukov2", "ZergX03"), label: "ZergX03", source: "enUS", target: "zhTW", stats: { entries: 288, missing: 40, identical: 8, translated: 240, locales_present: 2, size: 1261142, scanned_at: "2026-10-09T00:00:00Z" } },
        ],
        active: 0,
        created_at: "2026-10-08T00:00:00Z",
        updated_at: "2026-10-09T00:00:00Z",
        path: "D:/示例/斯托科夫战役.sc2transproj",
      };
    case "pick_json_save":
      return "D:/示例/Stukov2." + target + ".keys.json";
    case "pick_json_open":
      return "D:/示例/Stukov2." + target + ".keys.json";
    case "pick_archive_save":
      return archive.replace(".SC2Map", "." + target + ".SC2Map");
    case "default_output_name":
      return "Stukov2." + target + ".SC2Map";
    case "default_json_name":
      return "Stukov2." + target + ".keys.json";
    case "load_keys":
      return demoKeys(target);
    case "audit":
      return demoAudit();
    case "apply_document":
    case "seed":
      return demoApply(cmd);
    case "fill_locales": {
      const base = demoApply("seed");
      const codes = String(args?.locales ?? "all");
      const list =
        codes === "all"
          ? ["deDE", "esES", "esMX", "frFR", "itIT", "koKR", "plPL", "ptBR", "ruRU", "zhTW"]
          : codes.split(",");
      base.target_locale = list.length === 1 ? list[0] : "all";
      base.components_written = {};
      for (const code of list) {
        base.components_written[code] = {
          "GameStrings.txt": { component: code + ".SC2Data\\LocalizedData\\GameStrings.txt", added: 32, updated: 0, total: 32 },
          "ObjectStrings.txt": { component: code + ".SC2Data\\LocalizedData\\ObjectStrings.txt", added: 3, updated: 0, total: 3 },
        };
      }
      base.seeded = list.length * 35;
      return base;
    }
    case "export_document":
      return { path: args?.out, bytes: 12345 };
    case "import_document":
      // 演示：返回一份「翻译者已经翻好一部分」的 JSON
      return {
        entries: [
          { file: "GameStrings.txt", key: "DocInfo/Name", target: "斯托科夫的赌局" },
          { file: "GameStrings.txt", key: "DocInfo/DescShort", target: "斯托科夫02" },
          { file: "GameStrings.txt", key: "MapInfo/Player01/Name", target: "斯托科夫的部队" },
          { file: "GameStrings.txt", key: "Param/Value/24A5289E", target: "攻击" },
          { file: "ObjectStrings.txt", key: "Unit/EditorSuffix/Stukov", target: "被感染的舰队上将" },
        ],
      };
    case "inspect":
      return {
        path: archive,
        size: 902586,
        sha256: "demo",
        container: { format_version: 3, header_size: 208, block_size: 16384, components: 46, hash_entries: 64, blocks: 48, has_het: true, has_bet: true, has_attributes: true, header_md5_valid: true, raw_chunk_size: 0 },
        locales: [{ locale: "enUS", display: "英语（美国）", files: { "GameStrings.txt": { component: "enUS.SC2Data\\LocalizedData\\GameStrings.txt", bytes: 2048 } } }],
      } satisfies InspectReport;
    case "verify":
      return { path: archive, raw_chunk_size: 0, header_md5_valid: true, het_roundtrip_exact: true, bet_roundtrip_exact: true, het_vs_classic_mismatch: [], md5_ok: 49, md5_bad: 2, crc_ok: 50, crc_bad: 1, healthy: true, string_tables_roundtrip_exact: true } satisfies VerifyReport;
    case "reveal":
      return null;
    default:
      throw new Error("演示模式下没有实现这条命令: " + cmd);
  }
}
