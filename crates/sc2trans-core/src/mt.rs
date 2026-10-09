//! 机翻参考。
//!
//! 定位很清楚：**只给参考，绝不自动写进键里**。界面上它单独一块，用户点「用作译文」
//! 才会落到译文框 —— 这是刻意的，机翻直接落库等于把错误固化进地图。
//!
//! 两个 provider：
//!
//! * "offline"：**不联网**。用术语库拼装 —— 把命中的术语替换成译名，其余保持原文，
//!   并列出命中的术语。它的价值是「至少术语是对的」，也是没有 API key 时的兜底。
//! * "openai"：任何 OpenAI 兼容的 chat/completions 接口（官方、DeepSeek、本地 Ollama、
//!   one-api 网关都行）。base_url / api_key / model 都可配。
//!
//! 结果带缓存（<data>/mt-cache.json），同一句话不会反复打接口。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::glossary::{GlossaryPair, Term};
use crate::{CoreError, Result};

pub const MT_CACHE_FILE: &str = "mt-cache.json";
pub const MT_CONFIG_FILE: &str = "mt.json";

/// 用哪个 provider。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    /// 离线：术语库拼装。默认用它 —— 不需要任何配置就能给出「术语正确」的参考。
    #[default]
    Offline,
    /// 任何 OpenAI 兼容接口。
    Openai,
}

/// 机翻配置。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MtConfig {
    #[serde(default)]
    pub provider: Provider,
    /// 例如 https://api.openai.com/v1 或 http://127.0.0.1:11434/v1
    #[serde(default = "default_base")]
    pub base_url: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default = "default_model")]
    pub model: String,
    /// 目标语言的自然语言说法，写进提示词。
    #[serde(default = "default_target")]
    pub target_language: String,
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
}

fn default_base() -> String {
    "https://api.openai.com/v1".to_string()
}
fn default_model() -> String {
    "gpt-4o-mini".to_string()
}
fn default_target() -> String {
    "简体中文".to_string()
}
fn default_timeout() -> u64 {
    30
}

impl Default for MtConfig {
    fn default() -> Self {
        Self {
            provider: Provider::Offline,
            base_url: default_base(),
            api_key: String::new(),
            model: default_model(),
            target_language: default_target(),
            timeout_secs: default_timeout(),
        }
    }
}

impl MtConfig {
    pub fn load(data_dir: &Path) -> Self {
        std::fs::read_to_string(data_dir.join(MT_CONFIG_FILE))
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, data_dir: &Path) -> Result<()> {
        std::fs::create_dir_all(data_dir)?;
        std::fs::write(data_dir.join(MT_CONFIG_FILE), serde_json::to_string_pretty(self)?)?;
        Ok(())
    }

    /// 在线 provider 必备的字段齐了吗。
    pub fn ready(&self) -> bool {
        match self.provider {
            Provider::Offline => true,
            Provider::Openai => !self.api_key.trim().is_empty() && !self.base_url.trim().is_empty(),
        }
    }

    pub fn provider_name(&self) -> &'static str {
        match self.provider {
            Provider::Offline => "offline",
            Provider::Openai => "openai",
        }
    }
}

/// 一条参考译文。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Suggestion {
    /// 机翻/拼装出来的文本。
    pub text: String,
    /// 谁给的。
    pub provider: String,
    /// 命中的术语（离线 provider 一定给，在线 provider 也顺手带上）。
    #[serde(default)]
    pub terms: Vec<Term>,
    /// 这条建议的性质说明 —— 界面上要如实告诉用户。
    pub note: String,
    /// 是否来自缓存。
    #[serde(default)]
    pub cached: bool,
}

/// 机翻引擎（带内存 + 磁盘缓存）。
pub struct MtEngine {
    pub config: MtConfig,
    cache_path: PathBuf,
    cache: BTreeMap<String, String>,
}

impl MtEngine {
    pub fn new(config: MtConfig, data_dir: &Path) -> Self {
        let cache_path = data_dir.join(MT_CACHE_FILE);
        let cache = std::fs::read_to_string(&cache_path)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default();
        Self { config, cache_path, cache }
    }

    fn cache_key(&self, text: &str) -> String {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        self.config.provider_name().hash(&mut h);
        self.config.target_language.hash(&mut h);
        self.config.model.hash(&mut h);
        text.hash(&mut h);
        format!("{:016x}", h.finish())
    }

    fn remember(&mut self, key: String, value: String) {
        self.cache.insert(key, value);
        // 缓存写盘失败不影响主流程（下次重算而已）
        if let Some(parent) = self.cache_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(text) = serde_json::to_string(&self.cache) {
            let _ = std::fs::write(&self.cache_path, text);
        }
    }

    pub fn cache_len(&self) -> usize {
        self.cache.len()
    }

    pub fn clear_cache(&mut self) {
        self.cache.clear();
        let _ = std::fs::remove_file(&self.cache_path);
    }

    /// 翻一条。绝不抛「因为没配 key」这种错就把整个界面卡住 —— 调用方拿到 Err 显示即可。
    pub fn translate(&mut self, text: &str, glossary: &GlossaryPair) -> Result<Suggestion> {
        let text = text.trim();
        if text.is_empty() {
            return Err(CoreError::Invalid("没有可翻译的原文".into()));
        }
        let terms = glossary.match_text(text);
        let key = self.cache_key(text);
        if let Some(hit) = self.cache.get(&key) {
            return Ok(Suggestion {
                text: hit.clone(),
                provider: self.config.provider_name().to_string(),
                terms,
                note: "来自缓存".to_string(),
                cached: true,
            });
        }

        let suggestion = match self.config.provider {
            Provider::Offline => Ok(offline_assemble(text, &terms)),
            Provider::Openai => self.call_openai(text, &terms),
        }?;
        self.remember(key, suggestion.text.clone());
        Ok(suggestion)
    }

    fn call_openai(&self, text: &str, terms: &[Term]) -> Result<Suggestion> {
        if !self.config.ready() {
            return Err(CoreError::Invalid(
                "还没配置机翻接口：请到「机翻设置」里填 base_url 与 api_key（也可以用离线术语拼装）".into(),
            ));
        }
        let url = format!("{}/chat/completions", self.config.base_url.trim_end_matches('/'));

        let mut glossary_hint = String::new();
        if !terms.is_empty() {
            glossary_hint.push_str("必须使用以下术语译法：\n");
            for t in terms {
                glossary_hint.push_str(&format!("- {} -> {}\n", t.source, t.target));
            }
        }
        let system = format!(
            "你是《星际争霸 II》的专业本地化译员。把用户给的英文游戏文本翻成{}。\
             只输出译文本身，不要解释、不要引号、不要 Markdown。\
             保留原文里的全部内联标记（例如 <n/>、<c val=...>、<s val=...>、%token%、~A~）与标点结构。\
             {}",
            self.config.target_language, glossary_hint
        );

        let body = serde_json::json!({
            "model": self.config.model,
            "temperature": 0.2,
            "messages": [
                { "role": "system", "content": system },
                { "role": "user", "content": text },
            ],
        });

        let agent = ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(self.config.timeout_secs))
            .build();

        let response = agent
            .post(&url)
            .set("Content-Type", "application/json")
            .set("Authorization", &format!("Bearer {}", self.config.api_key.trim()))
            .send_json(body);

        let value: serde_json::Value = match response {
            Ok(resp) => resp.into_json().map_err(|e| {
                CoreError::Invalid(format!("机翻接口返回的不是 JSON：{e}"))
            })?,
            Err(ureq::Error::Status(code, resp)) => {
                let detail = resp.into_string().unwrap_or_default();
                let brief: String = detail.chars().take(200).collect();
                return Err(CoreError::Invalid(format!("机翻接口返回 {code}：{brief}")));
            }
            Err(e) => return Err(CoreError::Invalid(format!("机翻接口连不上：{e}"))),
        };

        let content = value
            .get("choices")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("message"))
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .ok_or_else(|| CoreError::Invalid("机翻接口的返回里没有 choices[0].message.content".into()))?;

        Ok(Suggestion {
            text: content.trim().to_string(),
            provider: format!("openai:{}", self.config.model),
            terms: terms.to_vec(),
            note: format!("由 {} 生成，仅供参考，写回前请自行校对", self.config.model),
            cached: false,
        })
    }
}

/// 离线拼装：把命中的术语换成译名，其余保持原文。
fn offline_assemble(text: &str, terms: &[Term]) -> Suggestion {
    let mut out = text.to_string();
    let mut replaced = 0usize;
    // 长的先替换，避免 Terran 把 Infested Terran 拆坏
    for term in terms {
        if term.source.trim().is_empty() {
            continue;
        }
        let replaced_once = if term.case_sensitive {
            if out.contains(&term.source) {
                out = out.replace(&term.source, &term.target);
                true
            } else {
                false
            }
        } else {
            let lower = out.to_lowercase();
            let needle = term.source.to_lowercase();
            if lower.contains(&needle) {
                // 大小写不敏感时按下标替换，保住原文其余部分
                let mut result = String::new();
                let mut rest = out.as_str();
                while let Some(pos) = rest.to_lowercase().find(&needle) {
                    result.push_str(&rest[..pos]);
                    result.push_str(&term.target);
                    rest = &rest[pos + needle.len()..];
                }
                result.push_str(rest);
                out = result;
                true
            } else {
                false
            }
        };
        if replaced_once {
            replaced += 1;
        }
    }

    let note = if terms.is_empty() {
        "离线模式：这句话没有命中任何术语，下面是原文（想看真正的机翻请到「机翻设置」里配在线接口）".to_string()
    } else {
        format!("离线模式：只替换了命中的 {replaced} 个术语，其余保持英文 —— 术语是对的，语序还得你自己来")
    };

    Suggestion {
        text: out,
        provider: "offline".to_string(),
        terms: terms.to_vec(),
        note,
        cached: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::glossary::{Glossary, Scope, Term};

    fn engine() -> (MtEngine, GlossaryPair) {
        let dir = std::env::temp_dir().join("sc2trans_mt_test");
        std::fs::create_dir_all(&dir).unwrap();
        let _ = std::fs::remove_file(dir.join(MT_CACHE_FILE));
        let engine = MtEngine::new(MtConfig::default(), &dir);
        let mut pair = GlossaryPair::default();
        pair.global.upsert(Term::new("Infested Terran", "被感染的陆战队员", Scope::Global));
        pair.global.upsert(Term::new("Hive", "主巢", Scope::Global));
        (engine, pair)
    }

    #[test]
    fn offline_replaces_matched_terms_only() {
        let (mut e, pair) = engine();
        let s = e.translate("Destroy the Hive with Infested Terran", &pair).unwrap();
        assert_eq!(s.text, "Destroy the 主巢 with 被感染的陆战队员");
        assert_eq!(s.terms.len(), 2);
        assert!(s.note.contains("离线"));
        assert_eq!(s.provider, "offline");
    }

    #[test]
    fn offline_without_terms_returns_original_and_says_so() {
        let (mut e, pair) = engine();
        let s = e.translate("nothing matches here", &pair).unwrap();
        assert_eq!(s.text, "nothing matches here");
        assert!(s.note.contains("没有命中任何术语"));
    }

    #[test]
    fn results_are_cached() {
        let (mut e, pair) = engine();
        let first = e.translate("Destroy the Hive", &pair).unwrap();
        assert!(!first.cached);
        let second = e.translate("Destroy the Hive", &pair).unwrap();
        assert!(second.cached, "第二次应当命中缓存");
        assert_eq!(second.text, first.text);
        assert!(e.cache_len() >= 1);
    }

    #[test]
    fn empty_text_is_rejected() {
        let (mut e, pair) = engine();
        assert!(e.translate("   ", &pair).is_err());
    }

    #[test]
    fn online_requires_credentials() {
        let (mut e, pair) = engine();
        e.config.provider = Provider::Openai;
        e.config.api_key = String::new();
        assert!(!e.config.ready());
        let err = e.translate("Hello", &pair).unwrap_err().to_string();
        assert!(err.contains("还没配置机翻接口"));
    }

    #[test]
    fn config_roundtrip() {
        let dir = std::env::temp_dir().join("sc2trans_mt_cfg");
        std::fs::create_dir_all(&dir).unwrap();
        let c = MtConfig {
            provider: Provider::Openai,
            model: "deepseek-chat".into(),
            ..MtConfig::default()
        };
        c.save(&dir).unwrap();
        let back = MtConfig::load(&dir);
        assert_eq!(back.provider, Provider::Openai);
        assert_eq!(back.model, "deepseek-chat");
        let _ = std::fs::remove_file(dir.join(MT_CONFIG_FILE));
    }

    #[test]
    fn glossary_is_attached_to_suggestions() {
        let (mut e, pair) = engine();
        let s = e.translate("A Hive rises", &pair).unwrap();
        assert_eq!(s.terms.len(), 1);
        assert_eq!(s.terms[0].target, "主巢");
    }

    #[test]
    fn unused_glossary_import_is_used() {
        let mut g = Glossary::new("x");
        g.upsert(Term::new("A", "甲", Scope::Global));
        assert_eq!(g.len(), 1);
    }
}
