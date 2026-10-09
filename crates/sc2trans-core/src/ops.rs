//! 业务动作：info / extract / audit / apply / seed / verify。
//!
//! 这里的函数签名与返回结构就是 CLI 与 GUI 的共同契约，也和 Python 调研原型逐字段对齐
//! （tests 里用同一批输入比较两边输出）。

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use sc2mpq::{Archive, Document};
use serde::Serialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::locales::{
    BASE, KNOWN_STRING_FILES, LOCALES, component_name, is_player_facing, locale_display,
    parse_string_component,
};
use crate::model::{
    ArchiveInfo, Diagnosis, Entry, STATUS_EXTRA, STATUS_IDENTICAL, STATUS_MISSING, STATUS_NO_SOURCE,
    STATUS_TRANSLATED, TranslationDocument, compute_stats, normalize_input,
};
use crate::strings::StringFile;
use crate::{CoreError, Result, SCHEMA_ID, TOOL_ID, timeutil};

// --------------------------------------------------------------------------- //
// 基础工具
// --------------------------------------------------------------------------- //

fn sha256_file(path: &Path) -> Result<String> {
    let data = std::fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&data);
    Ok(format!("{:x}", hasher.finalize()))
}

/// 列出归档里所有语言字符串表：locale -> 文件名 -> 组件名。
pub fn locale_tables(arc: &Archive) -> BTreeMap<String, BTreeMap<String, String>> {
    let mut out: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    for name in arc.names() {
        if let Some((locale, filename)) = parse_string_component(name) {
            out.entry(locale).or_default().insert(filename, name.clone());
        }
    }
    out
}

/// 没有 (listfile) 就枚举不出组件名。
///
/// MPQ 的 HET/BET 里**只存名字哈希、不存名字**，所以缺 (listfile) 时任何按名字的
/// 读取都做不了。这里提前给出可读的错误，而不是返回一个空的键列表让人困惑。
fn ensure_listfile(arc: &Archive, path: &Path) -> Result<()> {
    if arc.names().is_empty() {
        return Err(CoreError::Invalid(format!(
            "{} 里没有 (listfile)，枚举不出组件名（MPQ 的 HET/BET 只存哈希、不存名字）。\
             请先用银河编辑器打开并另存一次，再对本工具使用。",
            path.display()
        )));
    }
    Ok(())
}

fn load_tables(arc: &Archive, locale: &str) -> Result<BTreeMap<String, StringFile>> {
    let mut out = BTreeMap::new();
    for (filename, comp) in locale_tables(arc).get(locale).cloned().unwrap_or_default() {
        let raw = arc.read(&comp)?;
        out.insert(filename, StringFile::parse(&raw, &comp)?);
    }
    Ok(out)
}

// --------------------------------------------------------------------------- //
// info
// --------------------------------------------------------------------------- //

/// 归档概览：容器参数、组件数、存在的语言、字符串表清单。
pub fn inspect(path: &Path) -> Result<Value> {
    let arc = Archive::open(path)?;
    ensure_listfile(&arc, path)?;
    let tables = locale_tables(&arc);
    let mut locales = Vec::new();
    for (locale, files) in &tables {
        let mut file_info = serde_json::Map::new();
        for (filename, comp) in files {
            let bytes = arc.read(comp)?.len();
            file_info.insert(filename.clone(), json!({ "component": comp, "bytes": bytes }));
        }
        locales.push(json!({
            "locale": locale,
            "display": locale_display(locale),
            "files": Value::Object(file_info),
        }));
    }
    let v = arc.verify(false);
    Ok(json!({
        "path": arc.path.display().to_string(),
        "size": std::fs::metadata(path)?.len(),
        "sha256": sha256_file(path)?,
        "container": {
            "format_version": arc.format_version,
            "header_size": arc.header_size,
            "block_size": arc.block_size,
            "components": arc.component_names().len(),
            "hash_entries": arc.hash_count,
            "blocks": arc.block_count,
            "has_het": arc.het.is_some(),
            "has_bet": arc.bet.is_some(),
            "has_attributes": arc.attributes.is_some(),
            "header_md5_valid": v.header_md5_valid,
            "raw_chunk_size": arc.raw_chunk_size,
        },
        "locales": locales,
        "components": arc.names(),
    }))
}

// --------------------------------------------------------------------------- //
// extract
// --------------------------------------------------------------------------- //

/// 提取翻译键：以 source_locale 为原文，报告 target_locale 的覆盖情况。
pub fn extract(
    path: &Path,
    source_locale: &str,
    target_locale: &str,
    only_files: Option<Vec<String>>,
    include_all_locales: bool,
) -> Result<TranslationDocument> {
    let arc = Archive::open(path)?;
    ensure_listfile(&arc, path)?;
    let tables = locale_tables(&arc);
    let src_files = load_tables(&arc, source_locale)?;
    let base_files = load_tables(&arc, BASE)?;
    let tgt_files = load_tables(&arc, target_locale)?;

    let mut wanted: BTreeSet<String> = BTreeSet::new();
    match &only_files {
        Some(list) => wanted.extend(list.iter().cloned()),
        None => {
            wanted.extend(src_files.keys().cloned());
            wanted.extend(base_files.keys().cloned());
            wanted.extend(tgt_files.keys().cloned());
        }
    }

    let mut other_files: BTreeMap<String, BTreeMap<String, StringFile>> = BTreeMap::new();
    if include_all_locales {
        for locale in tables.keys() {
            if locale == source_locale || locale == target_locale || locale == BASE {
                continue;
            }
            other_files.insert(locale.clone(), load_tables(&arc, locale)?);
        }
    }

    let mut entries: Vec<Entry> = Vec::new();
    for filename in &wanted {
        let src = src_files.get(filename);
        let base = base_files.get(filename);
        let tgt = tgt_files.get(filename);

        let mut order: Vec<String> = Vec::new();
        let mut seen: BTreeSet<String> = BTreeSet::new();
        for table in [src, base, tgt].into_iter().flatten() {
            for key in table.keys() {
                if seen.insert(key.clone()) {
                    order.push(key);
                }
            }
        }
        for (_, loc_files) in other_files.iter() {
            if let Some(table) = loc_files.get(filename) {
                for key in table.keys() {
                    if seen.insert(key.clone()) {
                        order.push(key);
                    }
                }
            }
        }

        for key in order {
            let mut origin: Vec<String> = Vec::new();
            let mut source_value: Option<String> = None;
            for (locale, table) in [(source_locale, src), (BASE, base)] {
                let Some(table) = table else { continue };
                if let Some(p) = table.get(&key) {
                    origin.push(format!("{locale}.SC2Data/{filename}"));
                    if source_value.is_none() {
                        source_value = Some(p.value.clone());
                    }
                }
            }
            let target_value = tgt.and_then(|t| t.get(&key)).map(|p| p.value.clone());

            let status = match (&target_value, &source_value) {
                (None, None) => STATUS_NO_SOURCE,
                (None, Some(_)) => STATUS_MISSING,
                (Some(t), Some(s)) if t == s => STATUS_IDENTICAL,
                (Some(_), None) => STATUS_EXTRA,
                (Some(_), Some(_)) => STATUS_TRANSLATED,
            };

            entries.push(Entry {
                key,
                file: filename.clone(),
                source: source_value,
                target: target_value,
                status: status.to_string(),
                origin,
            });
        }
    }

    let present_locales: Vec<String> = tables.keys().cloned().collect();
    let stats = compute_stats(&entries, present_locales);
    let archive = ArchiveInfo {
        path: arc.path.display().to_string(),
        size: std::fs::metadata(path)?.len(),
        sha256: sha256_file(path)?,
    };
    let mut doc = TranslationDocument {
        schema: SCHEMA_ID.to_string(),
        tool: TOOL_ID.to_string(),
        generated_at: timeutil::now_rfc3339(),
        archive,
        source_locale: source_locale.to_string(),
        target_locale: target_locale.to_string(),
        files: wanted.into_iter().collect(),
        entries,
        stats,
        diagnosis: None,
    };
    doc.diagnosis = Some(diagnose_extract(&doc));
    Ok(doc)
}

fn diagnose_extract(doc: &TranslationDocument) -> Diagnosis {
    let missing = doc.stats.by_status.get(STATUS_MISSING).copied().unwrap_or(0)
        + doc.stats.by_status.get(STATUS_IDENTICAL).copied().unwrap_or(0);
    let target = &doc.target_locale;
    if doc.stats.entries == 0 {
        return Diagnosis {
            severity: "info".into(),
            message: "归档里没有找到任何本地化字符串表。".into(),
            ..Default::default()
        };
    }
    if missing == 0 {
        return Diagnosis {
            severity: "ok".into(),
            message: format!(
                "{target} 已覆盖全部 {} 个键，游戏内不会出现原始键路径。",
                doc.stats.entries
            ),
            ..Default::default()
        };
    }
    Diagnosis {
        severity: "warning".into(),
        message: format!(
            "{}（{}）缺少 {} / {} 个键：游戏语言设为 {} 时这些文本会显示成原始键路径，而不是 {} 里写的文本。",
            locale_display(target),
            target,
            missing,
            doc.stats.entries,
            target,
            doc.source_locale
        ),
        missing_in_target: Some(missing),
        actionable: Some(format!("sc2trans seed 或 sc2trans apply 可以把这些键补进 {target}。")),
        ..Default::default()
    }
}

// --------------------------------------------------------------------------- //
// audit
// --------------------------------------------------------------------------- //

/// 对归档里**每个已存在**的语言算一遍覆盖率，用于回答「为什么显示成路径」。
pub fn audit(path: &Path, source_locale: &str) -> Result<Value> {
    let arc = Archive::open(path)?;
    ensure_listfile(&arc, path)?;
    let tables = locale_tables(&arc);

    // 按**文件**分别统计：GameStrings / ObjectStrings 是玩家看得见的文本，
    // TriggerStrings 只喂给银河编辑器的触发器界面。混在一起算会误导人
    // —— 实测暴雪官方地图的非英文语言目录里根本没有 TriggerStrings，那不是缺陷。
    let mut source_by_file: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for locale in [source_locale, BASE] {
        for (filename, comp) in tables.get(locale).cloned().unwrap_or_default() {
            let table = StringFile::parse(&arc.read(&comp)?, &comp)?;
            source_by_file.entry(filename).or_default().extend(table.keys());
        }
    }
    let mut source_keys: BTreeSet<String> = BTreeSet::new();
    for keys in source_by_file.values() {
        source_keys.extend(keys.iter().cloned());
    }

    let mut locale_rows = Vec::new();
    for locale in tables.keys() {
        if locale == BASE {
            continue;
        }
        let mut present_keys: BTreeSet<String> = BTreeSet::new();
        for comp in tables[locale].values() {
            present_keys.extend(StringFile::parse(&arc.read(comp)?, comp)?.keys());
        }
        let expected = source_keys.len();
        let present = source_keys.intersection(&present_keys).count();
        let coverage = if expected == 0 { 0.0 } else { present as f64 / expected as f64 };
        locale_rows.push(json!({
            "locale": locale,
            "display": locale_display(locale),
            "keys_expected": expected,
            "keys_present": present,
            "keys_missing": expected - present,
            "keys_extra": present_keys.difference(&source_keys).count(),
            "coverage": (coverage * 10_000.0).round() / 10_000.0,
            "files": tables[locale].keys().cloned().collect::<Vec<_>>(),
        }));
    }

    let mut by_file = serde_json::Map::new();
    for (filename, keys) in &source_by_file {
        let mut rows = Vec::new();
        for locale in tables.keys() {
            if locale == BASE {
                continue;
            }
            let present = match tables[locale].get(filename) {
                Some(comp) => keys
                    .intersection(&StringFile::parse(&arc.read(comp)?, comp)?.key_set())
                    .count(),
                None => 0,
            };
            let coverage = if keys.is_empty() { 0.0 } else { present as f64 / keys.len() as f64 };
            rows.push(json!({
                "locale": locale,
                "present": present,
                "expected": keys.len(),
                "coverage": (coverage * 10_000.0).round() / 10_000.0,
            }));
        }
        by_file.insert(
            filename.clone(),
            json!({
                "player_facing": is_player_facing(filename),
                "source_keys": keys.len(),
                "locales": rows,
            }),
        );
    }

    let absent: Vec<&str> = LOCALES.iter().copied().filter(|l| !tables.contains_key(*l)).collect();
    Ok(json!({
        "path": arc.path.display().to_string(),
        "source_locale": source_locale,
        "source_keys": source_keys.len(),
        "locales": locale_rows,
        "by_file": Value::Object(by_file),
        "locales_absent": absent,
        "diagnosis": diagnose_audit(&locale_rows, &tables, source_locale),
    }))
}

fn diagnose_audit(
    rows: &[Value],
    tables: &BTreeMap<String, BTreeMap<String, String>>,
    source_locale: &str,
) -> Value {
    if rows.is_empty() {
        return json!({ "severity": "info", "message": "归档里没有任何 <locale>.SC2Data 语言目录。" });
    }
    // 归档里只有源语言一种 —— 这是最常见、也最容易被忽略的情形：
    // 它不会在「已存在的语言」里露出任何问题，但游戏切成别的语言时全是键路径。
    let has_other_locale = rows
        .iter()
        .any(|r| r.get("locale").and_then(|v| v.as_str()) != Some(source_locale));
    if !has_other_locale {
        return json!({
            "severity": "warning",
            "message": format!(
                "归档里只有 {source_locale} 一种语言：游戏语言切成其它任何一种时，地图里所有文本都会显示成原始键路径。用「补全语言」可以一次把其余语言补上。"
            ),
        });
    }

    // 只看**面向玩家**的表
    let player_files: Vec<String> = tables
        .values()
        .flat_map(|m| m.keys().cloned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .filter(|f| is_player_facing(f))
        .collect();
    if player_files.is_empty() {
        return json!({
            "severity": "info",
            "message": "归档里只有编辑器专用字符串表（TriggerStrings.txt 等），玩家在游戏里不会看到它们。",
        });
    }

    let mut worst: Vec<(String, f64)> = Vec::new();
    for row in rows {
        let coverage = row.get("coverage").and_then(|v| v.as_f64()).unwrap_or(1.0);
        let expected = row.get("keys_expected").and_then(|v| v.as_u64()).unwrap_or(0);
        if expected > 0 && coverage < 1.0 {
            let locale = row.get("locale").and_then(|v| v.as_str()).unwrap_or("").to_string();
            worst.push((locale, coverage));
        }
    }
    if worst.is_empty() {
        return json!({
            "severity": "ok",
            "message": format!("所有已存在的语言都完整覆盖了 {source_locale} 的玩家可见文本。"),
        });
    }
    worst.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    let parts: Vec<String> = worst
        .iter()
        .take(4)
        .map(|(l, c)| format!("{l} {:.1}%", c * 100.0))
        .collect();
    json!({
        "severity": "warning",
        "message": format!(
            "以下语言的**玩家可见**文本覆盖率不足：{}。用这些语言进入游戏时，未覆盖的键会显示成原始键路径，而不是文本。",
            parts.join("、")
        ),
        "worst": worst.iter().take(4).map(|(l, _)| l.clone()).collect::<Vec<_>>(),
        "player_files": player_files,
    })
}

// --------------------------------------------------------------------------- //
// apply / seed
// --------------------------------------------------------------------------- //

/// apply 的选项。
#[derive(Debug, Clone)]
pub struct ApplyOptions {
    pub out: Option<PathBuf>,
    pub target_locale: Option<String>,
    pub fill_missing: bool,
    pub with_original: bool,
    pub sort: bool,
    pub default_file: String,
    pub force: bool,
}

impl Default for ApplyOptions {
    fn default() -> Self {
        Self {
            out: None,
            target_locale: None,
            fill_missing: false,
            with_original: false,
            sort: false,
            default_file: KNOWN_STRING_FILES[0].to_string(),
            force: false,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct FileWriteInfo {
    pub component: String,
    pub added: usize,
    pub updated: usize,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContainerInfo {
    pub bytes: usize,
    pub entries: usize,
    pub blocks: usize,
    pub hash_entries: u32,
    pub het_bytes: usize,
    pub bet_bytes: usize,
    pub block_size: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReadbackInfo {
    pub ok: bool,
    pub keys: usize,
    pub expected: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApplyResult {
    pub ok: bool,
    pub input: String,
    pub output: String,
    pub target_locale: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_locale: Option<String>,
    pub components_written: BTreeMap<String, BTreeMap<String, FileWriteInfo>>,
    pub container: ContainerInfo,
    pub readback: BTreeMap<String, ReadbackInfo>,
    pub bytes_in: u64,
    pub bytes_out: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seeded: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagnosis_before: Option<Diagnosis>,
}

fn target_path(src: &Path, out: Option<&Path>, target: &str) -> PathBuf {
    if let Some(o) = out {
        return o.to_path_buf();
    }
    let stem = src.file_stem().and_then(|s| s.to_str()).unwrap_or("out");
    let ext = src.extension().and_then(|s| s.to_str()).unwrap_or("SC2Map");
    let name = format!("{stem}.{target}.{ext}");
    match src.parent() {
        Some(p) => p.join(name),
        None => PathBuf::from(name),
    }
}

/// 把翻译 JSON 写回归档。
///
/// 只依赖 (file, key, target) 三个字段；target 为空且 fill_missing 时用 source 顶上
/// —— 这就是「先填英文，至少不再显示键路径」的那一步。
pub fn apply(path: &Path, raw: &Value, opts: &ApplyOptions) -> Result<ApplyResult> {
    let normalized = normalize_input(raw, &opts.default_file)?;
    let target = opts
        .target_locale
        .clone()
        .or(normalized.target_locale.clone())
        .ok_or_else(|| CoreError::Invalid("必须指定目标语言：JSON 里没有 target_locale，命令行也没给 --target".into()))?;
    if !crate::locales::locale_code_ok(&target) {
        return Err(CoreError::Invalid(format!("语言代码形状不对: {target}")));
    }

    let dest = target_path(path, opts.out.as_deref(), &target);
    if same_file(path, &dest) && !opts.force {
        return Err(CoreError::Invalid(
            "输出路径与输入相同，会覆盖原文件；确认请加 --force".into(),
        ));
    }

    // 新建目标文件时，用它源语言对应文件的编码习惯（BOM / 换行）作模板
    let src_locale = normalized.source_locale.clone().unwrap_or_else(|| "enUS".to_string());
    let reader = Archive::open(path)?;
    ensure_listfile(&reader, path)?;
    let src_templates = load_tables(&reader, &src_locale)?;

    // 按文件分组，只保留真正要写的条目
    let mut grouped: BTreeMap<String, Vec<(String, String, Option<String>)>> = BTreeMap::new();
    for e in &normalized.entries {
        let mut value = e.target.clone().filter(|v| !v.is_empty());
        if value.is_none() {
            if !opts.fill_missing {
                continue;
            }
            value = e.source.clone().filter(|v| !v.is_empty());
        }
        let Some(value) = value else { continue };
        grouped.entry(e.file.clone()).or_default().push((e.key.clone(), value, e.source.clone()));
    }

    let mut doc = Document::open(path)?;
    let mut written: BTreeMap<String, BTreeMap<String, FileWriteInfo>> = BTreeMap::new();
    for (filename, items) in &grouped {
        let comp = component_name(&target, filename);
        let mut table = match doc.get(&comp) {
            Some(raw_bytes) => StringFile::parse(raw_bytes, &comp)?,
            None => match src_templates.get(filename) {
                Some(t) => StringFile {
                    records: Vec::new(),
                    bom: t.bom,
                    newline: t.newline,
                    trailing_newline: t.trailing_newline,
                    source_name: comp.clone(),
                },
                None => StringFile { source_name: comp.clone(), ..Default::default() },
            },
        };
        let before = table.key_set();
        for (key, value, source) in items {
            if opts.with_original {
                table.set_full(key, value, source.as_deref());
            } else {
                table.set(key, value);
            }
        }
        if opts.sort {
            table.sort();
        }
        let after = table.key_set();
        let info = FileWriteInfo {
            component: comp.clone(),
            added: after.difference(&before).count(),
            updated: after.intersection(&before).count(),
            total: after.len(),
        };
        doc.set(&comp, table.to_bytes());
        written.entry(target.clone()).or_default().insert(filename.clone(), info);
    }

    let info = doc.save(&dest)?;

    // 回读校验：写出去的归档必须能被自己读回来
    let check = Archive::open(&dest)?;
    let mut readback: BTreeMap<String, ReadbackInfo> = BTreeMap::new();
    for (filename, w) in written.get(&target).cloned().unwrap_or_default() {
        let wanted: BTreeSet<String> =
            grouped.get(&filename).map(|v| v.iter().map(|(k, _, _)| k.clone()).collect()).unwrap_or_default();
        match check.try_read(&w.component) {
            Some(bytes) => {
                let table = StringFile::parse(&bytes, &w.component)?;
                let got = wanted.iter().filter(|k| table.contains(k)).count();
                readback.insert(
                    filename,
                    ReadbackInfo { ok: got == wanted.len(), keys: got, expected: wanted.len() },
                );
            }
            None => {
                readback.insert(
                    filename,
                    ReadbackInfo { ok: false, keys: 0, expected: wanted.len() },
                );
            }
        }
    }
    let ok = !readback.is_empty() && readback.values().all(|r| r.ok);

    Ok(ApplyResult {
        ok,
        input: path.display().to_string(),
        output: dest.display().to_string(),
        target_locale: target,
        source_locale: normalized.source_locale.clone(),
        components_written: written,
        container: ContainerInfo {
            bytes: info.bytes,
            entries: info.entries,
            blocks: info.blocks,
            hash_entries: info.hash_entries,
            het_bytes: info.het_bytes,
            bet_bytes: info.bet_bytes,
            block_size: info.block_size,
        },
        readback,
        bytes_in: std::fs::metadata(path)?.len(),
        bytes_out: std::fs::metadata(&dest)?.len(),
        seeded: None,
        diagnosis_before: None,
    })
}

fn same_file(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(x), Ok(y)) => x == y,
        _ => a.to_path_buf() == b.to_path_buf(),
    }
}

/// 「补全默认语言」的作用范围。
#[derive(Debug, Clone)]
pub enum FillTargets {
    /// 全部已知语言（除源语言外），外加归档里出现过但不在官方 12 语言里的代码。
    All,
    /// 只补这几个语言。
    Explicit(Vec<String>),
}

impl FillTargets {
    /// 从命令行字符串解析：all，或逗号分隔的语言列表。
    pub fn parse(spec: &str) -> Result<Self> {
        let trimmed = spec.trim();
        if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("all") {
            return Ok(FillTargets::All);
        }
        let list: Vec<String> = trimmed
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        for code in &list {
            if !crate::locales::locale_code_ok(code) {
                return Err(CoreError::Invalid(format!("语言代码形状不对（应为 2~4 个字母）: {code}")));
            }
        }
        if list.is_empty() {
            return Err(CoreError::Invalid("没有解析出任何语言代码".into()));
        }
        Ok(FillTargets::Explicit(list))
    }
}

/// **补全默认语言**：用指定语言的文本，去覆盖那些「没有语言文件」的语言。
///
/// 这是本工具解决「键显示成路径」最直接的一招：一张只带 enUS 的地图，在其余
/// 11 种语言下全都会露出键路径 —— 这个操作一次把它们全部补上。
///
/// * 范围选 All 时补全部语言（源语言除外），包含归档里出现过的非官方代码；
/// * only_missing = true（默认）时**不覆盖已有译文**，只补缺的键；
///   设为 false 才是「用源语言整份覆盖」。
pub fn fill_locales(
    path: &Path,
    source_locale: &str,
    targets: &FillTargets,
    only_missing: bool,
    with_original: bool,
    out: Option<&Path>,
    force: bool,
) -> Result<ApplyResult> {
    let reader = Archive::open(path)?;
    ensure_listfile(&reader, path)?;
    let tables = locale_tables(&reader);
    let src = load_tables(&reader, source_locale)?;
    if src.is_empty() {
        return Err(CoreError::Invalid(format!(
            "{} 里没有 {} 的字符串表，无法作为填充来源",
            path.display(),
            source_locale
        )));
    }
    let base = load_tables(&reader, BASE)?;

    let target_list: Vec<String> = match targets {
        FillTargets::All => {
            let mut list: Vec<String> = LOCALES
                .iter()
                .filter(|l| !l.eq_ignore_ascii_case(source_locale))
                .map(|l| l.to_string())
                .collect();
            // 归档里出现过的非官方语言代码也一并补上（实测有 zhCH / enGB 之类）
            for code in tables.keys() {
                if code != BASE
                    && !code.eq_ignore_ascii_case(source_locale)
                    && !list.iter().any(|l| l.eq_ignore_ascii_case(code))
                {
                    list.push(code.clone());
                }
            }
            list
        }
        FillTargets::Explicit(v) => v.clone(),
    };
    if target_list.is_empty() {
        return Err(CoreError::Invalid("没有要补的目标语言".into()));
    }

    let label = if target_list.len() == 1 { target_list[0].as_str() } else { "all" };
    let dest = target_path(path, out, label);
    if same_file(path, &dest) && !force {
        return Err(CoreError::Invalid(
            "输出路径与输入相同，会覆盖原文件；确认请加 --force".into(),
        ));
    }

    let mut doc = Document::open(path)?;
    let mut written: BTreeMap<String, BTreeMap<String, FileWriteInfo>> = BTreeMap::new();
    let mut touched: BTreeMap<String, BTreeMap<String, BTreeSet<String>>> = BTreeMap::new();

    for target in &target_list {
        for (filename, source_table) in &src {
            let comp = component_name(target, filename);
            let mut table = match doc.get(&comp) {
                Some(bytes) => StringFile::parse(bytes, &comp)?,
                None => StringFile {
                    records: Vec::new(),
                    bom: source_table.bom,
                    newline: source_table.newline,
                    trailing_newline: source_table.trailing_newline,
                    source_name: comp.clone(),
                },
            };
            let before = table.key_set();
            let mut wanted: BTreeSet<String> = BTreeSet::new();
            for pair in source_table.pairs() {
                if pair.value.is_empty() {
                    continue; // 源语言里是空值就跳过，免得把空串铺满
                }
                if only_missing && table.contains(&pair.key) {
                    continue;
                }
                if with_original {
                    table.set_full(&pair.key, &pair.value, Some(pair.value.as_str()));
                } else {
                    table.set(&pair.key, &pair.value);
                }
                wanted.insert(pair.key.clone());
            }
            // Base 的同名表也当兜底来源（Base 任何语言都加载，所以只补缺的键）
            if let Some(base_table) = base.get(filename) {
                for pair in base_table.pairs() {
                    if table.contains(&pair.key) || pair.value.is_empty() {
                        continue;
                    }
                    table.set(&pair.key, &pair.value);
                    wanted.insert(pair.key.clone());
                }
            }
            let after = table.key_set();
            let added = after.difference(&before).count();
            if added == 0 && !before.is_empty() {
                continue; // 这条语言/表已经齐了，别白写一遍
            }
            doc.set(
                &comp,
                table.to_bytes(),
            );
            written.entry(target.clone()).or_default().insert(
                filename.clone(),
                FileWriteInfo { component: comp.clone(), added, updated: 0, total: after.len() },
            );
            touched.entry(target.clone()).or_default().insert(filename.clone(), wanted);
        }
    }

    if written.is_empty() {
        return Err(CoreError::Invalid("所有目标语言都已完整，没有需要补的内容".into()));
    }

    let info = doc.save(&dest)?;

    // 回读校验
    let check = Archive::open(&dest)?;
    let mut readback: BTreeMap<String, ReadbackInfo> = BTreeMap::new();
    for (target, files) in &touched {
        for (filename, want) in files {
            let entry = &written[target][filename];
            match check.try_read(&entry.component) {
                Some(bytes) => {
                    let table = StringFile::parse(&bytes, &entry.component)?;
                    let got = want.iter().filter(|k| table.contains(k)).count();
                    readback.insert(
                        format!("{target}/{filename}"),
                        ReadbackInfo { ok: got == want.len(), keys: got, expected: want.len() },
                    );
                }
                None => {
                    readback.insert(
                        format!("{target}/{filename}"),
                        ReadbackInfo { ok: false, keys: 0, expected: want.len() },
                    );
                }
            }
        }
    }
    let ok = !readback.is_empty() && readback.values().all(|r| r.ok);
    let total_added: usize = written.values().flat_map(|m| m.values()).map(|i| i.added).sum();

    Ok(ApplyResult {
        ok,
        input: path.display().to_string(),
        output: dest.display().to_string(),
        target_locale: if target_list.len() == 1 { target_list[0].clone() } else { "all".into() },
        source_locale: Some(source_locale.to_string()),
        components_written: written,
        container: ContainerInfo {
            bytes: info.bytes,
            entries: info.entries,
            blocks: info.blocks,
            hash_entries: info.hash_entries,
            het_bytes: info.het_bytes,
            bet_bytes: info.bet_bytes,
            block_size: info.block_size,
        },
        readback,
        bytes_in: std::fs::metadata(path)?.len(),
        bytes_out: std::fs::metadata(&dest)?.len(),
        seeded: Some(total_added),
        diagnosis_before: None,
    })
}

/// 快捷修复：把源语言的文本整份补进**一个**目标语言。
///
/// 等价于「补全默认语言」只勾一个语言 —— 显示英文总好过显示 Param/Value/xxxx。
pub fn seed(
    path: &Path,
    source_locale: &str,
    target_locale: &str,
    opts: &ApplyOptions,
) -> Result<ApplyResult> {
    let doc = extract(path, source_locale, target_locale, None, false)?;
    let missing = doc.stats.by_status.get(STATUS_MISSING).copied().unwrap_or(0)
        + doc.stats.by_status.get(STATUS_IDENTICAL).copied().unwrap_or(0);
    let mut result = fill_locales(
        path,
        source_locale,
        &FillTargets::Explicit(vec![target_locale.to_string()]),
        true,
        opts.with_original,
        opts.out.as_deref(),
        opts.force,
    )?;
    result.seeded = Some(missing);
    result.diagnosis_before = doc.diagnosis.clone();
    Ok(result)
}

// --------------------------------------------------------------------------- //
// verify
// --------------------------------------------------------------------------- //

/// 容器自检 + 字符串表往返回读。
pub fn verify(path: &Path, deep: bool) -> Result<Value> {
    let arc = Archive::open(path)?;
    ensure_listfile(&arc, path)?;
    let r = arc.verify(deep);
    let mut tables = serde_json::Map::new();
    let mut roundtrip_all = true;
    for (locale, files) in locale_tables(&arc) {
        let mut file_map = serde_json::Map::new();
        for (filename, comp) in files {
            let raw = arc.read(&comp)?;
            let table = StringFile::parse(&raw, &comp)?;
            let exact = table.to_bytes() == raw;
            roundtrip_all &= exact;
            file_map.insert(
                filename,
                json!({
                    "bytes": raw.len(),
                    "keys": table.len(),
                    "bom": table.bom,
                    "newline": table.newline.label(),
                    "roundtrip_exact": exact,
                }),
            );
        }
        tables.insert(locale, Value::Object(file_map));
    }
    Ok(json!({
        "path": arc.path.display().to_string(),
        "sha256": sha256_file(path)?,
        "size": std::fs::metadata(path)?.len(),
        "format_version": r.format_version,
        "header_size": r.header_size,
        "raw_chunk_size": r.raw_chunk_size,
        "header_md5_valid": r.header_md5_valid,
        "block_table_md5_valid": r.block_table_md5_valid,
        "hash_table_md5_valid": r.hash_table_md5_valid,
        "het_md5_valid": r.het_md5_valid,
        "bet_md5_valid": r.bet_md5_valid,
        "het_roundtrip_exact": r.het_roundtrip_exact,
        "bet_roundtrip_exact": r.bet_roundtrip_exact,
        "het_vs_classic_mismatch": r.het_vs_classic_mismatch,
        "bet_vs_blocktable_mismatch": r.bet_vs_blocktable_mismatch,
        "bet_flag_mismatch": r.bet_flag_mismatch,
        "md5_ok": r.md5_ok,
        "md5_bad": r.md5_bad,
        "crc_ok": r.crc_ok,
        "crc_bad": r.crc_bad,
        "healthy": r.healthy(),
        "string_tables": Value::Object(tables),
        "string_tables_roundtrip_exact": roundtrip_all,
    }))
}
