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

/** 演示用的工程根（工作区模式）。 */
export const DEMO_FOLDER = "D:/示例/斯托科夫战役";

/** 最近打开的文件夹（照参考图的规模：十来条、长短路径混合、有一条失效）。 */
const DEMO_RECENTS = [
  { path: "D:/Code/SC2/斯托科夫战役", name: "斯托科夫战役", opened_at: "2026-10-09T10:00:00Z", files: 3 },
  { path: "D:/Code/SC2/星际枢纽汉化", name: "星际枢纽汉化", opened_at: "2026-10-09T09:00:00Z", files: 12 },
  { path: "D:/Code/SC2/SCMR 复刻战役", name: "SCMR 复刻战役", opened_at: "2026-10-08T18:00:00Z", files: 46 },
  { path: "D:/Code/SC2/CoopCommanderPatch", name: "CoopCommanderPatch", opened_at: "2026-10-08T12:00:00Z", files: 8 },
  { path: "D:/Game/BLZ/StarCraft II/Maps/Campaign/swarm", name: "swarm", opened_at: "2026-10-07T20:00:00Z", files: 27 },
  { path: "D:/Code/SC2/Miya_Craft", name: "Miya_Craft", opened_at: "2026-10-06T15:00:00Z", files: 5 },
  { path: "D:/Code/SC2/TestCode_01", name: "TestCode_01", opened_at: "2026-10-05T11:00:00Z", files: 2 },
  { path: "D:/Code/SC2/MDK-1.21.1-NeoGradle-main", name: "MDK-1.21.1-NeoGradle-main", opened_at: "2026-10-04T09:00:00Z", files: 0 },
  { path: "D:/Code/SC2/ExampleMod-master (1)", name: "ExampleMod-master (1)", opened_at: "2026-10-03T09:00:00Z", files: 0 },
  { path: "E:/外接盘/PythonInstallJava", name: "PythonInstallJava", opened_at: "2026-10-02T09:00:00Z", files: -1 },
];

/** 演示工程目录：混着放 SC2 文档、文本、图片和打不开的二进制。 */
const DEMO_TREE = [
  { path: "Maps/Stukov2.SC2Map", kind: "sc2" as const, size: 902586, linked_json: "Maps/Stukov2.json" },
  { path: "Maps/Stukov2.json", kind: "text" as const, size: 22011, linked_json: null },
  { path: "Maps/Stukov3.SC2Map", kind: "sc2" as const, size: 1358082, linked_json: null },
  { path: "Maps/说明.txt", kind: "text" as const, size: 812, linked_json: null },
  { path: "Mods/SCMR.SC2Mod", kind: "sc2" as const, size: 44892160, linked_json: null },
  { path: "Refs/封面.png", kind: "image" as const, size: 402123, linked_json: null, preview: "/demo-ref.png" },
  { path: "Refs/字体说明.md", kind: "text" as const, size: 1204, linked_json: null },
  { path: "GameData/UnitData.xml", kind: "text" as const, size: 55210, linked_json: null },
  { path: "Assets/model.m3", kind: "binary" as const, size: 2201984, linked_json: null },
  { path: "Assets/voice.ogg", kind: "binary" as const, size: 884320, linked_json: null },
];

/** 演示文本内容。 */
const DEMO_TEXT: Record<string, string> = {
  "Maps/说明.txt": [
    "斯托科夫战役 汉化说明",
    "",
    "1. 先跑 audit 看每种语言缺多少键；",
    "2. 用 seed 把原文补进 zhCN，至少不再显示键路径；",
    "3. 译文写好后用 apply 写回，产物不要覆盖原件。",
    "",
    "注意：GameStrings.txt 里的 <n/> 是换行标记，改动时别弄丢。",
  ].join("\n"),
  "Maps/Stukov2.json": [
    "{",
    "  \"schema\": \"sc2trans.keys.v1\",",
    "  \"source_locale\": \"enUS\",",
    "  \"target_locale\": \"zhCN\",",
    "  \"entries\": [",
    "    { \"key\": \"DocInfo/Name\", \"source\": \"Stukov's Gambit\", \"target\": \"\", \"status\": \"missing\" },",
    "    { \"key\": \"Param/Value/24A5289E\", \"source\": \"Attack\", \"target\": \"\", \"status\": \"missing\" }",
    "  ]",
    "}",
  ].join("\n"),
  "Refs/字体说明.md": "# 字体\n\n正文用思源黑体，标题用思源宋体。\n",
  "GameData/UnitData.xml": "<Catalog>\n  <CUnit id=\"Marine\" parent=\"Marine\"/>\n</Catalog>\n",
};

/** 演示用的术语：覆盖面尽量像真的（含中英混排、简称、大小写敏感）。 */
const DEMO_TERMS = [
  { id: "t1", source: "Infested Terran", target: "被感染的陆战队员", note: "SC2 官方译法", domain: "单位", case_sensitive: true, scope: "global" as const },
  { id: "t2", source: "Hive", target: "主巢", note: "", domain: "建筑", case_sensitive: true, scope: "global" as const },
  { id: "t3", source: "Warp Conduit", target: "时空航道", note: "", domain: "建筑", case_sensitive: true, scope: "global" as const },
  { id: "t4", source: "Hybrid", target: "混合体", note: "不要译成「混血」", domain: "单位", case_sensitive: false, scope: "global" as const },
  { id: "t5", source: "Brood Lord", target: "巢虫领主", note: "", domain: "单位", case_sensitive: true, scope: "global" as const },
  { id: "t6", source: "Stukov", target: "斯托科夫", note: "本作主角，务必统一", domain: "人物", case_sensitive: true, scope: "project" as const },
  { id: "t7", source: "Zerg", target: "异虫", note: "不是「虫族」", domain: "阵营", case_sensitive: true, scope: "project" as const },
];

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
    case "pick_image":
      return "D:/Pic/miku/1.png";
    case "scan_folder":
    case "workspace_scan":
      return DEMO_TREE;
    case "pick_folder":
      return DEMO_FOLDER;
    case "workspace_load":
      return {
        workspace: {
          format: 1,
          name: "斯托科夫战役",
          defaults: { source: "enUS", target: "zhCN" },
          files: [
            { path: "Maps/Stukov2.SC2Map", label: "Stukov2 关卡", source: "enUS", target: "zhCN", output: null, keys_json: "Maps/Stukov2.json" },
            { path: "Maps/Stukov3.SC2Map", label: "Stukov3 关卡", source: "enUS", target: "zhCN", output: null, keys_json: null },
            { path: "Mods/SCMR.SC2Mod", label: "SCMR", source: "enUS", target: "zhTW", output: null, keys_json: null },
          ],
          open: ["Maps/Stukov2.SC2Map"],
          active: 0,
          view: {},
          created_at: "2026-10-09T00:00:00Z",
          updated_at: "2026-10-09T00:00:00Z",
        },
        legacy_projects: [],
        stats: {},
      };
    case "workspace_save":
    case "workspace_save_stats":
      return { ok: true };
    case "workspace_migrate":
      return { workspace: { format: 1, name: "迁移后的工程", defaults: { source: "enUS", target: "zhCN" }, files: [], open: [], active: 0, view: {}, created_at: "", updated_at: "" }, outside: [] };
    case "recent_folders":
    case "recent_touch":
    case "forget_folder":
      return { items: DEMO_RECENTS };
    case "open_file": {
      const path = String(args?.path ?? "");
      const entry = DEMO_TREE.find((f) => f.path === path) ?? DEMO_TREE[0];
      if (entry.kind === "sc2") {
        return { path: entry.path, kind: "sc2", size: entry.size, linked_json: entry.linked_json ?? null };
      }
      if (entry.kind === "text") {
        return { path: entry.path, kind: "text", size: entry.size, text: DEMO_TEXT[entry.path] ?? "# " + entry.path + "\n（演示内容）\n" };
      }
      if (entry.kind === "image") {
        // 演示：给一个能直接加载的地址（真机上是 asset 协议转出来的）
        return { path: entry.path, kind: "image", size: entry.size, url: entry.preview ?? "" };
      }
      return {
        path: entry.path,
        kind: "binary",
        size: entry.size,
        reason: "这个文件不是 UTF-8 文本（前 512 字节里有 NUL 字节），大概是个二进制资源。本工具不打开这类文件。",
      };
    }
    case "pick_archives":
      return [DEMO_ARCHIVE, DEMO_ARCHIVE.replace("Stukov2", "Stukov3"), DEMO_ARCHIVE.replace("Stukov2", "ZergX03")];
    case "recent_projects":
      return { items: DEMO_RECENTS };
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

    // --- 专有名词库（演示数据） ---
    case "glossary_load":
      return {
        global: { format: 1, name: "全局术语库", terms: DEMO_TERMS.filter((t) => t.scope === "global") },
        project: { format: 1, name: "项目术语库", terms: DEMO_TERMS.filter((t) => t.scope === "project") },
        global_path: "C:/Users/you/AppData/Roaming/net.miyin.sc2trans/glossary.global.json",
        project_path: "D:/示例/斯托科夫战役.glossary.json",
      };
    case "glossary_save":
      return { path: "demo", terms: (args?.glossary as { terms?: unknown[] })?.terms?.length ?? 0 };
    case "glossary_pick":
      return args?.save ? "D:/示例/术语库导出.csv" : "D:/示例/术语库导入.csv";
    case "glossary_read":
      return { terms: DEMO_TERMS, count: DEMO_TERMS.length, format: String(args?.path ?? "").endsWith(".csv") ? "csv" : "json" };
    case "glossary_write":
      return { path: args?.path, terms: (args?.glossary as { terms?: unknown[] })?.terms?.length ?? 0 };

    // --- 机翻 ---
    case "mt_config_get":
      return { provider: "offline", base_url: "https://api.openai.com/v1", api_key: "", model: "gpt-4o-mini", target_language: "简体中文", timeout_secs: 30 };
    case "mt_config_set":
      return args?.config;
    case "mt_translate": {
      const src = String(args?.text ?? "");
      const hit = DEMO_TERMS.filter((t) => src.includes(t.source));
      let out = src;
      for (const t of [...hit].sort((a, b) => b.source.length - a.source.length)) {
        out = out.split(t.source).join(t.target);
      }
      return {
        text: out,
        provider: "offline",
        terms: hit,
        note: hit.length
          ? "离线模式：只替换了命中的 " + hit.length + " 个术语，其余保持英文 —— 术语是对的，语序还得你自己来"
          : "离线模式：这句话没有命中任何术语，下面是原文（想看真正的机翻请到「机翻设置」里配在线接口）",
        cached: false,
      };
    }
    case "mt_translate_many": {
      const texts = (args?.texts as string[]) ?? [];
      const results = texts.map((t) => {
        const hit = DEMO_TERMS.filter((x) => t.includes(x.source));
        let out = t;
        for (const term of [...hit].sort((a, b) => b.source.length - a.source.length)) out = out.split(term.source).join(term.target);
        return { ok: true, suggestion: { text: out, provider: "offline", terms: hit, note: "离线模式：术语拼装", cached: false } };
      });
      return { results };
    }
    case "mt_clear_cache":
      return { cleared: 12 };

    // --- 版本管理 ---
    case "vcs_tool":
      return { id: "sc2diff", name: "SC2Diff", installed: true, version: "0.1.0-a2", path: "C:/Users/you/AppData/Roaming/net.miyin.sc2trans/tools/sc2diff/sc2diff.exe", source: "bundled", size_bytes: 1413632, message: null };
    case "vcs_reinstall":
      return { id: "sc2diff", name: "SC2Diff", installed: true, version: "0.1.0-a2", path: "demo", source: "bundled", size_bytes: 1413632, message: null };
    case "vcs_repo_path":
      return "D:/示例/斯托科夫战役.repos/Stukov2";
    case "vcs_status":
      return {
        repo: "D:/示例/斯托科夫战役.repos/Stukov2",
        status: {
          is_repo: true,
          branch: "main",
          has_commits: true,
          changes: [
            { index: " ", worktree: "M", name: "Objects" },
            { index: "M", worktree: " ", name: "Triggers" },
            { index: "?", worktree: "?", name: "zhCN.SC2Data\\LocalizedData\\GameStrings.txt" },
          ],
          staged: 1,
          dirty: 1,
          untracked: 1,
          raw: "On branch main\n\nChanges to be committed:\n        modified: Triggers\n\nChanges not staged for commit:\n        modified: Objects\n\nUntracked files:\n        zhCN.SC2Data\\LocalizedData\\GameStrings.txt",
        },
        tool: { id: "sc2diff", name: "SC2Diff", installed: true, version: "0.1.0-a2", path: "demo", source: "bundled", size_bytes: 1413632, message: null },
      };
    case "vcs_init":
      return { output: "Initialized empty sc2diff repository\n  imported Stukov2.SC2Map\n  components: 46" };
    case "vcs_commit":
      return { staged: "add      Objects\n", commit: "[main 63e4df69] 汉化 zhCN\n  46 component(s) in tree" };
    case "vcs_log":
      return [
        { sha: "63e4df69b97bf3f0", short: "63e4df69", author: "sc2diff", date: "1791511789", message: "汉化 zhCN：补全 677 条" },
        { sha: "29645933b58c8a16", short: "29645933", author: "sc2diff", date: "1791511777", message: "import" },
      ];
    case "vcs_diff":
      return { text: "diff HEAD~1 HEAD\n  Objects\n    Param/Value/0F5D0539: \"Hive Destroyed\" -> \"主巢被摧毁\"\n  1 changed, 0 added, 0 removed" };
    case "vcs_refs":
      return { branches: ["main", "zhcn-wip"], tags: ["v0.1"] };
    case "vcs_ref_action":
      return { output: "Switched to branch " + String(args?.value ?? ""), command: "sc2diff -C <repo> " + String(args?.action ?? "") + " " + String(args?.value ?? "") };
    case "vcs_pack":
      return { output: "packed D:/示例/out.SC2Map\n  878025 bytes, 51 blocks", path: String(args?.out ?? "") };
    case "vcs_pick_pack":
      return "D:/示例/Stukov2.repacked.SC2Map";
    case "vcs_unpack":
      return { output: "unpacked 46 components" };
    case "vcs_reveal":
      return null;
    default:
      throw new Error("演示模式下没有实现这条命令: " + cmd);
  }
}
