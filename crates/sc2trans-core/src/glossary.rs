//! 专有名词库（术语表）。
//!
//! 汉化最怕的就是同一个东西在不同条目里翻成不同的词。术语表解决两件事：
//!
//! 1. **查**：选中一条键时，自动把它命中的术语列出来（右侧详情面板）；
//!    打开术语库面板后 Ctrl+F 直接搜。
//! 2. **管**：术语分两个作用域 —— 全局（跨项目复用）与项目（只对当前项目生效）。
//!    查表时项目优先覆盖全局。
//!
//! 这一层不碰 GUI，也不碰网络；只做数据与检索。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{CoreError, Result, timeutil};

pub const GLOSSARY_FORMAT: u32 = 1;
pub const GLOBAL_FILE: &str = "glossary.global.json";

/// 术语作用域。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Scope {
    /// 全局：所有项目都能用。
    Global,
    /// 项目：只对当前项目生效，且覆盖同名的全局词条。
    Project,
}

impl Scope {
    pub fn as_str(self) -> &'static str {
        match self {
            Scope::Global => "global",
            Scope::Project => "project",
        }
    }
}

/// 一条术语。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Term {
    /// 稳定 id（用于增删改与前端 diff）。
    pub id: String,
    /// 原文（英文）。
    pub source: String,
    /// 译名（中文）。
    pub target: String,
    /// 备注：为什么这么翻、出处之类的。
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub note: String,
    /// 领域/分类，例如 单位、技能、地名。
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub domain: String,
    /// 大小写敏感匹配（专有名词一般要）。
    #[serde(default)]
    pub case_sensitive: bool,
    /// 作用域。
    pub scope: Scope,
    #[serde(default)]
    pub updated_at: String,
}

impl Term {
    pub fn new(source: impl Into<String>, target: impl Into<String>, scope: Scope) -> Self {
        let source = source.into();
        Self {
            id: make_id(&source, scope),
            source,
            target: target.into(),
            note: String::new(),
            domain: String::new(),
            case_sensitive: true,
            scope,
            updated_at: timeutil::now_rfc3339(),
        }
    }

    /// 匹配用的比较串。
    fn haystack(&self) -> String {
        if self.case_sensitive {
            self.source.clone()
        } else {
            self.source.to_lowercase()
        }
    }

    /// 这个词条在给定文本里出现过吗。
    pub fn matches(&self, text: &str) -> bool {
        if self.source.trim().is_empty() || text.is_empty() {
            return false;
        }
        let needle = self.haystack();
        let hay = if self.case_sensitive { text.to_string() } else { text.to_lowercase() };
        hay.contains(&needle)
    }

    fn touches(&self, query: &str) -> bool {
        let q = query.trim().to_lowercase();
        if q.is_empty() {
            return true;
        }
        self.source.to_lowercase().contains(&q)
            || self.target.contains(&q)
            || self.note.to_lowercase().contains(&q)
            || self.domain.to_lowercase().contains(&q)
    }
}

/// 由原文与作用域生成稳定 id。
///
/// 用「作用域 + 小写原文」的短哈希：同一个词条重复导入时能认出是同一条，
/// 而不同作用域的同名术语互不覆盖。
pub fn make_id(source: &str, scope: Scope) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    scope.as_str().hash(&mut hasher);
    source.trim().to_lowercase().hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// 一份术语表。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Glossary {
    #[serde(default = "default_format")]
    pub format: u32,
    /// 这份表的名字（全局表固定叫「全局术语库」）。
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub terms: Vec<Term>,
    #[serde(default)]
    pub updated_at: String,
    /// 磁盘位置（不写进文件）。
    #[serde(skip)]
    pub path: Option<String>,
}

fn default_format() -> u32 {
    GLOSSARY_FORMAT
}

impl Default for Glossary {
    fn default() -> Self {
        Self {
            format: GLOSSARY_FORMAT,
            name: String::new(),
            terms: Vec::new(),
            updated_at: timeutil::now_rfc3339(),
            path: None,
        }
    }
}

impl Glossary {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into(), ..Default::default() }
    }

    pub fn len(&self) -> usize {
        self.terms.len()
    }

    pub fn is_empty(&self) -> bool {
        self.terms.is_empty()
    }

    /// 加一条；同 id 存在就替换（返回是否新增）。
    pub fn upsert(&mut self, term: Term) -> bool {
        self.updated_at = timeutil::now_rfc3339();
        match self.terms.iter().position(|t| t.id == term.id) {
            Some(i) => {
                self.terms[i] = term;
                false
            }
            None => {
                self.terms.push(term);
                true
            }
        }
    }

    pub fn remove(&mut self, id: &str) -> bool {
        let before = self.terms.len();
        self.terms.retain(|t| t.id != id);
        let changed = self.terms.len() != before;
        if changed {
            self.updated_at = timeutil::now_rfc3339();
        }
        changed
    }

    /// 按原文排序，保证界面顺序稳定。
    pub fn sort(&mut self) {
        self.terms.sort_by(|a, b| {
            a.source.to_lowercase().cmp(&b.source.to_lowercase()).then_with(|| a.target.cmp(&b.target))
        });
    }

    /// 搜索：原文 / 译名 / 备注 / 领域都算，原文精确匹配排最前。
    pub fn search(&self, query: &str) -> Vec<Term> {
        let q = query.trim().to_lowercase();
        let mut hits: Vec<(u8, Term)> = self
            .terms
            .iter()
            .filter(|t| t.touches(query))
            .map(|t| {
                let rank = if q.is_empty() {
                    3
                } else if t.source.to_lowercase() == q {
                    0
                } else if t.source.to_lowercase().starts_with(&q) {
                    1
                } else if t.source.to_lowercase().contains(&q) {
                    2
                } else {
                    3
                };
                (rank, t.clone())
            })
            .collect();
        hits.sort_by(|a, b| {
            a.0.cmp(&b.0).then_with(|| a.1.source.to_lowercase().cmp(&b.1.source.to_lowercase()))
        });
        hits.into_iter().map(|(_, t)| t).collect()
    }

    /// 一段文本命中了哪些术语（长的优先 —— Infested Terran 比 Terran 更具体）。
    pub fn match_text(&self, text: &str) -> Vec<Term> {
        let mut hits: Vec<Term> = self.terms.iter().filter(|t| t.matches(text)).cloned().collect();
        hits.sort_by(|a, b| b.source.len().cmp(&a.source.len()));
        hits
    }

    pub fn to_json(&self) -> Result<String> {
        Ok(format!("{}\n", serde_json::to_string_pretty(self)?))
    }

    pub fn from_json(text: &str) -> Result<Self> {
        let mut g: Glossary = serde_json::from_str(text)?;
        if g.format > GLOSSARY_FORMAT {
            return Err(CoreError::Invalid(format!(
                "术语库版本 {} 比本程序支持的 {} 更新，请升级工具",
                g.format, GLOSSARY_FORMAT
            )));
        }
        g.format = GLOSSARY_FORMAT;
        Ok(g)
    }

    pub fn load(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path)?;
        let mut g = Self::from_json(&text)?;
        g.path = Some(path.display().to_string());
        Ok(g)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let mut copy = self.clone();
        copy.updated_at = timeutil::now_rfc3339();
        std::fs::write(path, copy.to_json()?)?;
        Ok(())
    }

    // ------------------------------------------------------------------ 导入导出

    /// CSV 导入：四列依次是 原文 / 译名 / 备注 / 领域；首行是表头则跳过。
    pub fn import_csv(&mut self, text: &str, scope: Scope) -> usize {
        let mut n = 0;
        for (i, line) in text.lines().enumerate() {
            let line = line.trim_end_matches(['\r', '\n']);
            if line.trim().is_empty() || line.trim_start().starts_with('#') {
                continue;
            }
            let fields = split_csv_line(line);
            if fields.is_empty() {
                continue;
            }
            if i == 0 {
                let head = fields[0].to_lowercase();
                if head == "source" || head == "原文" || head == "key" {
                    continue; // 表头
                }
            }
            if fields.len() < 2 {
                continue;
            }
            let mut term = Term::new(fields[0].trim(), fields[1].trim(), scope);
            if let Some(note) = fields.get(2) {
                term.note = note.trim().to_string();
            }
            if let Some(domain) = fields.get(3) {
                term.domain = domain.trim().to_string();
            }
            if term.source.is_empty() || term.target.is_empty() {
                continue;
            }
            self.upsert(term);
            n += 1;
        }
        n
    }

    pub fn export_csv(&self) -> String {
        let mut out = String::from("source,target,note,domain\n");
        for t in &self.terms {
            out.push_str(&escape_csv(&t.source));
            out.push(',');
            out.push_str(&escape_csv(&t.target));
            out.push(',');
            out.push_str(&escape_csv(&t.note));
            out.push(',');
            out.push_str(&escape_csv(&t.domain));
            out.push('\n');
        }
        out
    }
}

/// 全局 + 项目两份表合成一个视图。
#[derive(Debug, Clone, Default)]
pub struct GlossaryPair {
    pub global: Glossary,
    pub project: Glossary,
}

impl GlossaryPair {
    /// 合并后的检索：项目覆盖全局（同原文时项目那条胜出）。
    pub fn merged(&self) -> Glossary {
        let mut by_source: BTreeMap<String, Term> = BTreeMap::new();
        for t in &self.global.terms {
            by_source.insert(t.source.trim().to_lowercase(), t.clone());
        }
        for t in &self.project.terms {
            by_source.insert(t.source.trim().to_lowercase(), t.clone());
        }
        let mut g = Glossary::new("合并视图");
        g.terms = by_source.into_values().collect();
        g.sort();
        g
    }

    pub fn search(&self, query: &str) -> Vec<Term> {
        self.merged().search(query)
    }

    pub fn match_text(&self, text: &str) -> Vec<Term> {
        self.merged().match_text(text)
    }

    pub fn counts(&self) -> (usize, usize) {
        (self.global.len(), self.project.len())
    }
}

/// 全局术语库的位置（应用数据目录下）。
pub fn global_path(data_dir: &Path) -> PathBuf {
    data_dir.join(GLOBAL_FILE)
}

/// 项目术语库的位置（跟着项目文件走）。
pub fn project_path(project_file: &Path) -> PathBuf {
    let stem = project_file.file_stem().and_then(|s| s.to_str()).unwrap_or("project");
    let dir = project_file.parent().unwrap_or(Path::new("."));
    dir.join(format!("{stem}.glossary.json"))
}

// --------------------------------------------------------------------------- //
// CSV 小工具
// --------------------------------------------------------------------------- //

fn split_csv_line(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                if quoted && chars.peek() == Some(&'"') {
                    cur.push('"');
                    chars.next();
                } else {
                    quoted = !quoted;
                }
            }
            ',' if !quoted => {
                out.push(cur.clone());
                cur.clear();
            }
            _ => cur.push(c),
        }
    }
    out.push(cur);
    out
}

fn escape_csv(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn g() -> Glossary {
        let mut g = Glossary::new("t");
        g.upsert(Term::new("Infested Terran", "被感染的陆战队员", Scope::Global));
        g.upsert(Term::new("Terran", "人类", Scope::Global));
        g.upsert(Term::new("Hive", "主巢", Scope::Project));
        g
    }

    #[test]
    fn id_is_stable_and_scope_aware() {
        let a = make_id("Hive", Scope::Global);
        let b = make_id(" hive ", Scope::Global);
        let c = make_id("Hive", Scope::Project);
        assert_eq!(a, b, "同一原文 + 同一作用域应当同 id");
        assert_ne!(a, c, "不同作用域不应互相覆盖");
    }

    #[test]
    fn upsert_replaces_same_id() {
        let mut g = Glossary::new("t");
        assert!(g.upsert(Term::new("Hive", "主巢", Scope::Global)));
        let mut second = Term::new("Hive", "虫巢", Scope::Global);
        second.note = "改译名".into();
        assert!(!g.upsert(second));
        assert_eq!(g.len(), 1);
        assert_eq!(g.terms[0].target, "虫巢");
    }

    #[test]
    fn search_ranks_exact_first() {
        let g = g();
        let hits = g.search("terran");
        assert_eq!(hits[0].source, "Terran", "精确匹配应当排第一");
        assert!(hits.iter().any(|t| t.source == "Infested Terran"));
    }

    #[test]
    fn search_matches_target_and_note_too() {
        let mut g = Glossary::new("t");
        let mut t = Term::new("Hive", "主巢", Scope::Global);
        t.note = "虫族的建筑".into();
        g.upsert(t);
        assert_eq!(g.search("主巢").len(), 1);
        assert_eq!(g.search("虫族").len(), 1);
        assert_eq!(g.search("不存在").len(), 0);
    }

    #[test]
    fn match_text_prefers_longer_terms() {
        let g = g();
        let hits = g.match_text("An Infested Terran attacks the Hive");
        assert_eq!(hits[0].source, "Infested Terran", "更具体的词应当排在前面");
        assert_eq!(hits.len(), 3);
    }

    #[test]
    fn case_sensitivity_is_respected() {
        let mut g = Glossary::new("t");
        g.upsert(Term::new("Hive", "主巢", Scope::Global));
        assert!(g.match_text("a hive of scum").is_empty(), "默认大小写敏感，小写 hive 不该命中");
        let mut loose = Term::new("Scarab", "圣甲虫", Scope::Global);
        loose.case_sensitive = false;
        g.upsert(loose);
        assert_eq!(g.match_text("scarab").len(), 1);
    }

    #[test]
    fn pair_prefers_project_over_global() {
        let mut pair = GlossaryPair::default();
        pair.global.upsert(Term::new("Hive", "主巢", Scope::Global));
        pair.project.upsert(Term::new("Hive", "虫巢", Scope::Project));
        let merged = pair.merged();
        assert_eq!(merged.len(), 1);
        assert_eq!(merged.terms[0].target, "虫巢");
        assert_eq!(pair.counts(), (1, 1));
    }

    #[test]
    fn csv_roundtrip_handles_quotes_and_commas() {
        let mut g = Glossary::new("t");
        let mut t = Term::new("Zergling, swarm", "跳虫", Scope::Global);
        t.note = "带\"引号\"的备注".to_string();
        g.upsert(t);
        let csv = g.export_csv();
        let mut back = Glossary::new("t2");
        back.import_csv(&csv, Scope::Global);
        assert_eq!(back.len(), 1);
        assert_eq!(back.terms[0].source, "Zergling, swarm");
        assert_eq!(back.terms[0].note, "带\"引号\"的备注");
    }

    #[test]
    fn csv_import_skips_header_and_blank_lines() {
        let csv = "source,target,note,domain\n\nHive,主巢,,建筑\n# 注释\nTerran,人类\n";
        let mut g = Glossary::new("t");
        let n = g.import_csv(csv, Scope::Project);
        assert_eq!(n, 2);
        assert!(g.terms.iter().all(|t| t.scope == Scope::Project));
        assert_eq!(g.terms[0].domain, "建筑");
    }

    #[test]
    fn json_roundtrip_and_future_format_rejected() {
        let mut g = g();
        g.sort();
        let text = g.to_json().unwrap();
        let back = Glossary::from_json(&text).unwrap();
        assert_eq!(back.terms, g.terms);

        let bad = format!("{{\"format\": {}, \"terms\": []}}", GLOSSARY_FORMAT + 3);
        assert!(Glossary::from_json(&bad).is_err());
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = std::env::temp_dir().join("sc2trans_glossary_test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = global_path(&dir);
        let mut g = g();
        g.name = "全局术语库".into();
        g.save(&path).unwrap();
        let back = Glossary::load(&path).unwrap();
        assert_eq!(back.name, "全局术语库");
        assert_eq!(back.len(), 3);
        assert!(back.path.is_some());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn project_path_sits_next_to_project_file() {
        let p = project_path(Path::new("D:/x/战役.sc2transproj"));
        assert_eq!(p.file_name().unwrap().to_str().unwrap(), "战役.glossary.json");
    }
}
