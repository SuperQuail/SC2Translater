// 与 Rust 侧 (sc2trans-core) 的 JSON 结构一一对应。
// 权威定义在 crates/sc2trans-core/src/model.rs 与 ops.rs，改那边要同步改这里。

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

export interface Entry {
  key: string;
  file: string;
  source?: string;
  target?: string;
  status: string;
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
  by_file: Record<string, { player_facing: boolean; source_keys: number; locales: Array<{ locale: string; present: number; expected: number; coverage: number }> }>;
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

export interface AppInfo {
  name: string;
  version: string;
  tool: string;
  schema: string;
}
