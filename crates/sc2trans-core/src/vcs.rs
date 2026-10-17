//! SC2Diff 集成：地图 / 模组的 git 形状版本管理。
//!
//! 这一层的职责，以及为什么要有它：
//!
//! * **静默安装**：SC2Diff 是**内置**的（打包时随资源一起发），首次使用时把它复制到
//!   数据目录的 "tools/sc2diff/" 下 —— 不问用户、不弹浏览器、不要求装工具链。
//! * **参数形状收口**：SC2Diff 的用法是 "sc2diff -C <repo> <命令>"，
//!   而且 "-C" 只告诉它仓库在哪，解包类命令是往**进程当前目录**写文件的，
//!   所以必须同时设子进程的 current_dir。散在业务里拼命令行迟早出错，统一收在这里。
//! * **解析**：它输出的是人读文本，这里解析成结构化数据给 GUI 用；
//!   同时给出「等价命令」，让习惯命令行的人能原样复制（CLI 兼容）。
//!
//! 参考了同作者启动器 HSCL 里 crates/miyin-core/src/tools/sc2diff.rs 的做法。

use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Serialize;

use crate::{CoreError, Result};

/// 工具标识，同时是安装目录名。
pub const TOOL_ID: &str = "sc2diff";
/// 可执行文件名。
pub const BINARY: &str = "sc2diff.exe";

/// 一次命令的输出。
#[derive(Debug, Clone, Serialize)]
pub struct Output {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
    /// 等价命令行（去掉仓库路径前缀后的可复制形式）。
    pub command: String,
}

impl Output {
    pub fn ok(&self) -> bool {
        self.code == 0
    }

    /// 两路输出拼起来 —— SC2Diff 有的信息走 stderr。
    pub fn combined(&self) -> String {
        let mut text = self.stdout.clone();
        if !self.stderr.trim().is_empty() {
            if !text.is_empty() {
                text.push('\n');
            }
            text.push_str(&self.stderr);
        }
        text.trim().to_string()
    }
}

/// 工具从哪来的。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallSource {
    /// 之前已经装好了。
    Installed,
    /// 本次从随包资源里静默安装。
    Bundled,
    /// 本次从开发目录静默安装（源码树旁边的 target/release）。
    DevPath,
    /// 没找到。
    Missing,
}

/// 工具状态。
#[derive(Debug, Clone, Serialize)]
pub struct ToolStatus {
    pub id: String,
    pub name: String,
    pub installed: bool,
    pub version: Option<String>,
    pub path: Option<String>,
    pub source: InstallSource,
    pub size_bytes: u64,
    /// 给界面看的说明（没问题时是 None，不打扰用户）。
    pub message: Option<String>,
}

/// 安装目录。
pub fn tool_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("tools").join(TOOL_ID)
}

/// 已安装的可执行文件位置。
pub fn installed_path(data_dir: &Path) -> PathBuf {
    tool_dir(data_dir).join(BINARY)
}

/// 找一个能用的 sc2diff：先看装好的，再看随包资源，最后看开发目录。
pub fn locate(data_dir: &Path) -> Option<PathBuf> {
    let installed = installed_path(data_dir);
    if installed.is_file() {
        return Some(installed);
    }
    if let Ok(env) = std::env::var("SC2TRANS_SC2DIFF") {
        let p = PathBuf::from(env);
        if p.is_file() {
            return Some(p);
        }
    }
    bundled_candidates().into_iter().find(|p| p.is_file())
}

/// 随包资源与开发目录里的候选位置。
fn bundled_candidates() -> Vec<PathBuf> {
    let mut out = Vec::new();
    // 1) 可执行文件同级 / resources 子目录（Tauri 的 resource_dir 布局）
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            out.push(dir.join(BINARY));
            out.push(dir.join("resources").join(BINARY));
            out.push(dir.join("tools").join(BINARY));
            // 2) 开发时：target/release/SC2Translater.exe -> ../../SC2Diff/target/release/sc2diff.exe
            if let Some(target) = dir.parent() {
                if let Some(repo_root) = target.parent() {
                    if let Some(code) = repo_root.parent() {
                        out.push(code.join("SC2Diff").join("target").join("release").join(BINARY));
                        out.push(code.join("SC2Diff").join("target").join("debug").join(BINARY));
                    }
                }
            }
        }
    }
    out
}

/// 静默安装：把随包/开发目录里的 sc2diff 复制到数据目录。
///
/// 不弹任何界面、不联网。装不上就如实返回 —— 调用方据此把版本管理功能标成不可用，
/// 而不是报一堆红字。
pub fn ensure(data_dir: &Path, bundled_dir: Option<&Path>) -> ToolStatus {
    let installed = installed_path(data_dir);
    if installed.is_file() {
        return status_of(&installed, InstallSource::Installed);
    }

    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(dir) = bundled_dir {
        candidates.push(dir.join(BINARY));
        candidates.push(dir.join("resources").join(BINARY));
    }
    candidates.extend(bundled_candidates());

    let dev_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../SC2Diff");
    candidates.push(dev_root.join("target").join("release").join(BINARY));
    candidates.push(dev_root.join("target").join("debug").join(BINARY));

    let Some(source) = candidates.into_iter().find(|p| p.is_file()) else {
        return ToolStatus {
            id: TOOL_ID.to_string(),
            name: "SC2Diff".to_string(),
            installed: false,
            version: None,
            path: None,
            source: InstallSource::Missing,
            size_bytes: 0,
            message: Some("没找到内置的 SC2Diff —— 版本管理暂时不可用（重装本程序可以恢复）".to_string()),
        };
    };

    let dir = tool_dir(data_dir);
    if let Err(e) = std::fs::create_dir_all(&dir) {
        return ToolStatus {
            id: TOOL_ID.to_string(),
            name: "SC2Diff".to_string(),
            installed: false,
            version: None,
            path: Some(source.display().to_string()),
            source: InstallSource::Missing,
            size_bytes: 0,
            message: Some(format!("创建工具目录失败：{e}")),
        };
    }
    if let Err(e) = std::fs::copy(&source, &installed) {
        return ToolStatus {
            id: TOOL_ID.to_string(),
            name: "SC2Diff".to_string(),
            installed: false,
            version: None,
            path: Some(source.display().to_string()),
            source: InstallSource::Missing,
            size_bytes: 0,
            message: Some(format!("复制 SC2Diff 失败：{e}")),
        };
    }

    let origin = if dev_root.join("target").join("release").join(BINARY) == source {
        InstallSource::DevPath
    } else {
        InstallSource::Bundled
    };
    status_of(&installed, origin)
}

fn status_of(path: &Path, source: InstallSource) -> ToolStatus {
    let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    let version = version_of(path);
    ToolStatus {
        id: TOOL_ID.to_string(),
        name: "SC2Diff".to_string(),
        installed: true,
        version,
        path: Some(path.display().to_string()),
        source,
        size_bytes: size,
        message: None,
    }
}

fn version_of(path: &Path) -> Option<String> {
    let mut cmd = Command::new(path);
    cmd.arg("--version");
    hide_console(&mut cmd);
    let out = cmd.output().ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    Some(text.trim().trim_start_matches("sc2diff").trim().to_string())
}

/// Windows 上别让控制台程序弹窗。
fn hide_console(cmd: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    #[cfg(not(windows))]
    {
        let _ = cmd;
    }
}

/// 跑一条 SC2Diff 命令。
///
/// repo 传 Some 时加 "-C" **并把它同时设为子进程工作目录** —— 两件事都得做：
/// "-C" 只告诉它仓库在哪，而 unpack 之类是往进程当前目录写文件的。
/// 用子进程的 current_dir 而不是改本进程的 CWD：后者是全局状态，并发会互相踩。
pub fn run(data_dir: &Path, repo: Option<&Path>, args: &[&str]) -> Result<Output> {
    let binary = locate(data_dir)
        .ok_or_else(|| CoreError::Invalid("还没装 SC2Diff —— 版本管理暂时不可用".to_string()))?;

    let mut cmd = Command::new(&binary);
    if let Some(repo) = repo {
        cmd.arg("-C").arg(repo);
        if repo.is_dir() {
            cmd.current_dir(repo);
        }
    }
    cmd.args(args);
    hide_console(&mut cmd);
    cmd.env("LC_ALL", "C.UTF-8");

    let out = cmd
        .output()
        .map_err(|e| CoreError::Invalid(format!("启动 SC2Diff 失败：{e}")))?;

    let mut command = String::from("sc2diff");
    if let Some(repo) = repo {
        command.push_str(&format!(" -C \"{}\"", repo.display()));
    }
    for a in args {
        if a.contains(' ') {
            command.push_str(&format!(" \"{a}\""));
        } else {
            command.push(' ');
            command.push_str(a);
        }
    }

    Ok(Output {
        code: out.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        command,
    })
}

/// 跑一条命令，非 0 就报错（把原始输出带回去，别吞）。
pub fn run_checked(data_dir: &Path, repo: Option<&Path>, args: &[&str]) -> Result<Output> {
    let out = run(data_dir, repo, args)?;
    if !out.ok() {
        return Err(CoreError::Invalid(format!(
            "sc2diff {} 失败：{}",
            args.join(" "),
            out.combined()
        )));
    }
    Ok(out)
}

// --------------------------------------------------------------------------- //
// 解析
// --------------------------------------------------------------------------- //

/// 一条变更。
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Change {
    /// 暂存区状态（X）。
    pub index: String,
    /// 工作树状态（Y）。
    pub worktree: String,
    /// 组件名。
    pub name: String,
}

impl Change {
    /// 是不是未跟踪。
    pub fn untracked(&self) -> bool {
        self.index == "?" || self.worktree == "?"
    }
    /// 暂存区里有改动。
    pub fn staged(&self) -> bool {
        !self.untracked() && self.index != " " && !self.index.is_empty()
    }
    /// 工作树里有改动。
    pub fn dirty(&self) -> bool {
        !self.untracked() && self.worktree != " " && !self.worktree.is_empty()
    }
    pub fn label(&self) -> String {
        if self.untracked() {
            return "未跟踪".to_string();
        }
        let code = if self.staged() {
            self.index.chars().next().unwrap_or(' ')
        } else {
            self.worktree.chars().next().unwrap_or(' ')
        };
        match code {
            'A' => "新增",
            'M' => "修改",
            'D' => "删除",
            'R' => "重命名",
            'C' => "复制",
            _ => "改动",
        }
        .to_string()
    }
}

/// 仓库状态。
#[derive(Debug, Clone, Serialize)]
pub struct RepoStatus {
    /// 是不是一个 sc2diff 仓库。
    pub is_repo: bool,
    /// 当前分支。
    pub branch: String,
    /// 有没有提交。
    pub has_commits: bool,
    pub changes: Vec<Change>,
    pub staged: usize,
    pub dirty: usize,
    pub untracked: usize,
    /// 原始长格式输出，界面上「原始输出」折叠区用。
    pub raw: String,
}

/// 解析 "status -s" 的短格式：XY + 空格 + 名字。
pub fn parse_short_status(text: &str) -> Vec<Change> {
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim_end_matches(['\r', '\n']);
        if line.trim().is_empty() {
            continue;
        }
        // 形如 " M Objects" / "A  Foo" / "?? Bar"
        let mut chars = line.chars();
        let x = chars.next().unwrap_or(' ');
        let y = chars.next().unwrap_or(' ');
        let rest = chars.as_str();
        if x == ' ' && y == ' ' && rest.trim().is_empty() {
            continue;
        }
        let name = rest.trim();
        if name.is_empty() {
            continue;
        }
        out.push(Change {
            index: x.to_string(),
            worktree: y.to_string(),
            name: name.to_string(),
        });
    }
    out
}

/// 从 "On branch main" 里取分支名。
pub fn parse_branch_line(text: &str) -> Option<String> {
    text.lines()
        .find_map(|l| l.trim().strip_prefix("On branch ").map(|s| s.trim().to_string()))
}

/// 当前分支（从 branch 命令的第一行 "* main" 取）。
pub fn parse_current_branch(text: &str) -> Option<String> {
    text.lines()
        .find(|l| l.trim_start().starts_with('*'))
        .map(|l| l.trim_start_matches(['*', ' ']).trim().to_string())
}

/// 一条提交。
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Commit {
    pub sha: String,
    pub short: String,
    pub author: String,
    pub date: String,
    pub message: String,
}

/// 解析 "log --oneline"：每行 "<sha> <message>"。
pub fn parse_oneline_log(text: &str) -> Vec<Commit> {
    text.lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() {
                return None;
            }
            let mut it = line.splitn(2, char::is_whitespace);
            let sha = it.next()?.to_string();
            let message = it.next().unwrap_or("").trim().to_string();
            Some(Commit {
                short: sha.chars().take(8).collect(),
                sha,
                author: String::new(),
                date: String::new(),
                message,
            })
        })
        .collect()
}

/// 解析完整 log 的块格式。
pub fn parse_full_log(text: &str) -> Vec<Commit> {
    let mut out: Vec<Commit> = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim_end();
        if let Some(sha) = trimmed.strip_prefix("commit ") {
            out.push(Commit {
                sha: sha.trim().to_string(),
                short: sha.trim().chars().take(8).collect(),
                author: String::new(),
                date: String::new(),
                message: String::new(),
            });
        } else if let Some(rest) = trimmed.strip_prefix("Author:") {
            if let Some(c) = out.last_mut() {
                c.author = rest.trim().to_string();
            }
        } else if let Some(rest) = trimmed.strip_prefix("Date:") {
            if let Some(c) = out.last_mut() {
                c.date = rest.trim().to_string();
            }
        } else if !trimmed.trim().is_empty() {
            if let Some(c) = out.last_mut() {
                if c.message.is_empty() {
                    c.message = trimmed.trim().to_string();
                } else {
                    c.message.push('\n');
                    c.message.push_str(trimmed.trim());
                }
            }
        }
    }
    out
}

// --------------------------------------------------------------------------- //
// 组合操作
// --------------------------------------------------------------------------- //

/// 一个项目文档对应的仓库目录。
///
/// 约定：仓库跟着项目文件走，放在 "<项目名>.repos/<文档名>/" 下 ——
/// 一个 sc2diff 仓库只跟踪一份文档（它的工作树就是那份文档的组件），
/// 所以项目里有几个地图就有几个仓库，彼此不干扰。
pub fn repo_dir_for(project_file: &Path, document: &Path) -> PathBuf {
    let project_stem = project_file
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("project");
    let doc_stem = document
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("document");
    let dir = project_file.parent().unwrap_or(Path::new("."));
    dir.join(format!("{project_stem}.repos")).join(doc_stem)
}

/// 这里是不是一个 sc2diff 仓库（有 .sc2diff 目录）。
pub fn is_repo(repo: &Path) -> bool {
    repo.join(".sc2diff").is_dir()
}

/// 仓库状态：分支 + 变更。
pub fn status(data_dir: &Path, repo: &Path) -> Result<RepoStatus> {
    if !is_repo(repo) {
        return Ok(RepoStatus {
            is_repo: false,
            branch: String::new(),
            has_commits: false,
            changes: Vec::new(),
            staged: 0,
            dirty: 0,
            untracked: 0,
            raw: String::new(),
        });
    }
    let long = run_checked(data_dir, Some(repo), &["status"])?;
    let short = run_checked(data_dir, Some(repo), &["status", "-s"])?;
    let branch_out = run_checked(data_dir, Some(repo), &["branch"])?;

    let raw = long.combined();
    let changes = parse_short_status(&short.stdout);
    let branch = parse_branch_line(&raw)
        .or_else(|| parse_current_branch(&branch_out.stdout))
        .unwrap_or_else(|| "main".to_string());
    let has_commits = !raw.contains("No commits yet");

    let staged = changes.iter().filter(|c| c.staged()).count();
    let dirty = changes.iter().filter(|c| c.dirty()).count();
    let untracked = changes.iter().filter(|c| c.untracked()).count();

    Ok(RepoStatus {
        is_repo: true,
        branch,
        has_commits,
        changes,
        staged,
        dirty,
        untracked,
        raw,
    })
}

/// 提交历史。
///
/// 注意：sc2diff 的 log 只接受 [REV]，**没有 -n** —— 传 -10 会被 clap 当成
/// 未知参数直接报错（实测）。所以取全量再在本地截断。
pub fn log(data_dir: &Path, repo: &Path, limit: usize) -> Result<Vec<Commit>> {
    let out = run_checked(data_dir, Some(repo), &["log", "--oneline"])?;
    let mut commits = parse_oneline_log(&out.stdout);
    commits.truncate(limit);
    Ok(commits)
}

/// 分支列表与当前分支。
pub fn branches(data_dir: &Path, repo: &Path) -> Result<Vec<String>> {
    let out = run_checked(data_dir, Some(repo), &["branch"])?;
    Ok(out
        .stdout
        .lines()
        .map(|l| l.trim_start_matches(['*', ' ']).trim().to_string())
        .filter(|s| !s.is_empty())
        .collect())
}

/// 标签列表。
pub fn tags(data_dir: &Path, repo: &Path) -> Result<Vec<String>> {
    let out = run_checked(data_dir, Some(repo), &["tag"])?;
    Ok(out
        .stdout
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect())
}

/// 建仓（可选导入一份文档）。
pub fn init(data_dir: &Path, repo: &Path, document: Option<&Path>) -> Result<String> {
    std::fs::create_dir_all(repo)?;
    let doc = document.map(|d| d.display().to_string());
    let mut args: Vec<&str> = vec!["init"];
    if let Some(ref d) = doc {
        args.push(d);
    }
    Ok(run_checked(data_dir, Some(repo), &args)?.combined())
}

/// 暂存全部改动。
pub fn add_all(data_dir: &Path, repo: &Path) -> Result<String> {
    Ok(run_checked(data_dir, Some(repo), &["add", "-A"])?.combined())
}

/// 提交。
pub fn commit(data_dir: &Path, repo: &Path, message: &str) -> Result<String> {
    Ok(run_checked(data_dir, Some(repo), &["commit", "-m", message])?.combined())
}

/// 语义 diff。两个版本都给了就比它们，否则比「工作树 vs 索引」。
pub fn diff(data_dir: &Path, repo: &Path, from: Option<&str>, to: Option<&str>) -> Result<String> {
    let mut args: Vec<&str> = vec!["diff"];
    if let Some(f) = from {
        args.push(f);
    }
    if let Some(t) = to {
        args.push(t);
    }
    Ok(run_checked(data_dir, Some(repo), &args)?.combined())
}

/// 打包回文档。
pub fn pack(data_dir: &Path, repo: &Path, out: &Path) -> Result<String> {
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let target = out.display().to_string();
    Ok(run_checked(data_dir, Some(repo), &["pack", &target])?.combined())
}

/// 解包一份文档到工作树。
pub fn unpack(data_dir: &Path, repo: &Path, document: &Path) -> Result<String> {
    let doc = document.display().to_string();
    Ok(run_checked(data_dir, Some(repo), &["unpack", &doc])?.combined())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_short_status() {
        let text = " M Objects\nA  Triggers\n?? NewFile\nD  Old\n";
        let changes = parse_short_status(text);
        assert_eq!(changes.len(), 4);
        assert!(changes[0].dirty() && !changes[0].staged());
        assert!(changes[1].staged());
        assert!(changes[2].untracked());
        assert_eq!(changes[3].label(), "删除");
    }

    #[test]
    fn parses_branch_lines() {
        assert_eq!(parse_branch_line("On branch dev\n\nclean"), Some("dev".into()));
        assert_eq!(parse_current_branch("* main\n  dev\n"), Some("main".into()));
        assert_eq!(parse_branch_line("no branch here"), None);
    }

    #[test]
    fn parses_oneline_log() {
        let text = "29645933 import\n63e4df69 change Objects\n";
        let commits = parse_oneline_log(text);
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].short, "29645933");
        assert_eq!(commits[1].message, "change Objects");
    }

    #[test]
    fn parses_full_log_blocks() {
        let text = "commit 63e4df69b97bf3f0\nAuthor: sc2diff\nDate:   1791511789\n\n    change Objects\n";
        let commits = parse_full_log(text);
        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].sha, "63e4df69b97bf3f0");
        assert_eq!(commits[0].author, "sc2diff");
        assert_eq!(commits[0].message, "change Objects");
    }

    #[test]
    fn tool_dir_layout() {
        let dir = std::env::temp_dir();
        assert!(installed_path(&dir).ends_with("tools\\sc2diff\\sc2diff.exe"));
    }

    #[test]
    fn is_repo_detects_marker() {
        let dir = std::env::temp_dir().join("sc2trans_vcs_test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        assert!(!is_repo(&dir));
        std::fs::create_dir_all(dir.join(".sc2diff")).unwrap();
        assert!(is_repo(&dir));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 真的调用内置的 sc2diff 走一遍 init -> status -> commit -> log -> tag -> pack。
    /// 找不到二进制就跳过（CI 上不该因为缺工具而红）。
    #[test]
    fn end_to_end_with_real_sc2diff() {
        let data = std::env::temp_dir().join("sc2trans_vcs_data");
        std::fs::create_dir_all(&data).unwrap();
        let tool = ensure(&data, None);
        if !tool.installed {
            eprintln!("跳过：没有可用的 sc2diff（{}）", tool.message.unwrap_or_default());
            return;
        }
        assert!(tool.version.is_some(), "应当能问到版本号");

        let repo = std::env::temp_dir().join("sc2trans_vcs_repo");
        let _ = std::fs::remove_dir_all(&repo);
        std::fs::create_dir_all(&repo).unwrap();

        // 找一份样本文档
        let samples = [
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../SC2Diff/testdata/fixture.SC2Map"),
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../../SC2Diff/testdata/samples/SCORE-OtherC.SC2Mod"),
        ];
        let Some(doc) = samples.into_iter().find(|p| p.is_file()) else {
            eprintln!("跳过：没有样本文档");
            return;
        };

        init(&data, &repo, Some(&doc)).unwrap();
        let st = status(&data, &repo).unwrap();
        assert!(st.is_repo);
        assert!(!st.changes.is_empty(), "刚导入应当有变更");

        add_all(&data, &repo).unwrap();
        let out = commit(&data, &repo, "import").unwrap();
        assert!(out.contains("import"));

        let commits = log(&data, &repo, 10).unwrap();
        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].message, "import");

        let st2 = status(&data, &repo).unwrap();
        assert!(
            st2.changes.is_empty(),
            "提交后工作树应当是干净的: {:?}",
            st2.changes
        );

        run_checked(&data, Some(&repo), &["tag", "v0.1"]).unwrap();
        assert_eq!(tags(&data, &repo).unwrap(), vec!["v0.1".to_string()]);
        assert!(branches(&data, &repo).unwrap().contains(&"main".to_string()));

        let packed = std::env::temp_dir().join("sc2trans_vcs_packed.SC2Map");
        let report = pack(&data, &repo, &packed).unwrap();
        assert!(packed.is_file());
        assert!(report.contains("digests"), "pack 会打印摘要校验结果: {report}");

        let _ = std::fs::remove_dir_all(&repo);
        let _ = std::fs::remove_file(&packed);
    }
}
