//! 项目模型：一个项目 = 一组待处理的文档 + 每个文档的语言对。
//!
//! 为什么要有「项目」：汉化一张地图往往不是单文件的事 —— 战役包是几十张图，
//! 社区模组还带依赖。把它们收进一个项目，切文件、看整体进度、批量补全才有意义。
//!
//! 这一层**不碰文件系统**（除了显式的 load/save），路径由调用方给，便于测试。

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{CoreError, Result, timeutil};

/// 项目文件格式版本。改结构时 +1，并写迁移。
pub const PROJECT_FORMAT: u32 = 1;

/// 项目文件扩展名。
pub const PROJECT_EXT: &str = "sc2transproj";

/// 项目里的一个文档。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjectFile {
    /// 归档绝对路径。
    pub path: String,
    /// 标签（默认取文件名，用户可以改）。
    #[serde(default)]
    pub label: String,
    /// 原文语言。
    #[serde(default = "default_source")]
    pub source: String,
    /// 目标语言。
    #[serde(default = "default_target")]
    pub target: String,
    /// 上次写出的产物路径。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    /// 上次扫描到的统计（用于文件列表展示，避免每次重新解包）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stats: Option<FileStats>,
    /// 关联上的同名 JSON（同目录、同主文件名）。没有就是 None。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keys_json: Option<String>,
}

/// 项目级的默认值：新加进来的文件取这里的语言对。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Defaults {
    #[serde(default = "default_source")]
    pub source: String,
    #[serde(default = "default_target")]
    pub target: String,
}

impl Default for Defaults {
    fn default() -> Self {
        Self { source: default_source(), target: default_target() }
    }
}

/// 一个文档的缓存统计。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct FileStats {
    pub entries: usize,
    pub missing: usize,
    pub identical: usize,
    pub translated: usize,
    pub locales_present: usize,
    pub size: u64,
    #[serde(default)]
    pub scanned_at: String,
}

impl FileStats {
    /// 待翻译 = 缺失 + 未翻译。
    pub fn todo(&self) -> usize {
        self.missing + self.identical
    }

    /// 已有文本的比例（0~100）。
    pub fn percent(&self) -> f64 {
        if self.entries == 0 {
            return 0.0;
        }
        let done = self.entries - self.todo();
        ((done as f64 / self.entries as f64) * 1000.0).round() / 10.0
    }
}

fn default_source() -> String {
    "enUS".to_string()
}

/// 给别的模块用的同名默认值（serde 的 default 只能指向本模块函数）。
pub fn default_source_pub() -> String {
    default_source()
}

pub fn default_target_pub() -> String {
    default_target()
}

fn default_target() -> String {
    "zhCN".to_string()
}

impl ProjectFile {
    pub fn new(path: impl Into<String>) -> Self {
        let path = path.into();
        Self {
            label: label_for(&path),
            path,
            source: default_source(),
            target: default_target(),
            output: None,
            stats: None,
            keys_json: None,
        }
    }
}

/// 从路径取一个好看的标签：文件名去掉扩展名。
pub fn label_for(path: &str) -> String {
    Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(path)
        .to_string()
}

/// 一个项目。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Project {
    pub format: u32,
    pub name: String,
    #[serde(default)]
    pub files: Vec<ProjectFile>,
    /// 语言对的默认值。老项目文件里没有这一项，normalize 时从第一个文件推出来。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub defaults: Option<Defaults>,
    /// 当前选中的文件下标。
    #[serde(default)]
    pub active: usize,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
    /// 项目文件的路径（不在磁盘格式里，由调用方填）。
    #[serde(skip)]
    pub path: Option<String>,
}

impl Default for Project {
    fn default() -> Self {
        let now = timeutil::now_rfc3339();
        Self {
            format: PROJECT_FORMAT,
            name: "未命名项目".to_string(),
            files: Vec::new(),
            defaults: None,
            active: 0,
            created_at: now.clone(),
            updated_at: now,
            path: None,
        }
    }
}

impl Project {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into(), ..Default::default() }
    }

    /// 项目名默认取项目文件的文件名。
    pub fn name_from_path(path: &str) -> String {
        label_for(path)
    }

    /// 加一个文档；已经在项目里就只更新路径（不重复添加）。返回它的下标。
    pub fn add_file(&mut self, path: impl Into<String>) -> usize {
        let path = path.into();
        if let Some(i) = self.index_of(&path) {
            return i;
        }
        self.files.push(ProjectFile::new(path));
        self.files.len() - 1
    }

    pub fn index_of(&self, path: &str) -> Option<usize> {
        let norm = normalize_path(path);
        self.files.iter().position(|f| normalize_path(&f.path) == norm)
    }

    pub fn remove_file(&mut self, index: usize) -> Option<ProjectFile> {
        if index >= self.files.len() {
            return None;
        }
        let removed = self.files.remove(index);
        if self.active >= self.files.len() && !self.files.is_empty() {
            self.active = self.files.len() - 1;
        }
        Some(removed)
    }

    /// 汇总统计（文件列表头部用）。
    pub fn totals(&self) -> FileStats {
        let mut t = FileStats::default();
        for f in &self.files {
            if let Some(s) = &f.stats {
                t.entries += s.entries;
                t.missing += s.missing;
                t.identical += s.identical;
                t.translated += s.translated;
                t.locales_present = t.locales_present.max(s.locales_present);
                t.size += s.size;
            }
        }
        t
    }

    /// 保存前规范化：丢掉重复文件、修正越界的 active、刷新时间戳。
    pub fn normalize(&mut self) {
        let mut seen: Vec<String> = Vec::new();
        self.files.retain(|f| {
            let key = normalize_path(&f.path);
            if seen.contains(&key) {
                false
            } else {
                seen.push(key);
                true
            }
        });
        if self.files.is_empty() {
            self.active = 0;
        } else if self.active >= self.files.len() {
            self.active = self.files.len() - 1;
        }
        if self.format == 0 {
            self.format = PROJECT_FORMAT;
        }
        // 迁移：老项目文件没有 defaults，用第一个文件的语言对补上（不猜别的）
        if self.defaults.is_none() {
            self.defaults = Some(match self.files.first() {
                Some(f) => Defaults { source: f.source.clone(), target: f.target.clone() },
                None => Defaults::default(),
            });
        }
        if self.name.trim().is_empty() {
            self.name = "未命名项目".to_string();
        }
        self.updated_at = timeutil::now_rfc3339();
    }

    pub fn to_json(&self) -> Result<String> {
        let mut copy = self.clone();
        copy.normalize();
        Ok(format!("{}\n", serde_json::to_string_pretty(&copy)?))
    }

    pub fn from_json(text: &str) -> Result<Self> {
        let mut p: Project = serde_json::from_str(text)?;
        if p.format > PROJECT_FORMAT {
            return Err(CoreError::Invalid(format!(
                "项目文件版本 {} 比本程序支持的 {} 更新，请升级工具",
                p.format, PROJECT_FORMAT
            )));
        }
        p.normalize();
        Ok(p)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        std::fs::write(path, self.to_json()?)?;
        Ok(())
    }

    pub fn load(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path)?;
        let mut p = Self::from_json(&text)?;
        p.path = Some(path.display().to_string());
        if p.name.trim().is_empty() || p.name == "未命名项目" {
            p.name = Self::name_from_path(&path.display().to_string());
        }
        Ok(p)
    }
}

/// 统一路径写法，避免同一个文件因为斜杠/大小写被当成两个。
pub fn normalize_path(path: &str) -> String {
    let unified = path.replace('/', "\\");
    // Windows 路径大小写不敏感
    unified.to_ascii_lowercase()
}

// --------------------------------------------------------------------------- //
// 最近项目
// --------------------------------------------------------------------------- //

/// 最近打开的项目列表（存在应用数据目录里）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RecentProjects {
    #[serde(default)]
    pub items: Vec<RecentEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecentEntry {
    pub path: String,
    pub name: String,
    pub opened_at: String,
    #[serde(default)]
    pub files: usize,
}

/// 最近项目最多记这么多条。
pub const RECENT_LIMIT: usize = 12;

impl RecentProjects {
    pub fn load(dir: &Path) -> Self {
        Self::load_named(dir, "recent-projects.json")
    }

    pub fn save(&self, dir: &Path) -> Result<()> {
        self.save_named(dir, "recent-projects.json")
    }

    /// 指定文件名读写 —— 工作区改造后最近列表换成 recent-folders.json。
    pub fn load_named(dir: &Path, name: &str) -> Self {
        let file = dir.join(name);
        std::fs::read_to_string(&file)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default()
    }

    pub fn save_named(&self, dir: &Path, name: &str) -> Result<()> {
        std::fs::create_dir_all(dir)?;
        std::fs::write(dir.join(name), serde_json::to_string_pretty(self)?)?;
        Ok(())
    }

    /// 记一次「打开」。已存在就提到最前并更新时间。
    pub fn touch(&mut self, project: &Project) {
        let Some(path) = project.path.clone() else { return };
        let key = normalize_path(&path);
        self.items.retain(|e| normalize_path(&e.path) != key);
        self.items.insert(
            0,
            RecentEntry {
                path,
                name: project.name.clone(),
                opened_at: timeutil::now_rfc3339(),
                files: project.files.len(),
            },
        );
        self.items.truncate(RECENT_LIMIT);
    }

    pub fn forget(&mut self, path: &str) {
        let key = normalize_path(path);
        self.items.retain(|e| normalize_path(&e.path) != key);
    }

    /// 过滤掉已经不存在的文件。
    pub fn prune(&mut self) {
        self.items.retain(|e| PathBuf::from(&e.path).is_file());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_migrate_from_first_file() {
        let mut p = Project::new("战役");
        p.add_file("D:/m/a.SC2Map");
        p.files[0].source = "enGB".into();
        p.files[0].target = "zhTW".into();
        assert!(p.defaults.is_none());
        p.normalize();
        let d = p.defaults.unwrap();
        assert_eq!(d.source, "enGB");
        assert_eq!(d.target, "zhTW");
    }

    #[test]
    fn defaults_survive_roundtrip_and_are_not_overwritten() {
        let mut p = Project::new("战役");
        p.add_file("D:/m/a.SC2Map");
        p.defaults = Some(Defaults { source: "enUS".into(), target: "zhCN".into() });
        p.files[0].source = "jaJP".into();
        p.normalize();
        // 显式设过的默认值不该被文件的当前值盖掉
        assert_eq!(p.defaults.unwrap().source, "enUS");
    }

    #[test]
    fn keys_json_roundtrips() {
        let mut p = Project::new("战役");
        p.add_file("D:/m/a.SC2Map");
        p.files[0].keys_json = Some("D:/m/a.json".into());
        let text = p.to_json().unwrap();
        let back = Project::from_json(&text).unwrap();
        assert_eq!(back.files[0].keys_json.as_deref(), Some("D:/m/a.json"));
    }

    #[test]
    fn add_file_is_idempotent() {
        let mut p = Project::new("战役");
        let a = p.add_file("D:/Maps/a.SC2Map");
        let b = p.add_file("D:\\Maps\\a.SC2Map"); // 分隔符不同，同一个文件
        assert_eq!(a, b);
        assert_eq!(p.files.len(), 1);
        assert_eq!(p.files[0].label, "a");
    }

    #[test]
    fn remove_file_keeps_active_in_range() {
        let mut p = Project::new("x");
        p.add_file("D:/a.SC2Map");
        p.add_file("D:/b.SC2Map");
        p.active = 1;
        p.remove_file(1);
        assert_eq!(p.active, 0);
        assert_eq!(p.files.len(), 1);
    }

    #[test]
    fn json_roundtrip_keeps_state() {
        let mut p = Project::new("战役");
        p.add_file("D:/Maps/a.SC2Map");
        p.add_file("D:/Maps/b.SC2Mod");
        p.active = 1;
        p.files[0].target = "zhTW".into();
        p.files[0].stats = Some(FileStats {
            entries: 100,
            missing: 40,
            identical: 10,
            translated: 50,
            ..Default::default()
        });
        let text = p.to_json().unwrap();
        let back = Project::from_json(&text).unwrap();
        assert_eq!(back.name, "战役");
        assert_eq!(back.files.len(), 2);
        assert_eq!(back.active, 1);
        assert_eq!(back.files[0].target, "zhTW");
        assert_eq!(back.files[0].stats.as_ref().unwrap().todo(), 50);
        assert_eq!(back.files[0].stats.as_ref().unwrap().percent(), 50.0);
    }

    #[test]
    fn normalize_drops_duplicates_and_fixes_active() {
        let mut p = Project::new("");
        p.files.push(ProjectFile { path: "D:/a.SC2Map".into(), ..ProjectFile::new("D:/a.SC2Map") });
        p.files.push(ProjectFile { path: "d:\\a.SC2Map".into(), ..ProjectFile::new("d:\\a.SC2Map") });
        p.active = 9;
        p.normalize();
        assert_eq!(p.files.len(), 1);
        assert_eq!(p.active, 0);
        assert_eq!(p.name, "未命名项目");
    }

    #[test]
    fn future_format_is_rejected_with_a_readable_message() {
        let text = format!("{{\"format\": {}, \"name\": \"x\", \"files\": [], \"active\": 0}}", PROJECT_FORMAT + 5);
        let err = Project::from_json(&text).unwrap_err();
        assert!(format!("{err}").contains("比本程序支持的"));
    }

    #[test]
    fn recent_list_moves_reopened_project_to_front() {
        let mut recent = RecentProjects::default();
        let mut a = Project::new("A");
        a.path = Some("D:/A.sc2transproj".into());
        a.files.push(ProjectFile::new("D:/a.SC2Map"));
        let mut b = Project::new("B");
        b.path = Some("D:/B.sc2transproj".into());
        recent.touch(&a);
        recent.touch(&b);
        assert_eq!(recent.items[0].name, "B");
        recent.touch(&a);
        assert_eq!(recent.items[0].name, "A");
        assert_eq!(recent.items.len(), 2);
        assert_eq!(recent.items[0].files, 1);
    }

    #[test]
    fn totals_add_up_across_files() {
        let mut p = Project::new("x");
        p.add_file("D:/a.SC2Map");
        p.add_file("D:/b.SC2Map");
        p.files[0].stats = Some(FileStats { entries: 10, missing: 4, identical: 1, translated: 5, ..Default::default() });
        p.files[1].stats = Some(FileStats { entries: 20, missing: 6, identical: 2, translated: 12, ..Default::default() });
        let t = p.totals();
        assert_eq!(t.entries, 30);
        assert_eq!(t.todo(), 13);
        assert_eq!(t.percent(), 56.7);
    }
}
