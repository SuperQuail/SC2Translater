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
use tauri::command;

use sc2trans_core::ops::{self, ApplyOptions};

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
            pick_archive,
            pick_json_save,
            pick_json_open,
            pick_archive_save,
            inspect,
            audit,
            extract,
            apply,
            seed,
            verify,
            reveal,
            default_output_name,
            default_json_name,
        ])
        .run(tauri::generate_context!())
        .expect("启动 Tauri 应用失败");
}
