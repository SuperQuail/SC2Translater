//! 工作区：**一个文件夹就是一个工程**（照 VS Code / JetBrains 的做法）。
//!
//! 数据全部收进 @@<文件夹>/.sc2translater/@@：
//!
//! @@@
//! 工程根/
//!   Maps/…                     用户的地图，原地不动
//!   .sc2translater/
//!     workspace.json           工作区清单（本模块管的就是它）
//!     glossary.json            项目术语库（glossary 模块管）
//!     cache/stats.json         扫描统计（可再生的，单独放，别把主文件写脏）
//!     repos/                    每个文档一个 SC2Diff 仓库（vcs 模块管）
//!     .gitignore               自动生成，内容一行 cache/
//! @@@
//!
//! 两条硬规矩：
//!
//! 1. **路径一律相对工程根** —— 整个文件夹打包发给别人，打开就能用；
//! 2. **只新增，不改用户的东西** —— 迁移老项目文件时原文件保留。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::project::{Defaults, FileStats, PROJECT_EXT, Project};
use crate::{CoreError, Result, timeutil};

/// 工作区目录名。
pub const WORKSPACE_DIR: &str = ".sc2translater";
/// 清单文件名。
pub const WORKSPACE_FILE: &str = "workspace.json";
/// 缓存目录（可随时删）。
pub const CACHE_DIR: &str = "cache";
/// 统计缓存文件名。
pub const STATS_FILE: &str = "stats.json";
/// 仓库目录名。
pub const REPOS_DIR: &str = "repos";
/// 清单格式版本。
pub const WORKSPACE_FORMAT: u32 = 1;

/// 工作区目录。
pub fn workspace_dir(root: &Path) -> PathBuf {
    root.join(WORKSPACE_DIR)
}

/// 清单文件。
pub fn workspace_file(root: &Path) -> PathBuf {
    workspace_dir(root).join(WORKSPACE_FILE)
}

/// 统计缓存文件。
pub fn stats_file(root: &Path) -> PathBuf {
    workspace_dir(root).join(CACHE_DIR).join(STATS_FILE)
}

/// 仓库根目录。
pub fn repos_dir(root: &Path) -> PathBuf {
    workspace_dir(root).join(REPOS_DIR)
}

/// 工程里的一个文档。路径**相对工程根**。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Entry {
    pub path: String,
    #[serde(default)]
    pub label: String,
    #[serde(default = "crate::project::default_source_pub")]
    pub source: String,
    #[serde(default = "crate::project::default_target_pub")]
    pub target: String,
    /// 上次写出的产物（相对工程根，或盘外的绝对路径）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    /// 关联上的同名 JSON。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keys_json: Option<String>,
}

impl Entry {
    pub fn new(path: impl Into<String>) -> Self {
        let path = path.into();
        Self {
            label: crate::project::label_for(&path),
            path,
            source: Defaults::default().source,
            target: Defaults::default().target,
            output: None,
            keys_json: None,
        }
    }
}

/// 界面偏好，跟着工作区走（换台机器、发给同事，布局跟着去）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct View {
    #[serde(default = "view_activity")]
    pub activity: String,
    #[serde(default = "yes")]
    pub left: bool,
    #[serde(default = "yes")]
    pub right: bool,
    #[serde(default = "view_left_width")]
    pub left_width: u32,
    #[serde(default = "view_right_width")]
    pub right_width: u32,
    #[serde(default = "view_density")]
    pub density: String,
    #[serde(default = "view_tab")]
    pub tab_view: String,
}

fn view_activity() -> String {
    "explorer".into()
}
fn yes() -> bool {
    true
}
fn view_left_width() -> u32 {
    252
}
fn view_right_width() -> u32 {
    336
}
fn view_density() -> String {
    "standard".into()
}
fn view_tab() -> String {
    "table".into()
}

impl Default for View {
    fn default() -> Self {
        Self {
            activity: view_activity(),
            left: true,
            right: true,
            left_width: view_left_width(),
            right_width: view_right_width(),
            density: view_density(),
            tab_view: view_tab(),
        }
    }
}

/// 一个工作区。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Workspace {
    pub format: u32,
    pub name: String,
    #[serde(default)]
    pub defaults: Defaults,
    #[serde(default)]
    pub files: Vec<Entry>,
    /// 打开的标签组（对应 VS Code 的 opened editors）。
    #[serde(default)]
    pub open: Vec<String>,
    #[serde(default)]
    pub active: usize,
    #[serde(default)]
    pub view: View,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
    /// 工程根（不在磁盘格式里，由调用方填）。
    #[serde(skip)]
    pub root: Option<String>,
}

impl Default for Workspace {
    fn default() -> Self {
        let now = timeutil::now_rfc3339();
        Self {
            format: WORKSPACE_FORMAT,
            name: "未命名工作区".into(),
            defaults: Defaults::default(),
            files: Vec::new(),
            open: Vec::new(),
            active: 0,
            view: View::default(),
            created_at: now.clone(),
            updated_at: now,
            root: None,
        }
    }
}

impl Workspace {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into(), ..Default::default() }
    }

    /// 读一个文件夹的工作区。没有清单就返回一个**空的新工作区**（不落盘）。
    pub fn load(root: &Path) -> Result<Self> {
        let file = workspace_file(root);
        let mut ws = if file.is_file() {
            let text = std::fs::read_to_string(&file)?;
            let ws: Workspace = serde_json::from_str(&text)?;
            if ws.format > WORKSPACE_FORMAT {
                return Err(CoreError::Invalid(format!(
                    "工作区格式 {} 比本程序支持的 {} 更新，请升级工具",
                    ws.format, WORKSPACE_FORMAT
                )));
            }
            ws
        } else {
            let mut ws = Workspace::new(crate::project::label_for(&root.display().to_string()));
            ws.created_at = timeutil::now_rfc3339();
            ws
        };
        ws.format = WORKSPACE_FORMAT;
        ws.root = Some(root.display().to_string());
        Ok(ws)
    }

    /// 写清单。顺带把 .gitignore 与目录建好。
    pub fn save(&self, root: &Path) -> Result<()> {
        let mut copy = self.clone();
        copy.format = WORKSPACE_FORMAT;
        copy.updated_at = timeutil::now_rfc3339();
        copy.root = None;
        copy.normalize();

        let dir = workspace_dir(root);
        std::fs::create_dir_all(dir.join(CACHE_DIR))?;
        std::fs::create_dir_all(repos_dir(root))?;
        ensure_gitignore(root)?;

        let text = format!("{}\n", serde_json::to_string_pretty(&copy)?);
        std::fs::write(workspace_file(root), text)?;
        Ok(())
    }

    /// 统计**不进清单** —— 每次扫描都变，挪到 cache/ 去。
    pub fn load_stats(root: &Path) -> BTreeMap<String, FileStats> {
        std::fs::read_to_string(stats_file(root))
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default()
    }

    pub fn save_stats(root: &Path, stats: &BTreeMap<String, FileStats>) -> Result<()> {
        let file = stats_file(root);
        if let Some(parent) = file.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(file, serde_json::to_string_pretty(stats)?)?;
        Ok(())
    }

    /// 把绝对路径变成相对工程根；不在工程里就返回 None。
    pub fn relativize(root: &Path, absolute: &Path) -> Option<String> {
        let rel = absolute.strip_prefix(root).ok()?;
        Some(rel.to_string_lossy().replace('\\', "/"))
    }

    /// 相对路径解析回绝对路径。
    pub fn resolve(&self, root: &Path, rel: &str) -> PathBuf {
        let p = Path::new(rel);
        if p.is_absolute() {
            p.to_path_buf()
        } else {
            root.join(rel)
        }
    }

    /// 加一个文档；已经在里面就返回它的下标。
    pub fn add_file(&mut self, rel: impl Into<String>) -> usize {
        let rel = rel.into();
        if let Some(i) = self.files.iter().position(|f| f.path == rel) {
            return i;
        }
        let mut e = Entry::new(rel);
        e.source = self.defaults.source.clone();
        e.target = self.defaults.target.clone();
        self.files.push(e);
        self.files.len() - 1
    }

    /// 保存前规范化：丢重复、修正越界下标、补默认值。
    pub fn normalize(&mut self) {
        let mut seen: Vec<String> = Vec::new();
        self.files.retain(|f| {
            let key = f.path.replace('/', "\\").to_ascii_lowercase();
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
        // open 里指向已不在清单中的路径要丢掉
        let known: Vec<String> = self.files.iter().map(|f| f.path.clone()).collect();
        self.open.retain(|p| known.contains(p));
        if self.format == 0 {
            self.format = WORKSPACE_FORMAT;
        }
    }

    /// 从老项目文件迁移：绝对路径尽量相对化，盘外的原样留着并报出来。
    ///
    /// **不删除、不修改**老项目文件 —— 用户的东西只有他自己能删。
    pub fn from_project(project: &Project, root: &Path) -> (Self, Vec<String>) {
        let mut ws = Workspace::new(project.name.clone());
        ws.defaults = project.defaults.clone().unwrap_or_default();
        ws.created_at = project.created_at.clone();
        let mut outside: Vec<String> = Vec::new();
        for f in &project.files {
            let abs = PathBuf::from(&f.path);
            let path = match Workspace::relativize(root, &abs) {
                Some(rel) => rel,
                None => {
                    // 盘外的地图：留着绝对路径，并告诉用户这条搬不走
                    outside.push(f.path.clone());
                    f.path.clone()
                }
            };
            let mut e = Entry::new(path);
            e.label = f.label.clone();
            e.source = f.source.clone();
            e.target = f.target.clone();
            e.output = f.output.clone();
            e.keys_json = f.keys_json.clone();
            ws.files.push(e);
        }
        ws.active = project.active;
        ws.normalize();
        (ws, outside)
    }
}

/// 生成 .gitignore（内容就一行），已存在就不动。
fn ensure_gitignore(root: &Path) -> Result<()> {
    let file = workspace_dir(root).join(".gitignore");
    if file.exists() {
        return Ok(());
    }
    std::fs::write(file, "# 扫描统计可随时重建，别提交\ncache/\n")?;
    Ok(())
}

// --------------------------------------------------------------------------- //
// 扫描
// --------------------------------------------------------------------------- //

/// 扫出来的一个文件。
#[derive(Debug, Clone, Serialize)]
pub struct Found {
    /// 相对工程根。
    pub path: String,
    pub kind: crate::filekind::FileKind,
    pub size: u64,
    /// SC2 文档关联上的同名 JSON（相对路径）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linked_json: Option<String>,
}

/// 扫一个文件夹，列出我们认得的文件。
///
/// 有深度与条目上限 —— 扫几十 GB 的模组目录不能把界面拖死。
pub fn scan(root: &Path, max_depth: usize, max_entries: usize) -> Vec<Found> {
    use crate::filekind::{self, FileKind};
    use crate::link::{self, Link};

    const SKIP: [&str; 6] = [WORKSPACE_DIR, ".git", "node_modules", "target", ".idea", "__pycache__"];

    let mut out: Vec<Found> = Vec::new();
    let mut stack = vec![(root.to_path_buf(), 0usize)];
    while let Some((dir, depth)) = stack.pop() {
        if depth > max_depth || out.len() >= max_entries {
            continue;
        }
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') || SKIP.contains(&name.as_str()) {
                continue;
            }
            if path.is_dir() {
                stack.push((path, depth + 1));
                continue;
            }
            let c = filekind::classify(&path);
            let rel = Workspace::relativize(root, &path).unwrap_or_else(|| name.clone());
            let linked_json = if c.kind == FileKind::Sc2 {
                match link::link(&path) {
                    Link::Linked { path } => Workspace::relativize(root, &path),
                    _ => None,
                }
            } else {
                None
            };
            out.push(Found { path: rel, kind: c.kind, size: c.size, linked_json });
        }
    }
    out.sort_by(|a, b| a.path.cmp(&b.path));
    out.truncate(max_entries);
    out
}

/// 工程里有没有老的项目文件（迁移入口用）。
pub fn legacy_projects(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(root) else { return out };
    for e in entries.flatten() {
        let p = e.path();
        if p.extension().and_then(|x| x.to_str()) == Some(PROJECT_EXT) {
            out.push(p);
        }
    }
    out.sort();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join("sc2trans_workspace_test").join(name);
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn load_missing_returns_empty_without_writing() {
        let root = tmp("missing");
        let ws = Workspace::load(&root).unwrap();
        assert!(ws.files.is_empty());
        assert!(!workspace_file(&root).exists(), "只读不落盘");
    }

    #[test]
    fn save_then_load_roundtrip() {
        let root = tmp("roundtrip");
        let mut ws = Workspace::new("战役");
        ws.add_file("Maps/a.SC2Map");
        ws.files[0].target = "zhTW".into();
        ws.save(&root).unwrap();

        let back = Workspace::load(&root).unwrap();
        assert_eq!(back.name, "战役");
        assert_eq!(back.files.len(), 1);
        assert_eq!(back.files[0].path, "Maps/a.SC2Map");
        assert_eq!(back.files[0].target, "zhTW");
        assert_eq!(back.root.as_deref(), Some(root.display().to_string().as_str()));
    }

    #[test]
    fn gitignore_is_created_once_and_not_clobbered() {
        let root = tmp("gitignore");
        Workspace::new("x").save(&root).unwrap();
        let file = workspace_dir(&root).join(".gitignore");
        let first = std::fs::read_to_string(&file).unwrap();
        assert!(first.contains("cache/"));
        std::fs::write(&file, "用户改过的内容\n").unwrap();
        Workspace::new("x").save(&root).unwrap();
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "用户改过的内容\n", "不该覆盖用户的改动");
    }

    #[test]
    fn stats_live_in_cache_not_in_the_manifest() {
        let root = tmp("stats");
        let mut ws = Workspace::new("x");
        ws.add_file("a.SC2Map");
        ws.save(&root).unwrap();

        let mut stats = BTreeMap::new();
        stats.insert("a.SC2Map".to_string(), FileStats { entries: 677, ..Default::default() });
        Workspace::save_stats(&root, &stats).unwrap();

        let manifest = std::fs::read_to_string(workspace_file(&root)).unwrap();
        assert!(!manifest.contains("677"), "统计不该写进清单");
        assert_eq!(Workspace::load_stats(&root).get("a.SC2Map").unwrap().entries, 677);
    }

    #[test]
    fn paths_are_relative_so_the_folder_can_move() {
        let root = tmp("move");
        std::fs::create_dir_all(root.join("Maps")).unwrap();
        let map = root.join("Maps").join("a.SC2Map");
        std::fs::write(&map, b"x").unwrap();

        let mut ws = Workspace::new("x");
        ws.add_file(Workspace::relativize(&root, &map).unwrap());
        ws.save(&root).unwrap();

        // 换个根读同一份清单：路径仍然能解析
        let other = tmp("move-elsewhere");
        let text = std::fs::read_to_string(workspace_file(&root)).unwrap();
        let ws2: Workspace = serde_json::from_str(&text).unwrap();
        assert_eq!(ws2.files[0].path, "Maps/a.SC2Map");
        assert_eq!(ws2.resolve(&other, &ws2.files[0].path), other.join("Maps").join("a.SC2Map"));
    }

    #[test]
    fn migrate_from_project_relativizes_inside_and_flags_outside() {
        let root = tmp("migrate");
        std::fs::create_dir_all(root.join("Maps")).unwrap();
        let inside = root.join("Maps").join("a.SC2Map");
        std::fs::write(&inside, b"x").unwrap();

        let mut p = Project::new("老项目");
        p.add_file(inside.display().to_string());
        p.add_file("E:/别处/b.SC2Map");
        p.files[0].target = "zhTW".into();

        let (ws, outside) = Workspace::from_project(&p, &root);
        assert_eq!(ws.files.len(), 2);
        assert_eq!(ws.files[0].path, "Maps/a.SC2Map", "盘内的要相对化");
        assert_eq!(ws.files[1].path, "E:/别处/b.SC2Map", "盘外的保持绝对");
        assert_eq!(outside, vec!["E:/别处/b.SC2Map".to_string()]);
        assert_eq!(ws.files[0].target, "zhTW");
    }

    #[test]
    fn normalize_drops_duplicates_and_fixes_active() {
        let mut ws = Workspace::new("x");
        ws.files.push(Entry::new("a.SC2Map"));
        ws.files.push(Entry::new("A.sc2map"));
        ws.files.push(Entry::new("b.SC2Map"));
        ws.active = 99;
        ws.open = vec!["a.SC2Map".into(), "gone.SC2Map".into()];
        ws.normalize();
        assert_eq!(ws.files.len(), 2, "大小写不同也算同一个");
        assert_eq!(ws.active, 1);
        assert_eq!(ws.open, vec!["a.SC2Map".to_string()]);
    }

    #[test]
    fn scan_skips_our_own_dir_and_finds_known_types() {
        let root = tmp("scan");
        std::fs::create_dir_all(root.join("Maps")).unwrap();
        std::fs::create_dir_all(workspace_dir(&root)).unwrap();
        std::fs::write(root.join("Maps").join("a.SC2Map"), b"x").unwrap();
        std::fs::write(root.join("readme.txt"), b"hello").unwrap();
        std::fs::write(workspace_dir(&root).join("workspace.json"), b"{}").unwrap();
        std::fs::write(root.join(".hidden"), b"x").unwrap();

        let found = scan(&root, 6, 100);
        let paths: Vec<&str> = found.iter().map(|f| f.path.as_str()).collect();
        assert!(paths.contains(&"Maps/a.SC2Map"));
        assert!(paths.contains(&"readme.txt"));
        assert!(!paths.iter().any(|p| p.contains(".sc2translater")), "不该扫自己的工作区目录");
        assert!(!paths.iter().any(|p| p.contains(".hidden")));
    }

    #[test]
    fn scan_respects_entry_budget() {
        let root = tmp("budget");
        for i in 0..30 {
            std::fs::write(root.join(format!("f{i}.txt")), b"x").unwrap();
        }
        assert_eq!(scan(&root, 6, 10).len(), 10);
    }

    #[test]
    fn legacy_projects_are_found() {
        let root = tmp("legacy");
        std::fs::write(root.join("战役.sc2transproj"), b"{}").unwrap();
        std::fs::write(root.join("notes.txt"), b"x").unwrap();
        let found = legacy_projects(&root);
        assert_eq!(found.len(), 1);
    }
}
