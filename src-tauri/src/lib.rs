//! SC2Translater 的 Tauri 2 桌面壳。
//!
//! 这一层**只做状态持有与命令转发**：所有业务都在 sc2trans-core 里，GUI 与 CLI 共用同一套实现。
//!
//! 约定（沿用参考项目 SC2Miyin Launcher）：
//!
//! * 每个 #[tauri::command] 一律是 async，且重活走 spawn_blocking —— 否则解压一个 44MB 的
//!   模组会把 UI 线程卡死，用户看到的是「点了没反应」。
//! * 错误一律转成 String 回传前端，前端直接显示；不吞错。
//! * 文件选择用 rfd（原生对话框），前端不接触绝对路径。

use std::path::{Path, PathBuf};

use serde_json::Value;
use tauri::{Manager, command};

use sc2trans_core::ops::{self, ApplyOptions};
use sc2trans_core::project::{Project, RecentProjects};

/// 把阻塞的重活丢到线程池，避免卡住 UI。
async fn blocking<T, F>(f: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, String> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| format!("后台任务异常: {e}"))?
}

/// 应用数据目录：放最近项目列表。
fn data_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("取应用数据目录失败: {e}"))?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("建应用数据目录失败: {e}"))?;
    Ok(dir)
}

/// 最近打开的项目（已过滤掉磁盘上不存在的）。
#[command]
async fn recent_projects(app: tauri::AppHandle) -> Result<Value, String> {
    let dir = data_dir(&app)?;
    blocking(move || {
        let mut recent = RecentProjects::load(&dir);
        recent.prune();
        let _ = recent.save(&dir);
        serde_json::to_value(recent).map_err(|e| format!("{e:#}"))
    })
    .await
}

/// 从最近列表里移除一个项目。
#[command]
async fn forget_project(app: tauri::AppHandle, path: String) -> Result<(), String> {
    let dir = data_dir(&app)?;
    blocking(move || {
        let mut recent = RecentProjects::load(&dir);
        recent.forget(&path);
        recent.save(&dir).map_err(|e| format!("{e:#}"))
    })
    .await
}

/// 选择项目文件要保存到哪里。
#[command]
async fn pick_project_save(default_name: String) -> Option<String> {
    tauri::async_runtime::spawn_blocking(move || {
        rfd::FileDialog::new()
            .set_title("保存项目")
            .set_file_name(default_name)
            .add_filter("SC2Translater 项目", &[sc2trans_core::project::PROJECT_EXT])
            .save_file()
            .map(|p| p.display().to_string())
    })
    .await
    .ok()
    .flatten()
}

/// 选择一个已有项目文件。
#[command]
async fn pick_project_open() -> Option<String> {
    tauri::async_runtime::spawn_blocking(|| {
        rfd::FileDialog::new()
            .set_title("打开项目")
            .add_filter("SC2Translater 项目", &[sc2trans_core::project::PROJECT_EXT])
            .pick_file()
            .map(|p| p.display().to_string())
    })
    .await
    .ok()
    .flatten()
}

/// 一次选多个地图 / 模组加进项目。
#[command]
async fn pick_archives() -> Vec<String> {
    tauri::async_runtime::spawn_blocking(|| {
        rfd::FileDialog::new()
            .set_title("添加地图 / 模组（可多选）")
            .add_filter("星际争霸 II 文档", &["SC2Map", "SC2Mod", "sc2map", "sc2mod"])
            .pick_files()
            .map(|paths| paths.into_iter().map(|p| p.display().to_string()).collect())
            .unwrap_or_default()
    })
    .await
    .unwrap_or_default()
}

/// 保存项目（写 JSON 文件 + 记入最近列表）。
#[command]
async fn save_project(app: tauri::AppHandle, path: String, project: Value) -> Result<Value, String> {
    let dir = data_dir(&app)?;
    blocking(move || {
        let mut p: Project = serde_json::from_value(project).map_err(|e| format!("项目结构不对: {e}"))?;
        let target = PathBuf::from(&path);
        p.path = Some(target.display().to_string());
        p.normalize();
        p.save(&target).map_err(|e| format!("{e:#}"))?;
        let mut recent = RecentProjects::load(&dir);
        recent.touch(&p);
        let _ = recent.save(&dir);
        serde_json::to_value(&p).map_err(|e| format!("{e:#}"))
    })
    .await
}

/// 打开项目。
#[command]
async fn load_project(app: tauri::AppHandle, path: String) -> Result<Value, String> {
    let dir = data_dir(&app)?;
    blocking(move || {
        let p = Project::load(Path::new(&path)).map_err(|e| format!("{e:#}"))?;
        let mut recent = RecentProjects::load(&dir);
        recent.touch(&p);
        let _ = recent.save(&dir);
        serde_json::to_value(&p).map_err(|e| format!("{e:#}"))
    })
    .await
}

#[command]
async fn app_info() -> Value {
    serde_json::json!({
        "name": "SC2Translater",
        "version": env!("CARGO_PKG_VERSION"),
        "tool": sc2trans_core::TOOL_ID,
        "schema": sc2trans_core::SCHEMA_ID,
    })
}

/// 选择地图 / 模组归档。
#[command]
async fn pick_archive() -> Option<String> {
    tauri::async_runtime::spawn_blocking(|| {
        rfd::FileDialog::new()
            .set_title("选择星际争霸 II 地图或模组")
            .add_filter("星际争霸 II 文档", &["SC2Map", "SC2Mod", "sc2map", "sc2mod"])
            .pick_file()
            .map(|p| p.display().to_string())
    })
    .await
    .ok()
    .flatten()
}

/// 选择要保存的 JSON 路径。
#[command]
async fn pick_json_save(default_name: String) -> Option<String> {
    tauri::async_runtime::spawn_blocking(move || {
        rfd::FileDialog::new()
            .set_title("导出翻译键")
            .set_file_name(default_name)
            .add_filter("翻译键 JSON", &["json"])
            .save_file()
            .map(|p| p.display().to_string())
    })
    .await
    .ok()
    .flatten()
}

/// 选择要打开的翻译 JSON。
#[command]
async fn pick_json_open() -> Option<String> {
    tauri::async_runtime::spawn_blocking(|| {
        rfd::FileDialog::new()
            .set_title("选择翻译 JSON")
            .add_filter("JSON", &["json"])
            .pick_file()
            .map(|p| p.display().to_string())
    })
    .await
    .ok()
    .flatten()
}

/// 选择产物归档的保存位置。
#[command]
async fn pick_archive_save(default_name: String) -> Option<String> {
    tauri::async_runtime::spawn_blocking(move || {
        rfd::FileDialog::new()
            .set_title("保存汉化后的地图 / 模组")
            .set_file_name(default_name)
            .add_filter("星际争霸 II 文档", &["SC2Map", "SC2Mod"])
            .save_file()
            .map(|p| p.display().to_string())
    })
    .await
    .ok()
    .flatten()
}

/// 归档概览。
#[command]
async fn inspect(archive: String) -> Result<Value, String> {
    blocking(move || ops::inspect(Path::new(&archive)).map_err(|e| format!("{e:#}"))).await
}

/// 覆盖率审计（回答「为什么显示成键路径」）。
#[command]
async fn audit(archive: String, source: String) -> Result<Value, String> {
    blocking(move || ops::audit(Path::new(&archive), &source).map_err(|e| format!("{e:#}"))).await
}

/// 载入全部翻译键（表格数据源）。不落盘，直接把文档结构返回给前端。
#[command]
async fn load_keys(
    archive: String,
    source: String,
    target: String,
    all_locales: bool,
) -> Result<Value, String> {
    blocking(move || {
        let doc = ops::extract(Path::new(&archive), &source, &target, None, all_locales)
            .map_err(|e| format!("{e:#}"))?;
        serde_json::to_value(&doc).map_err(|e| format!("{e:#}"))
    })
    .await
}

/// 把内存里的文档导出成 JSON 文件。
#[command]
async fn export_document(doc: Value, out: String) -> Result<Value, String> {
    blocking(move || {
        let text = serde_json::to_string_pretty(&doc).map_err(|e| format!("{e:#}"))?;
        std::fs::write(&out, format!("{text}\n")).map_err(|e| format!("写出 {out} 失败: {e}"))?;
        Ok(serde_json::json!({ "path": out, "bytes": text.len() + 1 }))
    })
    .await
}

/// 读入一份翻译 JSON（导入外部译文用）。
#[command]
async fn import_document(path: String) -> Result<Value, String> {
    blocking(move || {
        let text =
            std::fs::read_to_string(&path).map_err(|e| format!("读取 {path} 失败: {e}"))?;
        let value: Value = serde_json::from_str(&text).map_err(|e| format!("解析 JSON 失败: {e}"))?;
        Ok(value)
    })
    .await
}

/// 把内存里的文档写回归档（GUI 的「写回地图」）。
#[command]
async fn apply_document(
    archive: String,
    doc: Value,
    out: String,
    with_original: bool,
    sort_keys: bool,
) -> Result<Value, String> {
    blocking(move || {
        let opts = ApplyOptions {
            out: Some(PathBuf::from(&out)),
            with_original,
            sort: sort_keys,
            ..Default::default()
        };
        let result = ops::apply(Path::new(&archive), &doc, &opts).map_err(|e| format!("{e:#}"))?;
        serde_json::to_value(&result).map_err(|e| format!("{e:#}"))
    })
    .await
}

/// 把翻译 JSON 写回归档。
#[command]
#[allow(clippy::too_many_arguments)]
async fn apply(
    archive: String,
    json_path: String,
    out: String,
    target: Option<String>,
    fill_missing: bool,
    with_original: bool,
    sort_keys: bool,
) -> Result<Value, String> {
    blocking(move || {
        let text = std::fs::read_to_string(&json_path)
            .map_err(|e| format!("读取 {json_path} 失败: {e}"))?;
        let raw: Value = serde_json::from_str(&text).map_err(|e| format!("解析 JSON 失败: {e}"))?;
        let opts = ApplyOptions {
            out: Some(PathBuf::from(&out)),
            target_locale: target,
            fill_missing,
            with_original,
            sort: sort_keys,
            ..Default::default()
        };
        let result = ops::apply(Path::new(&archive), &raw, &opts).map_err(|e| format!("{e:#}"))?;
        serde_json::to_value(&result).map_err(|e| format!("{e:#}"))
    })
    .await
}

/// 一键兜底：把原文整份补进目标语言。
#[command]
async fn seed(
    archive: String,
    source: String,
    target: String,
    out: String,
    with_original: bool,
    sort_keys: bool,
) -> Result<Value, String> {
    blocking(move || {
        let opts = ApplyOptions {
            out: Some(PathBuf::from(&out)),
            with_original,
            sort: sort_keys,
            ..Default::default()
        };
        let result = ops::seed(Path::new(&archive), &source, &target, &opts)
            .map_err(|e| format!("{e:#}"))?;
        serde_json::to_value(&result).map_err(|e| format!("{e:#}"))
    })
    .await
}

/// 补全默认语言：用指定语言的文本覆盖「没有语言文件」的语言。
#[command]
async fn fill_locales(
    archive: String,
    source: String,
    locales: String,
    only_missing: bool,
    with_original: bool,
    out: String,
) -> Result<Value, String> {
    blocking(move || {
        let targets = ops::FillTargets::parse(&locales).map_err(|e| format!("{e:#}"))?;
        let result = ops::fill_locales(
            Path::new(&archive),
            &source,
            &targets,
            only_missing,
            with_original,
            Some(Path::new(&out)),
            false,
        )
        .map_err(|e| format!("{e:#}"))?;
        serde_json::to_value(&result).map_err(|e| format!("{e:#}"))
    })
    .await
}

/// 容器自检。
#[command]
async fn verify(archive: String, deep: bool) -> Result<Value, String> {
    blocking(move || ops::verify(Path::new(&archive), deep).map_err(|e| format!("{e:#}"))).await
}

/// 在资源管理器里定位产物 —— 用户下一步通常就是把它拖进游戏目录。
#[command]
async fn reveal(path: String) -> Result<(), String> {
    blocking(move || {
        let p = PathBuf::from(&path);
        let target = if p.is_file() {
            p.parent().map(|x| x.to_path_buf()).unwrap_or(p.clone())
        } else {
            p
        };
        std::process::Command::new("explorer")
            .arg(target)
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("打开资源管理器失败: {e}"))
    })
    .await
}

/// 默认的产物文件名：<原名>.<目标语言>.<扩展名>
#[command]
async fn default_output_name(archive: String, target: String) -> String {
    let p = Path::new(&archive);
    let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("out");
    let ext = p.extension().and_then(|s| s.to_str()).unwrap_or("SC2Map");
    format!("{stem}.{target}.{ext}")
}

/// 默认的导出 JSON 文件名。
#[command]
async fn default_json_name(archive: String, target: String) -> String {
    let p = Path::new(&archive);
    let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("keys");
    format!("{stem}.{target}.keys.json")
}

// =========================================================================== //
// 专有名词库 / 机翻参考 / 版本管理（内置 SC2Diff）
// =========================================================================== //

use sc2trans_core::glossary::{self, Glossary, Scope};
use sc2trans_core::mt::{MtConfig, MtEngine};
use sc2trans_core::vcs;

/// 读全局 + 项目两份术语库。
#[command]
async fn glossary_load(app: tauri::AppHandle, project_file: Option<String>) -> Result<Value, String> {
    let dir = data_dir(&app)?;
    blocking(move || {
        let global_path = glossary::global_path(&dir);
        let global = if global_path.is_file() {
            Glossary::load(&global_path).unwrap_or_else(|_| Glossary::new("全局术语库"))
        } else {
            Glossary::new("全局术语库")
        };
        let mut project = Glossary::new("项目术语库");
        let mut project_path = None;
        if let Some(pf) = project_file.as_ref() {
            let path = glossary::project_path(Path::new(pf));
            if path.is_file() {
                project = Glossary::load(&path).unwrap_or_else(|_| Glossary::new("项目术语库"));
            }
            project_path = Some(path.display().to_string());
        }
        Ok(serde_json::json!({
            "global": global,
            "project": project,
            "global_path": global_path.display().to_string(),
            "project_path": project_path,
        }))
    })
    .await
}

/// 保存某一份术语库。scope = global | project。
#[command]
async fn glossary_save(
    app: tauri::AppHandle,
    scope: String,
    glossary: Value,
    project_file: Option<String>,
) -> Result<Value, String> {
    let dir = data_dir(&app)?;
    blocking(move || {
        let mut g: Glossary =
            serde_json::from_value(glossary).map_err(|e| format!("术语库结构不对: {e}"))?;
        g.sort();
        let path = match scope.as_str() {
            "global" => glossary::global_path(&dir),
            "project" => {
                let pf = project_file
                    .as_ref()
                    .ok_or_else(|| "项目术语库需要先保存项目（Ctrl+S）".to_string())?;
                glossary::project_path(Path::new(pf))
            }
            other => return Err(format!("未知的作用域: {other}")),
        };
        g.save(&path).map_err(|e| format!("{e:#}"))?;
        Ok(serde_json::json!({ "path": path.display().to_string(), "terms": g.terms.len() }))
    })
    .await
}

/// 选一个术语库文件（导入 / 导出共用）。
#[command]
async fn glossary_pick(save: bool, default_name: String) -> Option<String> {
    tauri::async_runtime::spawn_blocking(move || {
        let dialog = rfd::FileDialog::new()
            .set_title(if save { "导出术语库" } else { "导入术语库" })
            .add_filter("术语库", &["json", "csv"]);
        if save {
            dialog.set_file_name(default_name).save_file()
        } else {
            dialog.pick_file()
        }
        .map(|p| p.display().to_string())
    })
    .await
    .ok()
    .flatten()
}

/// 读一个术语库文件：按扩展名分派 JSON / CSV。
#[command]
async fn glossary_read(path: String, scope: String) -> Result<Value, String> {
    blocking(move || {
        let text = std::fs::read_to_string(&path).map_err(|e| format!("读取 {path} 失败: {e}"))?;
        let is_csv = path.to_ascii_lowercase().ends_with(".csv");
        let st = if scope == "project" { Scope::Project } else { Scope::Global };
        if is_csv {
            let mut g = Glossary::new("导入");
            let n = g.import_csv(&text, st);
            Ok(serde_json::json!({ "terms": g.terms, "count": n, "format": "csv" }))
        } else {
            let g = Glossary::from_json(&text).map_err(|e| format!("{e:#}"))?;
            let n = g.terms.len();
            Ok(serde_json::json!({ "terms": g.terms, "count": n, "format": "json" }))
        }
    })
    .await
}

/// 写一个术语库文件：按扩展名分派 JSON / CSV。
#[command]
async fn glossary_write(path: String, glossary: Value) -> Result<Value, String> {
    blocking(move || {
        let mut g: Glossary =
            serde_json::from_value(glossary).map_err(|e| format!("术语库结构不对: {e}"))?;
        g.sort();
        let body = if path.to_ascii_lowercase().ends_with(".csv") {
            g.export_csv()
        } else {
            g.to_json().map_err(|e| format!("{e:#}"))?
        };
        if let Some(parent) = Path::new(&path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(&path, body).map_err(|e| format!("写出 {path} 失败: {e}"))?;
        Ok(serde_json::json!({ "path": path, "terms": g.terms.len() }))
    })
    .await
}

#[command]
async fn mt_config_get(app: tauri::AppHandle) -> Result<Value, String> {
    let dir = data_dir(&app)?;
    blocking(move || {
        let cfg = MtConfig::load(&dir);
        serde_json::to_value(cfg).map_err(|e| format!("{e:#}"))
    })
    .await
}

#[command]
async fn mt_config_set(app: tauri::AppHandle, config: Value) -> Result<Value, String> {
    let dir = data_dir(&app)?;
    blocking(move || {
        let cfg: MtConfig =
            serde_json::from_value(config).map_err(|e| format!("机翻配置不对: {e}"))?;
        cfg.save(&dir).map_err(|e| format!("{e:#}"))?;
        serde_json::to_value(cfg).map_err(|e| format!("{e:#}"))
    })
    .await
}

/// 机翻一条（只返回参考，绝不写进键里）。
#[command]
async fn mt_translate(
    app: tauri::AppHandle,
    text: String,
    project_file: Option<String>,
) -> Result<Value, String> {
    let dir = data_dir(&app)?;
    blocking(move || {
        let pair = load_glossary_pair(&dir, project_file.as_deref());
        let cfg = MtConfig::load(&dir);
        let mut engine = MtEngine::new(cfg, &dir);
        let s = engine.translate(&text, &pair).map_err(|e| format!("{e:#}"))?;
        serde_json::to_value(s).map_err(|e| format!("{e:#}"))
    })
    .await
}

/// 机翻一批（按需触发，不自动跑）。
#[command]
async fn mt_translate_many(
    app: tauri::AppHandle,
    texts: Vec<String>,
    project_file: Option<String>,
) -> Result<Value, String> {
    let dir = data_dir(&app)?;
    blocking(move || {
        let pair = load_glossary_pair(&dir, project_file.as_deref());
        let cfg = MtConfig::load(&dir);
        let mut engine = MtEngine::new(cfg, &dir);
        let mut out = Vec::new();
        for t in &texts {
            match engine.translate(t, &pair) {
                Ok(s) => out.push(serde_json::json!({ "ok": true, "suggestion": s })),
                Err(e) => out.push(serde_json::json!({ "ok": false, "error": format!("{e:#}") })),
            }
        }
        Ok(serde_json::json!({ "results": out }))
    })
    .await
}

#[command]
async fn mt_clear_cache(app: tauri::AppHandle) -> Result<Value, String> {
    let dir = data_dir(&app)?;
    blocking(move || {
        let cfg = MtConfig::load(&dir);
        let mut engine = MtEngine::new(cfg, &dir);
        let n = engine.cache_len();
        engine.clear_cache();
        Ok(serde_json::json!({ "cleared": n }))
    })
    .await
}

fn load_glossary_pair(dir: &Path, project_file: Option<&str>) -> glossary::GlossaryPair {
    let mut pair = glossary::GlossaryPair::default();
    let gp = glossary::global_path(dir);
    if gp.is_file() {
        if let Ok(g) = Glossary::load(&gp) {
            pair.global = g;
        }
    }
    if let Some(pf) = project_file {
        let pp = glossary::project_path(Path::new(pf));
        if pp.is_file() {
            if let Ok(g) = Glossary::load(&pp) {
                pair.project = g;
            }
        }
    }
    pair
}

/// 扫工程目录，列出我们认得的文件（有深度与条目上限，不扫 node_modules 这类）。
#[command]
async fn scan_folder(root: String) -> Result<Value, String> {
    blocking(move || {
        use sc2trans_core::filekind;
        const MAX_DEPTH: usize = 6;
        const MAX_ENTRIES: usize = 2000;
        const SKIP: [&str; 6] = [".sc2translater", ".git", "node_modules", "target", ".idea", "__pycache__"];

        let base = PathBuf::from(&root);
        let mut out: Vec<Value> = Vec::new();
        let mut stack = vec![(base.clone(), 0usize)];
        while let Some((dir, depth)) = stack.pop() {
            if depth > MAX_DEPTH || out.len() >= MAX_ENTRIES {
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
                let rel = path
                    .strip_prefix(&base)
                    .map(|p| p.to_string_lossy().replace('\\', "/"))
                    .unwrap_or_else(|_| name.clone());
                let linked = if c.kind == filekind::FileKind::Sc2 {
                    match sc2trans_core::link::link(&path) {
                        sc2trans_core::link::Link::Linked { path } => {
                            Some(path.strip_prefix(&base).map(|p| p.to_string_lossy().replace('\\', "/")).unwrap_or_default())
                        }
                        _ => None,
                    }
                } else {
                    None
                };
                out.push(serde_json::json!({
                    "path": rel,
                    "kind": c.kind,
                    "size": c.size,
                    "linked_json": linked,
                }));
            }
        }
        out.sort_by(|a, b| a["path"].as_str().cmp(&b["path"].as_str()));
        Ok(serde_json::json!(out))
    })
    .await
}

/// 打开一个文件：返回类型与内容，或者拒绝的理由。
#[command]
async fn open_file(root: String, path: String) -> Result<Value, String> {
    blocking(move || {
        use sc2trans_core::filekind;
        use sc2trans_core::link;
        let full = PathBuf::from(&root).join(&path);
        let c = filekind::classify(&full);
        let mut value = serde_json::json!({ "path": path, "kind": c.kind, "size": c.size });
        if let Some(reason) = c.reason {
            value["reason"] = serde_json::json!(reason);
        }
        match c.kind {
            filekind::FileKind::Text => {
                let text = std::fs::read_to_string(&full).map_err(|e| format!("读取 {path} 失败: {e}"))?;
                value["text"] = serde_json::json!(text);
            }
            filekind::FileKind::Sc2 => {
                let linked = match link::link(&full) {
                    link::Link::Linked { path } => Some(path.file_name().map(|n| n.to_string_lossy().to_string())),
                    _ => None,
                };
                value["linked_json"] = serde_json::json!(linked);
            }
            _ => {}
        }
        Ok(value)
    })
    .await
}

/// 选一张背景图片。
#[command]
async fn pick_image() -> Option<String> {
    tauri::async_runtime::spawn_blocking(move || {
        rfd::FileDialog::new()
            .set_title("选择背景图像")
            .add_filter("图片", &["png", "jpg", "jpeg", "webp", "gif", "bmp"])
            .pick_file()
            .map(|p| p.display().to_string())
    })
    .await
    .ok()
    .flatten()
}

/// 保证 SC2Diff 就位（首次调用即静默安装）。
#[command]
async fn vcs_tool(app: tauri::AppHandle) -> Result<Value, String> {
    let dir = data_dir(&app)?;
    let bundled = app.path().resource_dir().ok();
    blocking(move || {
        let status = vcs::ensure(&dir, bundled.as_deref());
        serde_json::to_value(status).map_err(|e| format!("{e:#}"))
    })
    .await
}

#[command]
async fn vcs_reinstall(app: tauri::AppHandle) -> Result<Value, String> {
    let dir = data_dir(&app)?;
    let bundled = app.path().resource_dir().ok();
    blocking(move || {
        let _ = std::fs::remove_file(vcs::installed_path(&dir));
        let status = vcs::ensure(&dir, bundled.as_deref());
        serde_json::to_value(status).map_err(|e| format!("{e:#}"))
    })
    .await
}

fn repo_of(project_file: &str, document: &str) -> PathBuf {
    vcs::repo_dir_for(Path::new(project_file), Path::new(document))
}

#[command]
async fn vcs_repo_path(project_file: String, document: String) -> String {
    repo_of(&project_file, &document).display().to_string()
}

#[command]
async fn vcs_status(app: tauri::AppHandle, project_file: String, document: String) -> Result<Value, String> {
    let dir = data_dir(&app)?;
    let bundled = app.path().resource_dir().ok();
    blocking(move || {
        let repo = repo_of(&project_file, &document);
        let tool = vcs::ensure(&dir, bundled.as_deref());
        let st = if tool.installed {
            vcs::status(&dir, &repo).map_err(|e| format!("{e:#}"))?
        } else {
            vcs::status(&dir, &repo).unwrap_or(vcs::RepoStatus {
                is_repo: false,
                branch: String::new(),
                has_commits: false,
                changes: Vec::new(),
                staged: 0,
                dirty: 0,
                untracked: 0,
                raw: String::new(),
            })
        };
        Ok(serde_json::json!({
            "repo": repo.display().to_string(),
            "status": st,
            "tool": tool,
        }))
    })
    .await
}

#[command]
async fn vcs_init(app: tauri::AppHandle, project_file: String, document: String) -> Result<Value, String> {
    let dir = data_dir(&app)?;
    blocking(move || {
        let repo = repo_of(&project_file, &document);
        let out = vcs::init(&dir, &repo, Some(Path::new(&document))).map_err(|e| format!("{e:#}"))?;
        Ok(serde_json::json!({ "output": out }))
    })
    .await
}

#[command]
async fn vcs_commit(
    app: tauri::AppHandle,
    project_file: String,
    document: String,
    message: String,
) -> Result<Value, String> {
    let dir = data_dir(&app)?;
    blocking(move || {
        let repo = repo_of(&project_file, &document);
        let added = vcs::add_all(&dir, &repo).map_err(|e| format!("{e:#}"))?;
        let committed = vcs::commit(&dir, &repo, &message).map_err(|e| format!("{e:#}"))?;
        Ok(serde_json::json!({ "staged": added, "commit": committed }))
    })
    .await
}

#[command]
async fn vcs_log(
    app: tauri::AppHandle,
    project_file: String,
    document: String,
    limit: usize,
) -> Result<Value, String> {
    let dir = data_dir(&app)?;
    blocking(move || {
        let repo = repo_of(&project_file, &document);
        let commits = vcs::log(&dir, &repo, limit).map_err(|e| format!("{e:#}"))?;
        serde_json::to_value(commits).map_err(|e| format!("{e:#}"))
    })
    .await
}

#[command]
async fn vcs_diff(
    app: tauri::AppHandle,
    project_file: String,
    document: String,
    from: Option<String>,
    to: Option<String>,
) -> Result<Value, String> {
    let dir = data_dir(&app)?;
    blocking(move || {
        let repo = repo_of(&project_file, &document);
        let text = vcs::diff(&dir, &repo, from.as_deref(), to.as_deref())
            .map_err(|e| format!("{e:#}"))?;
        Ok(serde_json::json!({ "text": text }))
    })
    .await
}

#[command]
async fn vcs_refs(app: tauri::AppHandle, project_file: String, document: String) -> Result<Value, String> {
    let dir = data_dir(&app)?;
    blocking(move || {
        let repo = repo_of(&project_file, &document);
        let branches = vcs::branches(&dir, &repo).unwrap_or_default();
        let tags = vcs::tags(&dir, &repo).unwrap_or_default();
        Ok(serde_json::json!({ "branches": branches, "tags": tags }))
    })
    .await
}

#[command]
async fn vcs_ref_action(
    app: tauri::AppHandle,
    project_file: String,
    document: String,
    action: String,
    value: String,
) -> Result<Value, String> {
    let dir = data_dir(&app)?;
    blocking(move || {
        let repo = repo_of(&project_file, &document);
        let args: Vec<String> = match action.as_str() {
            "branch" => vec!["branch".into(), value.clone()],
            "tag" => vec!["tag".into(), value.clone()],
            "checkout" => vec!["checkout".into(), value.clone()],
            "switch" => vec!["switch".into(), value.clone()],
            other => return Err(format!("未知操作: {other}")),
        };
        let refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        let out = vcs::run_checked(&dir, Some(&repo), &refs).map_err(|e| format!("{e:#}"))?;
        Ok(serde_json::json!({ "output": out.combined(), "command": out.command }))
    })
    .await
}

#[command]
async fn vcs_pack(
    app: tauri::AppHandle,
    project_file: String,
    document: String,
    out: String,
) -> Result<Value, String> {
    let dir = data_dir(&app)?;
    blocking(move || {
        let repo = repo_of(&project_file, &document);
        let report = vcs::pack(&dir, &repo, Path::new(&out)).map_err(|e| format!("{e:#}"))?;
        Ok(serde_json::json!({ "output": report, "path": out }))
    })
    .await
}

#[command]
async fn vcs_unpack(
    app: tauri::AppHandle,
    project_file: String,
    document: String,
) -> Result<Value, String> {
    let dir = data_dir(&app)?;
    blocking(move || {
        let repo = repo_of(&project_file, &document);
        let out = vcs::unpack(&dir, &repo, Path::new(&document)).map_err(|e| format!("{e:#}"))?;
        Ok(serde_json::json!({ "output": out }))
    })
    .await
}

#[command]
async fn vcs_pick_pack(default_name: String) -> Option<String> {
    tauri::async_runtime::spawn_blocking(move || {
        rfd::FileDialog::new()
            .set_title("从版本仓库导出文档")
            .set_file_name(default_name)
            .add_filter("星际争霸 II 文档", &["SC2Map", "SC2Mod"])
            .save_file()
            .map(|p| p.display().to_string())
    })
    .await
    .ok()
    .flatten()
}

#[command]
async fn vcs_reveal(project_file: String, document: String) -> Result<(), String> {
    blocking(move || {
        let repo = repo_of(&project_file, &document);
        let target = if repo.is_dir() { repo } else { PathBuf::from(&project_file) };
        std::process::Command::new("explorer")
            .arg(target)
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("打开资源管理器失败: {e}"))
    })
    .await
}
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            app_info,
            recent_projects,
            forget_project,
            pick_project_save,
            pick_project_open,
            pick_archives,
            save_project,
            load_project,
            pick_archive,
            pick_json_save,
            pick_json_open,
            pick_archive_save,
            inspect,
            audit,
            load_keys,
            export_document,
            import_document,
            apply_document,
            apply,
            seed,
            fill_locales,
            verify,
            reveal,
            default_output_name,
            default_json_name,
            glossary_load,
            glossary_save,
            glossary_pick,
            glossary_read,
            glossary_write,
            mt_config_get,
            mt_config_set,
            mt_translate,
            mt_translate_many,
            mt_clear_cache,
            vcs_tool,
            vcs_reinstall,
            vcs_repo_path,
            vcs_status,
            vcs_init,
            vcs_commit,
            vcs_log,
            vcs_diff,
            vcs_refs,
            vcs_ref_action,
            vcs_pack,
            vcs_pick_pack,
            vcs_unpack,
            vcs_reveal,
            pick_image,
            scan_folder,
            open_file,
        ])
        .run(tauri::generate_context!())
        .expect("启动 Tauri 应用失败");
}
