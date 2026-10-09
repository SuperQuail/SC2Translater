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

/// 导出翻译键到 JSON。
#[command]
async fn extract(
    archive: String,
    source: String,
    target: String,
    out: String,
    all_locales: bool,
) -> Result<Value, String> {
    blocking(move || {
        let doc = ops::extract(Path::new(&archive), &source, &target, None, all_locales)
            .map_err(|e| format!("{e:#}"))?;
        let value = serde_json::to_value(&doc).map_err(|e| format!("{e:#}"))?;
        let text = serde_json::to_string_pretty(&value).map_err(|e| format!("{e:#}"))?;
        std::fs::write(&out, format!("{text}\n")).map_err(|e| format!("写出 {out} 失败: {e}"))?;
        Ok(value)
    })
    .await
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
            extract,
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
        ])
        .run(tauri::generate_context!())
        .expect("启动 Tauri 应用失败");
}
