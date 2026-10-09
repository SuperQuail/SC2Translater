// 与 Rust 侧 (sc2trans-core) 的 JSON 结构一一对应。
// 权威定义在 crates/sc2trans-core/src/model.rs 与 ops.rs；改那边要同步改这里。

export type Severity = "ok" | "info" | "warning";

export interface Diagnosis {
  severity: Severity;
  message: string;
  missing_in_target?: number;
  actionable?: string;
  worst?: string[];
  player_files?: string[];
}

export interface ArchiveInfo {
  path: string;
  size: number;
  sha256: string;
}

/** 单条翻译的状态。 */
export type EntryStatus = "missing" | "identical" | "translated" | "extra" | "no-source" | string;

export interface Entry {
  key: string;
  file: string;
  source?: string | null;
  target?: string | null;
  status: EntryStatus;
  origin?: string[];
}

export interface TranslationDocument {
  schema: string;
  tool: string;
  generated_at: string;
  archive: ArchiveInfo;
  source_locale: string;
  target_locale: string;
  files: string[];
  entries: Entry[];
  stats: {
    entries: number;
    by_status: Record<string, number>;
    by_file: Record<string, number>;
    translatable: number;
    locales_present?: string[];
  };
  diagnosis?: Diagnosis;
}

export interface AuditRow {
  locale: string;
  display: string;
  keys_expected: number;
  keys_present: number;
  keys_missing: number;
  keys_extra: number;
  coverage: number;
  files: string[];
}

export interface AuditReport {
  path: string;
  source_locale: string;
  source_keys: number;
  locales: AuditRow[];
  by_file: Record<
    string,
    { player_facing: boolean; source_keys: number; locales: Array<{ locale: string; present: number; expected: number; coverage: number }> }
  >;
  locales_absent: string[];
  diagnosis: Diagnosis;
}

export interface FileWriteInfo {
  component: string;
  added: number;
  updated: number;
  total: number;
}

export interface ApplyResult {
  ok: boolean;
  input: string;
  output: string;
  target_locale: string;
  source_locale?: string;
  components_written: Record<string, Record<string, FileWriteInfo>>;
  readback: Record<string, { ok: boolean; keys: number; expected: number }>;
  bytes_in: number;
  bytes_out: number;
  seeded?: number;
  diagnosis_before?: Diagnosis;
}

export interface InspectReport {
  path: string;
  size: number;
  sha256: string;
  container: {
    format_version: number;
    header_size: number;
    block_size: number;
    components: number;
    hash_entries: number;
    blocks: number;
    has_het: boolean;
    has_bet: boolean;
    has_attributes: boolean;
    header_md5_valid: boolean;
    raw_chunk_size: number;
  };
  locales: Array<{ locale: string; display: string; files: Record<string, { component: string; bytes: number }> }>;
}

export interface VerifyReport {
  path: string;
  raw_chunk_size: number;
  header_md5_valid: boolean;
  het_roundtrip_exact: boolean;
  bet_roundtrip_exact: boolean;
  het_vs_classic_mismatch: string[];
  md5_ok: number;
  md5_bad: number;
  crc_ok: number;
  crc_bad: number;
  healthy: boolean;
  string_tables_roundtrip_exact: boolean;
}

// --------------------------------------------------------------------------- //
// 项目模型（与 Rust 侧 sc2trans-core/src/project.rs 一一对应）
// --------------------------------------------------------------------------- //

export interface FileStats {
  entries: number;
  missing: number;
  identical: number;
  translated: number;
  locales_present: number;
  size: number;
  scanned_at: string;
}

export interface ProjectFile {
  path: string;
  label: string;
  source: string;
  target: string;
  output?: string | null;
  stats?: FileStats | null;
}

export interface Project {
  format: number;
  name: string;
  files: ProjectFile[];
  active: number;
  created_at: string;
  updated_at: string;
  path?: string | null;
}

export interface RecentEntry {
  path: string;
  name: string;
  opened_at: string;
  files: number;
}

export interface RecentProjects {
  items: RecentEntry[];
}

// --------------------------------------------------------------------------- //
// 专有名词库 / 机翻 / 版本管理
// --------------------------------------------------------------------------- //

export type TermScope = "global" | "project";

export interface Term {
  id: string;
  source: string;
  target: string;
  note?: string;
  domain?: string;
  case_sensitive: boolean;
  scope: TermScope;
  updated_at?: string;
}

export interface Glossary {
  format: number;
  name: string;
  terms: Term[];
  updated_at?: string;
}

export interface GlossaryBundle {
  global: Glossary;
  project: Glossary;
  global_path: string;
  project_path?: string | null;
}

export type MtProvider = "offline" | "openai";

export interface MtConfig {
  provider: MtProvider;
  base_url: string;
  api_key: string;
  model: string;
  target_language: string;
  timeout_secs: number;
}

export interface MtSuggestion {
  text: string;
  provider: string;
  terms: Term[];
  note: string;
  cached: boolean;
}

export interface MtResult {
  ok: boolean;
  suggestion?: MtSuggestion;
  error?: string;
}

export type InstallSource = "installed" | "bundled" | "dev_path" | "missing";

export interface ToolStatus {
  id: string;
  name: string;
  installed: boolean;
  version?: string | null;
  path?: string | null;
  source: InstallSource;
  size_bytes: number;
  message?: string | null;
}

export interface VcsChange {
  index: string;
  worktree: string;
  name: string;
}

export interface VcsStatus {
  is_repo: boolean;
  branch: string;
  has_commits: boolean;
  changes: VcsChange[];
  staged: number;
  dirty: number;
  untracked: number;
  raw: string;
}

export interface VcsCommit {
  sha: string;
  short: string;
  author: string;
  date: string;
  message: string;
}

export interface VcsStatusBundle {
  repo: string;
  status: VcsStatus;
  tool: ToolStatus;
}

export interface VcsRefs {
  branches: string[];
  tags: string[];
}

// --------------------------------------------------------------------------- //
// 文件访问（类 VS Code）
// --------------------------------------------------------------------------- //

/** 打开一个文件时判出来的类型。 */
export type FileKind = "sc2" | "text" | "image" | "binary";

export interface FileEntry {
  /** 相对工程根的路径。 */
  path: string;
  kind: FileKind;
  size: number;
  /** sc2 文件关联上的同名 JSON（没关联上就是 null）。 */
  linked_json?: string | null;
}

export interface OpenFile {
  path: string;
  kind: FileKind;
  size: number;
  /** text 才有：文件内容。 */
  text?: string;
  /** 打不开时的原因（binary / 太大 / 解码失败）。 */
  reason?: string;
  /** 文本被截断了（超过上限）。 */
  truncated?: boolean;
  /** 能直接喂给 <img src> 的地址（桌面壳里由 asset 协议转出来）。 */
  url?: string;
}

export interface AppInfo {
  name: string;
  version: string;
  tool: string;
  schema: string;
}
