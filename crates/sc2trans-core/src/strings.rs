//! SC2 本地化字符串表（<locale>.SC2Data\\LocalizedData\\*.txt）的解析与序列化。
//!
//! ## 实测到的字节级事实（见 docs/research/sc2-localization.md）
//!
//! | 事实 | 观测 |
//! | --- | --- |
//! | 编码 | UTF-8；**BOM 有无都可能**（银河编辑器写出的有 BOM，暴雪正式发行的 zhCN 多为无 BOM） |
//! | 换行 | CRLF / LF 都能用；编辑器产出的地图是 CRLF |
//! | 语法 | key=value，一行一条；**只按第一个等号切**（值里可含等号，如 <s val="X">） |
//! | 转义 | 没有。换行用 <n/>；<c val>、<IMG path>、%token%、~A~ 必须整体保留 |
//! | 注释 | 以 // 开头的整行；真实文件里极少 |
//! | 译文原文对照 | key=本地化文本 /// 英文原文 —— **暴雪自己就这么写** |
//! | 重复键 | 真实存在；本实现只改第一条、保序 |
//!
//! 设计原则是**逐字节保真**：无法识别的行原样保留在 Raw 记录里，写回时按原顺序输出。

use std::fmt::Write as _;

pub const BOM_BYTES: [u8; 3] = [0xEF, 0xBB, 0xBF];

/// 译文与原文的分隔符。暴雪官方本地化文件即使用此约定。
pub const ORIGINAL_SEP: &str = " /// ";

/// 字符串表错误。
#[derive(Debug, thiserror::Error)]
pub enum StringFileError {
    #[error("{0} 不是 UTF-8 文本（SC2 字符串表应为 UTF-8）")]
    NotUtf8(String),
}

/// 换行风格。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Newline {
    Crlf,
    Lf,
}

impl Newline {
    pub fn as_str(self) -> &'static str {
        match self {
            Newline::Crlf => "\r\n",
            Newline::Lf => "\n",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Newline::Crlf => "CRLF",
            Newline::Lf => "LF",
        }
    }
}

/// 一条 key=value。original 是 /// 之后的英文原文（可空）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pair {
    pub key: String,
    pub value: String,
    pub original: Option<String>,
}

impl Pair {
    pub fn render(&self) -> String {
        match &self.original {
            Some(o) => format!("{}={}{}{}", self.key, self.value, ORIGINAL_SEP, o),
            None => format!("{}={}", self.key, self.value),
        }
    }
}

/// 一行记录：要么是键值对，要么是无法/无需解析的原文。
#[derive(Clone, Debug)]
pub enum Record {
    Raw(String),
    Pair(Pair),
}

/// 一张字符串表。
#[derive(Clone, Debug)]
pub struct StringFile {
    pub records: Vec<Record>,
    pub bom: bool,
    pub newline: Newline,
    pub trailing_newline: bool,
    pub source_name: String,
}

impl Default for StringFile {
    fn default() -> Self {
        // 银河编辑器新建文件时的习惯：BOM + CRLF + 末尾换行
        Self {
            records: Vec::new(),
            bom: true,
            newline: Newline::Crlf,
            trailing_newline: true,
            source_name: String::new(),
        }
    }
}

impl StringFile {
    // ------------------------------------------------------------------ 解析
    pub fn parse(data: &[u8], source_name: &str) -> Result<Self, StringFileError> {
        let bom = data.starts_with(&BOM_BYTES);
        let body = if bom { &data[BOM_BYTES.len()..] } else { data };
        let text = std::str::from_utf8(body)
            .map_err(|_| StringFileError::NotUtf8(source_name.to_string()))?;

        let newline = if text.contains("\r\n") { Newline::Crlf } else { Newline::Lf };
        let trailing_newline = text.ends_with("\r\n") || text.ends_with('\n');
        let mut lines: Vec<&str> = match newline {
            Newline::Crlf => text.split("\r\n").collect(),
            Newline::Lf => text.split('\n').collect(),
        };
        if trailing_newline && lines.last().is_some_and(|l| l.is_empty()) {
            lines.pop();
        }

        let mut records = Vec::with_capacity(lines.len());
        for line in lines {
            let stripped = line.trim();
            if stripped.is_empty() || stripped.starts_with("//") || !line.contains('=') {
                records.push(Record::Raw(line.to_string()));
                continue;
            }
            let (key_part, rest) = line.split_once('=').expect("已判定有等号");
            let key = key_part.trim();
            if key.is_empty() {
                records.push(Record::Raw(line.to_string()));
                continue;
            }
            let pair = match rest.split_once(ORIGINAL_SEP) {
                Some((value, original)) => Pair {
                    key: key.to_string(),
                    value: value.to_string(),
                    original: Some(original.to_string()),
                },
                None => Pair { key: key.to_string(), value: rest.to_string(), original: None },
            };
            records.push(Record::Pair(pair));
        }

        Ok(Self {
            records,
            bom,
            newline,
            trailing_newline,
            source_name: source_name.to_string(),
        })
    }

    // ------------------------------------------------------------------ 写出
    pub fn to_bytes(&self) -> Vec<u8> {
        let nl = self.newline.as_str();
        let mut text = String::new();
        for (i, record) in self.records.iter().enumerate() {
            if i > 0 {
                text.push_str(nl);
            }
            match record {
                Record::Raw(line) => text.push_str(line),
                Record::Pair(pair) => text.push_str(&pair.render()),
            }
        }
        if self.trailing_newline && !self.records.is_empty() {
            text.push_str(nl);
        }
        let mut out = Vec::with_capacity(text.len() + 3);
        if self.bom {
            out.extend_from_slice(&BOM_BYTES);
        }
        out.extend_from_slice(text.as_bytes());
        out
    }

    // ------------------------------------------------------------------ 访问
    pub fn pairs(&self) -> impl Iterator<Item = &Pair> {
        self.records.iter().filter_map(|r| match r {
            Record::Pair(p) => Some(p),
            Record::Raw(_) => None,
        })
    }

    pub fn keys(&self) -> Vec<String> {
        self.pairs().map(|p| p.key.clone()).collect()
    }

    pub fn len(&self) -> usize {
        self.pairs().count()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get(&self, key: &str) -> Option<&Pair> {
        self.pairs().find(|p| p.key == key)
    }

    pub fn contains(&self, key: &str) -> bool {
        self.get(key).is_some()
    }

    /// 键 -> 值。
    pub fn as_map(&self) -> std::collections::HashMap<String, String> {
        self.pairs().map(|p| (p.key.clone(), p.value.clone())).collect()
    }

    /// 键集合（BTreeSet：便于做集合运算且顺序确定）。
    pub fn key_set(&self) -> std::collections::BTreeSet<String> {
        self.pairs().map(|p| p.key.clone()).collect()
    }

    // ------------------------------------------------------------------ 修改
    /// 写入/更新一条；**保留已有的 /// 原文**。
    pub fn set(&mut self, key: &str, value: &str) {
        if let Some(p) = self.find_pair_mut(key) {
            p.value = value.to_string();
            return;
        }
        self.records.push(Record::Pair(Pair {
            key: key.to_string(),
            value: value.to_string(),
            original: None,
        }));
    }

    /// 写入/更新一条，并显式设置 /// 原文（None = 不加原文）。
    pub fn set_full(&mut self, key: &str, value: &str, original: Option<&str>) {
        if let Some(p) = self.find_pair_mut(key) {
            p.value = value.to_string();
            p.original = original.map(|s| s.to_string());
            return;
        }
        self.records.push(Record::Pair(Pair {
            key: key.to_string(),
            value: value.to_string(),
            original: original.map(|s| s.to_string()),
        }));
    }

    fn find_pair_mut(&mut self, key: &str) -> Option<&mut Pair> {
        self.records.iter_mut().find_map(|r| match r {
            Record::Pair(p) if p.key == key => Some(p),
            _ => None,
        })
    }

    pub fn remove(&mut self, key: &str) -> bool {
        let before = self.records.len();
        let mut done = false;
        self.records.retain(|r| {
            if done {
                return true;
            }
            match r {
                Record::Pair(p) if p.key == key => {
                    done = true;
                    false
                }
                _ => true,
            }
        });
        self.records.len() != before
    }

    /// 就地按 key 排序（只重排 Pair，Raw 行的位置不动）。
    pub fn sort(&mut self) {
        let mut pairs: Vec<Pair> = self.pairs().cloned().collect();
        pairs.sort_by(|a, b| a.key.cmp(&b.key));
        let mut it = pairs.into_iter();
        for r in self.records.iter_mut() {
            if matches!(r, Record::Pair(_)) {
                if let Some(p) = it.next() {
                    *r = Record::Pair(p);
                }
            }
        }
    }

    /// 批量补键；返回真正新增的键。已存在的不动。
    pub fn add_missing<'a, I: IntoIterator<Item = (&'a str, &'a str)>>(&mut self, items: I) -> Vec<String> {
        let mut added = Vec::new();
        for (key, value) in items {
            if !self.contains(key) {
                self.records.push(Record::Pair(Pair {
                    key: key.to_string(),
                    value: value.to_string(),
                    original: None,
                }));
                added.push(key.to_string());
            }
        }
        added
    }

    /// 统计值里的内联标记数量（<n/>、<c val>、<s val>、<IMG>、%token%），用于回填后自检。
    pub fn markup_count(&self) -> usize {
        self.pairs()
            .map(|p| {
                p.value.matches("<n/>").count()
                    + p.value.matches("<c val=").count()
                    + p.value.matches("<s val=").count()
                    + p.value.matches("<IMG ").count()
                    + p.value.matches('%').count() / 2
            })
            .sum()
    }

    /// 调试用摘要。
    pub fn describe(&self) -> String {
        let mut s = String::new();
        let _ = write!(
            s,
            "{} 条 / BOM={} / {} / 末尾换行={}",
            self.len(),
            self.bom,
            self.newline.label(),
            self.trailing_newline
        );
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CRLF_BOM: &[u8] = b"\xef\xbb\xbfParam/Value/AB=Hive Destroyed\r\nDocInfo/Name=Domination\r\n";
    fn crlf_no_bom() -> Vec<u8> {
        "DocInfo/Author=Blizzard Entertainment\r\nDocInfo/Name=统治 /// Domination\r\n"
            .as_bytes()
            .to_vec()
    }

    #[test]
    fn roundtrip_is_byte_exact() {
        let f = StringFile::parse(CRLF_BOM, "t").unwrap();
        assert!(f.bom);
        assert_eq!(f.newline, Newline::Crlf);
        assert!(f.trailing_newline);
        assert_eq!(f.to_bytes(), CRLF_BOM);
        assert_eq!(f.keys(), vec!["Param/Value/AB", "DocInfo/Name"]);
    }

    #[test]
    fn bom_less_file_stays_bom_less() {
        let raw = crlf_no_bom();
        let f = StringFile::parse(&raw, "t").unwrap();
        assert!(!f.bom);
        assert_eq!(f.to_bytes(), raw);
    }

    #[test]
    fn triple_slash_splits_translation_and_original() {
        let f = StringFile::parse(&crlf_no_bom(), "t").unwrap();
        let p = f.get("DocInfo/Name").unwrap();
        assert_eq!(p.value, "统治");
        assert_eq!(p.original.as_deref(), Some("Domination"));
        assert_eq!(p.render(), "DocInfo/Name=统治 /// Domination");
    }

    #[test]
    fn value_may_contain_equals_sign() {
        let data = "LoadingScreen/Title=<s val=\"VoidLoadingIntroScreenTitle\">LEGACY OF THE VOID</s>\r\n";
        let raw = data.as_bytes();

        let f = StringFile::parse(raw, "t").unwrap();
        assert_eq!(
            f.get("LoadingScreen/Title").unwrap().value,
            "<s val=\"VoidLoadingIntroScreenTitle\">LEGACY OF THE VOID</s>"
        );
        assert_eq!(f.to_bytes(), raw);
    }

    #[test]
    fn empty_value_is_legal() {
        let raw: &[u8] = b"DocInfo/DescLong=\r\n";
        let f = StringFile::parse(raw, "t").unwrap();
        assert_eq!(f.get("DocInfo/DescLong").unwrap().value, "");
        assert_eq!(f.to_bytes(), raw);
    }

    #[test]
    fn lf_file_stays_lf() {
        let raw: &[u8] = b"DocInfo/Name=X\nDocInfo/Author=Y\n";
        let f = StringFile::parse(raw, "t").unwrap();
        assert_eq!(f.newline, Newline::Lf);
        assert_eq!(f.to_bytes(), raw);
    }

    #[test]
    fn comments_and_odd_lines_survive() {
        let raw: &[u8] = b"// a comment\r\n\r\nthis line has no equals\r\nA=1\r\n";
        let f = StringFile::parse(raw, "t").unwrap();
        assert_eq!(f.to_bytes(), raw);
        assert_eq!(f.len(), 1);
    }

    #[test]
    fn set_keeps_existing_original_by_default() {
        let mut f = StringFile::parse(&crlf_no_bom(), "t").unwrap();
        f.set("DocInfo/Name", "统治者");
        let p = f.get("DocInfo/Name").unwrap();
        assert_eq!(p.value, "统治者");
        assert_eq!(p.original.as_deref(), Some("Domination"));
    }

    #[test]
    fn new_key_has_no_original() {
        let mut f = StringFile::parse(&crlf_no_bom(), "t").unwrap();
        f.set("New/Key", "value");
        assert_eq!(f.keys().last().unwrap(), "New/Key");
        assert!(f.get("New/Key").unwrap().original.is_none());
    }

    #[test]
    fn add_missing_does_not_touch_existing() {
        let mut f = StringFile::parse(&crlf_no_bom(), "t").unwrap();
        let added = f.add_missing([("DocInfo/Name", "nope"), ("DocInfo/Website", "https://x")]);
        assert_eq!(added, vec!["DocInfo/Website"]);
        assert_eq!(f.get("DocInfo/Name").unwrap().value, "统治");
    }

    #[test]
    fn sort_keeps_raw_lines_in_place() {
        let raw: &[u8] = b"// head\r\nB=1\r\nA=2\r\n";
        let mut f = StringFile::parse(raw, "t").unwrap();
        f.sort();
        assert_eq!(f.keys(), vec!["A", "B"]);
        assert_eq!(f.to_bytes(), b"// head\r\nA=2\r\nB=1\r\n");
    }

    #[test]
    fn default_new_file_is_bom_crlf() {
        let mut f = StringFile::default();
        f.set("A", "1");
        assert_eq!(f.to_bytes(), b"\xef\xbb\xbfA=1\r\n");
    }

    #[test]
    fn invalid_utf8_is_rejected() {
        assert!(StringFile::parse(&[0xFF, 0xFE, 0x00], "t").is_err());
    }
}
