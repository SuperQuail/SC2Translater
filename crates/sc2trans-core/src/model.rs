//! 翻译键 JSON 结构（schema = sc2trans.keys.v1）与输入归一化。
//!
//! 设计目标：
//!
//! 1. **给 agent 用** —— 字段名自解释，附 stats，一次调用就能拿到结论；
//! 2. **给翻译者用** —— 每条同时带 key / 原文 / 译文，翻译者只填 target；
//! 3. **可回填** —— apply 只依赖 (file, key, target)，多余字段不影响兼容；
//!    因此也接受**手写的扁平 JSON** { "DocInfo/Name": "统治" }。

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{CoreError, Result};

pub const STATUS_MISSING: &str = "missing";
pub const STATUS_IDENTICAL: &str = "identical";
pub const STATUS_TRANSLATED: &str = "translated";
pub const STATUS_EXTRA: &str = "extra";
pub const STATUS_NO_SOURCE: &str = "no-source";

/// 状态的显示名。
pub fn status_label(status: &str) -> &'static str {
    match status {
        STATUS_MISSING => "缺失（会显示原始键路径）",
        STATUS_IDENTICAL => "与原文相同（未翻译）",
        STATUS_TRANSLATED => "已翻译",
        STATUS_EXTRA => "仅目标语言存在",
        STATUS_NO_SOURCE => "无原文",
        _ => "",
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ArchiveInfo {
    pub path: String,
    pub size: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Entry {
    pub key: String,
    pub file: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    #[serde(default)]
    pub status: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub origin: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Stats {
    pub entries: usize,
    #[serde(default)]
    pub by_status: BTreeMap<String, usize>,
    #[serde(default)]
    pub by_file: BTreeMap<String, usize>,
    #[serde(default)]
    pub translatable: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub locales_present: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Diagnosis {
    pub severity: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub missing_in_target: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actionable: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub worst: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub player_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TranslationDocument {
    pub schema: String,
    pub tool: String,
    pub generated_at: String,
    pub archive: ArchiveInfo,
    pub source_locale: String,
    pub target_locale: String,
    #[serde(default)]
    pub files: Vec<String>,
    #[serde(default)]
    pub entries: Vec<Entry>,
    #[serde(default)]
    pub stats: Stats,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diagnosis: Option<Diagnosis>,
}

/// 归一化后的输入：条目 + JSON 里声明的源/目标语言。
pub struct Normalized {
    pub entries: Vec<Entry>,
    pub source_locale: Option<String>,
    pub target_locale: Option<String>,
}

/// 把任意输入 JSON 归一成条目列表。
///
/// 支持两种形态：本工具导出的完整结构（含 entries），或翻译者手写的扁平字典。
pub fn normalize_input(raw: &Value, default_file: &str) -> Result<Normalized> {
    if let Value::Object(map) = raw {
        if let Some(Value::Array(items)) = map.get("entries") {
            let mut entries = Vec::with_capacity(items.len());
            for item in items {
                let Value::Object(obj) = item else { continue };
                let Some(Value::String(key)) = obj.get("key") else { continue };
                entries.push(Entry {
                    key: key.clone(),
                    file: obj
                        .get("file")
                        .and_then(|v| v.as_str())
                        .filter(|s| !s.is_empty())
                        .unwrap_or(default_file)
                        .to_string(),
                    source: obj.get("source").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    target: obj.get("target").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    status: obj.get("status").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    origin: obj
                        .get("origin")
                        .and_then(|v| v.as_array())
                        .map(|a| {
                            a.iter().filter_map(|x| x.as_str()).map(|s| s.to_string()).collect()
                        })
                        .unwrap_or_default(),
                });
            }
            let source_locale =
                map.get("source_locale").and_then(|v| v.as_str()).map(|s| s.to_string());
            let target_locale =
                map.get("target_locale").and_then(|v| v.as_str()).map(|s| s.to_string());
            return Ok(Normalized { entries, source_locale, target_locale });
        }

        // 扁平字典
        let mut entries = Vec::with_capacity(map.len());
        for (key, value) in map {
            let (file, source, target) = if let Value::Object(obj) = value {
                (
                    obj.get("file").and_then(|v| v.as_str()).unwrap_or(default_file).to_string(),
                    obj.get("source").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    obj.get("target").and_then(|v| v.as_str()).map(|s| s.to_string()),
                )
            } else {
                (default_file.to_string(), None, value.as_str().map(|s| s.to_string()))
            };
            entries.push(Entry {
                key: key.clone(),
                file,
                source,
                target,
                status: String::new(),
                origin: Vec::new(),
            });
        }
        return Ok(Normalized { entries, source_locale: None, target_locale: None });
    }
    Err(CoreError::Invalid(
        "无法识别的翻译 JSON：既不是 {entries:[...]} 也不是 {key: value}".to_string(),
    ))
}

/// 统计条目。
pub fn compute_stats(entries: &[Entry], locales_present: Vec<String>) -> Stats {
    let mut by_status: BTreeMap<String, usize> = BTreeMap::new();
    let mut by_file: BTreeMap<String, usize> = BTreeMap::new();
    let mut translatable = 0usize;
    for e in entries {
        *by_status.entry(e.status.clone()).or_insert(0) += 1;
        *by_file.entry(e.file.clone()).or_insert(0) += 1;
        if e.status == STATUS_MISSING || e.status == STATUS_IDENTICAL {
            translatable += 1;
        }
    }
    Stats { entries: entries.len(), by_status, by_file, translatable, locales_present }
}
